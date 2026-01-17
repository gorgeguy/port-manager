//! macOS-specific port detection.
//!
//! Uses sysctl to enumerate TCP connections (reliable, no permission issues)
//! and libproc to map ports to processes.

use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::ptr;

use libc::{c_int, c_void, size_t};

use crate::error::{PortDetectionError, Result};
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
    let listening_ports = get_listening_ports_sysctl()?;

    if listening_ports.is_empty() {
        // Fallback to lsof if sysctl fails
        return get_listening_ports_lsof();
    }

    // Try to get PID info via libproc for each port
    let port_to_pid = build_port_to_pid_map(&listening_ports);

    // Combine port list with PID info
    let mut result: Vec<ListeningPort> = listening_ports
        .into_iter()
        .map(|port| {
            let (pid, name) = port_to_pid
                .get(&port)
                .cloned()
                .unwrap_or((None, None));
            ListeningPort {
                port,
                pid,
                process_name: name,
            }
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
    let ret = unsafe {
        sysctl(
            mib.as_ptr(),
            4,
            ptr::null_mut(),
            &mut len,
            ptr::null(),
            0,
        )
    };
    if ret < 0 || len == 0 {
        let errno = std::io::Error::last_os_error();
        return Err(
            PortDetectionError::ProcessEnumFailed(format!(
                "sysctl size query failed: ret={}, len={}, errno={}",
                ret, len, errno
            ))
            .into(),
        );
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
    const INP_FPORT_OFFSET: usize = 20;

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

/// Builds a map from port number to (PID, process name) using lsof.
/// This is a fallback for getting PID info since libproc socket info is restricted.
fn build_port_to_pid_map(ports: &[u16]) -> HashMap<u16, (Option<i32>, Option<String>)> {
    let mut map = HashMap::new();

    // Use lsof to get PID info for the listening ports
    let output = match Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-P", "-n", "-F", "pcn"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return map,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut current_pid: Option<i32> = None;
    let mut current_name: Option<String> = None;

    for line in stdout.lines() {
        if line.starts_with('p') {
            current_pid = line[1..].parse().ok();
        } else if line.starts_with('c') {
            current_name = Some(line[1..].to_string());
        } else if line.starts_with('n') {
            if let Some(port_str) = line.rsplit(':').next() {
                if let Ok(port) = port_str.parse::<u16>() {
                    if ports.contains(&port) {
                        map.entry(port).or_insert((current_pid, current_name.clone()));
                    }
                }
            }
        }
    }

    map
}

/// Gets listening ports using lsof (fallback).
fn get_listening_ports_lsof() -> Result<Vec<ListeningPort>> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-P", "-n", "-F", "pcn"])
        .output()
        .map_err(|e| PortDetectionError::ProcessEnumFailed(format!("lsof failed: {}", e)))?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports: HashMap<u16, ListeningPort> = HashMap::new();
    let mut current_pid: Option<i32> = None;
    let mut current_name: Option<String> = None;

    for line in stdout.lines() {
        if line.starts_with('p') {
            current_pid = line[1..].parse().ok();
        } else if line.starts_with('c') {
            current_name = Some(line[1..].to_string());
        } else if line.starts_with('n') {
            if let Some(port_str) = line.rsplit(':').next() {
                if let Ok(port) = port_str.parse::<u16>() {
                    ports.entry(port).or_insert_with(|| ListeningPort {
                        port,
                        pid: current_pid,
                        process_name: current_name.clone(),
                    });
                }
            }
        }
    }

    let mut result: Vec<_> = ports.into_values().collect();
    result.sort_by_key(|p| p.port);
    Ok(result)
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
