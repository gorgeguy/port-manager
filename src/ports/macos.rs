//! macOS-specific port detection.
//!
//! Uses sysctl to enumerate TCP connections (reliable, no permission issues)
//! and libproc to map ports to processes.

use std::collections::{HashMap, HashSet};
use std::ptr;

use libc::{c_int, c_void, size_t};
use libproc::libproc::file_info::{pidfdinfo, ListFDs, ProcFDType};
use libproc::libproc::net_info::SocketFDInfo;
use libproc::libproc::proc_pid::{listpidinfo, name};
use libproc::processes::{pids_by_type, ProcFilter};

use crate::error::{PortDetectionError, Result};
use crate::port::Port;
use crate::ports::ListeningPort;

// sysctl MIB constants (verified from macOS headers)
const CTL_NET: c_int = 4;
const PF_INET: c_int = 2;
const IPPROTO_TCP: c_int = 6;
const TCPCTL_PCBLIST: c_int = 11;

// TCP states
const TCPS_LISTEN: c_int = 1;

/// xinpgen header structure from netinet/tcp_var.h
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct XInpGen {
    xig_len: u32,
    xig_count: u32,
    xig_gen: u64,
    xig_sogen: u64,
}

// External sysctl function
extern "C" {
    fn sysctl(
        name: *const c_int,
        namelen: u32,
        oldp: *mut c_void,
        oldlenp: *mut size_t,
        newp: *const c_void,
        newlen: size_t,
    ) -> c_int;
}

/// Gets all listening TCP ports on the system.
pub fn get_listening_ports() -> Result<Vec<ListeningPort>> {
    // Use sysctl to get all listening ports (reliable, no permission issues)
    let listening_ports_raw = get_listening_ports_sysctl()?;

    // Try to get PID info via libproc for each port
    let port_to_pid = build_port_to_pid_map(&listening_ports_raw);

    // Combine port list with PID info, converting raw u16 to Port
    let mut result: Vec<ListeningPort> = listening_ports_raw
        .into_iter()
        .filter_map(|port_num| {
            // Port::new only fails for port 0, which we filter out in get_listening_ports_sysctl
            let port = Port::new(port_num).ok()?;
            let (pid, proc_name) = port_to_pid.get(&port_num).cloned().unwrap_or((None, None));
            Some(ListeningPort {
                port,
                pid,
                process_name: proc_name,
            })
        })
        .collect();

    result.sort_by_key(|p| p.port);
    result.dedup_by_key(|p| p.port);
    Ok(result)
}

/// Gets listening ports using sysctl (TCPCTL_PCBLIST).
fn get_listening_ports_sysctl() -> Result<Vec<u16>> {
    let mib: [c_int; 4] = [CTL_NET, PF_INET, IPPROTO_TCP, TCPCTL_PCBLIST];

    // First call to get buffer size
    let mut len: size_t = 0;
    let ret = unsafe { sysctl(mib.as_ptr(), 4, ptr::null_mut(), &mut len, ptr::null(), 0) };
    if ret < 0 || len == 0 {
        let errno = std::io::Error::last_os_error();
        return Err(PortDetectionError::ProcessEnumFailed(format!(
            "sysctl size query failed: ret={}, len={}, errno={}",
            ret, len, errno
        ))
        .into());
    }

    // Allocate buffer with some extra space (the size can change between calls)
    let buffer_size = len + 4096;
    let mut buffer: Vec<u8> = vec![0; buffer_size];
    let mut actual_len = buffer_size;

    let ret = unsafe {
        sysctl(
            mib.as_ptr(),
            4,
            buffer.as_mut_ptr() as *mut c_void,
            &mut actual_len,
            ptr::null(),
            0,
        )
    };
    if ret < 0 {
        return Err(
            PortDetectionError::ProcessEnumFailed("sysctl data query failed".to_string()).into(),
        );
    }

    // Parse the buffer
    let mut listening_ports: HashSet<u16> = HashSet::new();

    // Offsets determined from macOS headers (verified with offsetof):
    // sizeof(xtcpcb) = 524
    // sizeof(xinpgen) = 24
    // xt_inp at offset 4 in xtcpcb
    // xt_tp at offset 212 in xtcpcb
    // t_state at offset 32 in tcpcb -> offset 244 in xtcpcb (212 + 32)
    // inp_lport at offset 18 in inpcb -> offset 22 in xtcpcb (4 + 18), network byte order
    // inp_fport at offset 16 in inpcb -> offset 20 in xtcpcb (4 + 16), network byte order
    const XTCPCB_SIZE: usize = 524;
    const T_STATE_OFFSET: usize = 244;
    const INP_LPORT_OFFSET: usize = 22;

    // First entry is xinpgen header (24 bytes)
    if actual_len < 24 {
        return Ok(vec![]);
    }

    let header: &XInpGen = unsafe { &*(buffer.as_ptr() as *const XInpGen) };
    let mut offset = header.xig_len as usize;

    // Iterate through xtcpcb entries
    while offset + XTCPCB_SIZE <= actual_len {
        let entry_len = u32::from_ne_bytes([
            buffer[offset],
            buffer[offset + 1],
            buffer[offset + 2],
            buffer[offset + 3],
        ]) as usize;

        // End marker check (xinpgen trailer has smaller size)
        if entry_len < XTCPCB_SIZE {
            break;
        }

        if offset + entry_len > actual_len {
            break;
        }

        // Read t_state at offset 244
        let state = i32::from_ne_bytes([
            buffer[offset + T_STATE_OFFSET],
            buffer[offset + T_STATE_OFFSET + 1],
            buffer[offset + T_STATE_OFFSET + 2],
            buffer[offset + T_STATE_OFFSET + 3],
        ]);

        if state == TCPS_LISTEN {
            // Read local port at offset 22 (network byte order = big-endian)
            let lport = u16::from_be_bytes([
                buffer[offset + INP_LPORT_OFFSET],
                buffer[offset + INP_LPORT_OFFSET + 1],
            ]);

            if lport > 0 {
                listening_ports.insert(lport);
            }
        }

        offset += entry_len;
    }

    Ok(listening_ports.into_iter().collect())
}

/// Builds a map from port number to (PID, process name) using libproc.
/// Iterates all processes and their file descriptors to find socket owners.
fn build_port_to_pid_map(ports: &[u16]) -> HashMap<u16, (Option<i32>, Option<String>)> {
    let mut map = HashMap::new();

    if ports.is_empty() {
        return map;
    }

    // Convert to HashSet for faster lookups
    let port_set: HashSet<u16> = ports.iter().copied().collect();

    // Get all PIDs on the system
    let pids = match pids_by_type(ProcFilter::All) {
        Ok(pids) => pids,
        Err(_) => return map,
    };

    for pid in pids {
        let pid_i32 = pid as i32;

        // List file descriptors for this process
        // First get count, then get the actual list
        let fds = match listpidinfo::<ListFDs>(pid_i32, 256) {
            Ok(fds) => fds,
            Err(_) => continue,
        };

        for fd_info in fds {
            // Check if this is a socket file descriptor
            if fd_info.proc_fdtype != ProcFDType::Socket as u32 {
                continue;
            }

            // Get socket details
            let socket = match pidfdinfo::<SocketFDInfo>(pid_i32, fd_info.proc_fd) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // Access the socket info - check if it's a TCP socket
            // soi_kind: 2 = SOCKINFO_TCP
            if socket.psi.soi_kind != 2 {
                continue;
            }

            // Access TCP-specific info via the union
            // SAFETY: We've verified soi_kind == 2 (TCP), so pri_tcp is valid
            let tcp_info = unsafe { socket.psi.soi_proto.pri_tcp };
            let local_port = u16::from_be(tcp_info.tcpsi_ini.insi_lport as u16);

            // Check if this is a port we're looking for
            if local_port > 0 && port_set.contains(&local_port) && !map.contains_key(&local_port) {
                let proc_name = name(pid_i32).ok();
                map.insert(local_port, (Some(pid_i32), proc_name));

                // Early exit if we've found all ports
                if map.len() == port_set.len() {
                    return map;
                }
            }
        }
    }

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_listening_ports_sysctl() {
        // This should work without special permissions
        let result = get_listening_ports_sysctl();
        if let Err(ref e) = result {
            eprintln!("sysctl error: {:?}", e);
        }
        assert!(result.is_ok(), "sysctl failed: {:?}", result);
        // Just verify we don't crash - actual ports depend on system state
    }

    #[test]
    fn test_get_listening_ports() {
        let result = get_listening_ports();
        assert!(result.is_ok());
    }
}
