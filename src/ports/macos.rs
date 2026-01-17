//! macOS-specific port detection.
//!
//! Uses `lsof` for reliable port detection, with native FFI available for future optimization.

use std::collections::HashMap;
use std::mem;
use std::process::Command;
use std::ptr;

use libc::{c_int, c_void};

use crate::error::{PortDetectionError, Result};
use crate::ports::ListeningPort;

// Constants from sys/sysctl.h
const CTL_NET: c_int = 4;
const PF_INET: c_int = 2;
const IPPROTO_TCP: c_int = 6;
const TCPCTL_PCBLIST: c_int = 1;

// TCP states from netinet/tcp_fsm.h
const TCPS_LISTEN: c_int = 1;

// libproc constants
const PROC_PIDLISTFDS: c_int = 1;
const PROC_PIDFDSOCKETINFO: c_int = 3;
const PROX_FDTYPE_SOCKET: u32 = 2;

// Socket info constants
const SO_TCPINFO: c_int = 0x200;

/// xinpgen header structure from netinet/tcp_var.h
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct XInpGen {
    xig_len: u32,
    xig_count: u32,
    xig_gen: u64,
    xig_sogen: u64,
}

/// Internet address structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InAddr {
    s_addr: u32,
}

/// Socket address for IPv4
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SockAddrIn {
    sin_len: u8,
    sin_family: u8,
    sin_port: u16,
    sin_addr: InAddr,
    sin_zero: [u8; 8],
}

/// Process file descriptor info from libproc
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ProcFdInfo {
    proc_fd: i32,
    proc_fdtype: u32,
}

/// Socket info structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SocketInfo {
    soi_so: u64,
    soi_pcb: u64,
    soi_type: c_int,
    soi_protocol: c_int,
    soi_family: c_int,
    soi_options: i16,
    soi_linger: i16,
    soi_state: i16,
    soi_qlen: i16,
    soi_incqlen: i16,
    soi_qlimit: i16,
    soi_timeo: i16,
    soi_error: u16,
    soi_oobmark: u32,
    soi_rcv: SockBufInfo,
    soi_snd: SockBufInfo,
    soi_kind: c_int,
    _padding: u32,
    // Union follows - we'll use raw bytes for it
    soi_proto: [u8; 524],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SockBufInfo {
    sbi_cc: u32,
    sbi_hiwat: u32,
    sbi_mbcnt: u32,
    sbi_mbmax: u32,
    sbi_lowat: u32,
    sbi_flags: i16,
    sbi_timeo: i16,
}

/// TCP socket info within the union
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InSockInfo {
    insi_fport: u16,
    insi_lport: u16,
    insi_gencnt: u64,
    insi_flags: u32,
    insi_flow: u32,
    insi_vflag: u8,
    insi_ip_ttl: u8,
    _padding: [u8; 2],
    // Addresses follow
    insi_faddr: [u8; 16],
    insi_laddr: [u8; 16],
    insi_v4: InSockInfoV4,
    insi_v6: InSockInfoV6,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InSockInfoV4 {
    in4_tos: u8,
    _padding: [u8; 3],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InSockInfoV6 {
    in6_hlim: u8,
    in6_cksum: c_int,
    in6_ifindex: u16,
    in6_hops: i16,
}

/// Socket FD info structure
#[repr(C)]
struct SocketFdInfo {
    pfi: ProcFileInfo,
    psi: SocketInfo,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: i64,
    fi_type: i32,
    fi_guardflags: u32,
}

// External libproc functions
extern "C" {
    fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;
    fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
    fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;
}

/// Gets all listening TCP ports on the system using lsof.
pub fn get_listening_ports() -> Result<Vec<ListeningPort>> {
    get_listening_ports_lsof()
}

/// Gets listening ports using lsof (reliable fallback).
fn get_listening_ports_lsof() -> Result<Vec<ListeningPort>> {
    let output = Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-P", "-n", "-F", "pcn"])
        .output()
        .map_err(|e| PortDetectionError::ProcessEnumFailed(format!("lsof failed: {}", e)))?;

    if !output.status.success() {
        return Ok(vec![]); // lsof might fail without sudo, return empty
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut ports: HashMap<u16, ListeningPort> = HashMap::new();
    let mut current_pid: Option<i32> = None;
    let mut current_name: Option<String> = None;

    for line in stdout.lines() {
        if line.starts_with('p') {
            // PID line: p12345
            current_pid = line[1..].parse().ok();
        } else if line.starts_with('c') {
            // Command name line: cnode
            current_name = Some(line[1..].to_string());
        } else if line.starts_with('n') {
            // Name line: n*:8080 or n127.0.0.1:3000
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

/// Gets listening ports using native FFI (for future optimization).
#[allow(dead_code)]
fn get_listening_ports_native() -> Result<Vec<ListeningPort>> {
    // Strategy: enumerate all processes and their sockets to find listeners
    let pid_to_ports = get_process_listening_ports()?;

    let mut ports: Vec<ListeningPort> = pid_to_ports
        .into_iter()
        .flat_map(|(pid, port_list)| {
            let process_name = get_process_name(pid);
            port_list.into_iter().map(move |port| ListeningPort {
                port,
                pid: Some(pid),
                process_name: process_name.clone(),
            })
        })
        .collect();

    // Sort by port number
    ports.sort_by_key(|p| p.port);

    // Deduplicate (same port may appear for different addresses)
    ports.dedup_by_key(|p| p.port);

    Ok(ports)
}

/// Enumerates all processes and finds which ones have listening TCP sockets.
fn get_process_listening_ports() -> Result<HashMap<i32, Vec<u16>>> {
    let pids = list_all_pids()?;
    let mut result: HashMap<i32, Vec<u16>> = HashMap::new();

    for pid in pids {
        if let Ok(ports) = get_listening_ports_for_pid(pid) {
            if !ports.is_empty() {
                result.insert(pid, ports);
            }
        }
        // Ignore errors for individual processes (permission denied, process exited, etc.)
    }

    Ok(result)
}

/// Lists all process IDs on the system.
fn list_all_pids() -> Result<Vec<i32>> {
    // First call to get the number of PIDs
    let num_pids = unsafe { proc_listallpids(ptr::null_mut(), 0) };
    if num_pids < 0 {
        return Err(PortDetectionError::ProcessEnumFailed("proc_listallpids failed".to_string()).into());
    }

    // Allocate buffer with some extra space
    let buffer_size = (num_pids as usize + 100) * mem::size_of::<i32>();
    let mut buffer: Vec<i32> = vec![0; num_pids as usize + 100];

    let actual_count =
        unsafe { proc_listallpids(buffer.as_mut_ptr() as *mut c_void, buffer_size as c_int) };

    if actual_count < 0 {
        return Err(PortDetectionError::ProcessEnumFailed("proc_listallpids failed".to_string()).into());
    }

    buffer.truncate(actual_count as usize);
    Ok(buffer)
}

/// Gets listening TCP ports for a specific process.
fn get_listening_ports_for_pid(pid: i32) -> Result<Vec<u16>> {
    let fds = get_process_fds(pid)?;
    let mut listening_ports = Vec::new();

    for fd in fds {
        if fd.proc_fdtype == PROX_FDTYPE_SOCKET {
            if let Ok(Some(port)) = get_socket_listening_port(pid, fd.proc_fd) {
                listening_ports.push(port);
            }
        }
    }

    Ok(listening_ports)
}

/// Gets file descriptors for a process.
fn get_process_fds(pid: i32) -> Result<Vec<ProcFdInfo>> {
    // First call to get buffer size
    let buffer_size = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDLISTFDS,
            0,
            ptr::null_mut(),
            0,
        )
    };

    if buffer_size <= 0 {
        return Ok(vec![]);
    }

    let num_fds = buffer_size as usize / mem::size_of::<ProcFdInfo>();
    let mut buffer: Vec<ProcFdInfo> = Vec::with_capacity(num_fds + 10);
    unsafe {
        buffer.set_len(num_fds + 10);
    }

    let actual_size = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDLISTFDS,
            0,
            buffer.as_mut_ptr() as *mut c_void,
            (buffer.len() * mem::size_of::<ProcFdInfo>()) as c_int,
        )
    };

    if actual_size <= 0 {
        return Ok(vec![]);
    }

    let actual_count = actual_size as usize / mem::size_of::<ProcFdInfo>();
    buffer.truncate(actual_count);
    Ok(buffer)
}

/// Checks if a socket is a listening TCP socket and returns its port.
fn get_socket_listening_port(pid: i32, fd: i32) -> Result<Option<u16>> {
    let mut socket_info: SocketFdInfo = unsafe { mem::zeroed() };

    let result = unsafe {
        proc_pidinfo(
            pid,
            PROC_PIDFDSOCKETINFO,
            fd as u64,
            &mut socket_info as *mut SocketFdInfo as *mut c_void,
            mem::size_of::<SocketFdInfo>() as c_int,
        )
    };

    if result <= 0 {
        return Ok(None);
    }

    // Check if it's a TCP socket
    if socket_info.psi.soi_protocol != IPPROTO_TCP {
        return Ok(None);
    }

    // A socket is listening if it has a listen queue limit > 0
    // This is the most reliable way to detect listening sockets
    if socket_info.psi.soi_qlimit <= 0 {
        return Ok(None);
    }

    // Extract the local port from the union
    let local_port = extract_local_port(&socket_info.psi);
    if local_port == 0 {
        return Ok(None);
    }

    Ok(Some(local_port))
}

/// Extracts TCP state from socket info.
fn extract_tcp_state(si: &SocketInfo) -> c_int {
    // The TCP state is at a known offset in the soi_proto union
    // For TCP sockets, it's in the tcp_info.tcpi_state field
    // Layout: insi (InSockInfo) followed by tcp-specific info
    // TCP state is typically at offset sizeof(InSockInfo) + some offset

    // Based on XNU source, for TCP sockets, the state is stored differently
    // We need to check soi_kind first
    if si.soi_kind == SO_TCPINFO {
        // The state is embedded in the union after the InSockInfo
        // At offset 64 (size of InSockInfo) there's typically tcp_info
        // with tcpi_state at offset 0
        if si.soi_proto.len() >= 68 {
            return si.soi_proto[64] as c_int;
        }
    }

    // Fallback: Check if socket options indicate listening
    // SS_ISCONNECTED = 0x0002, SS_ISCONNECTING = 0x0004
    // A socket in LISTEN state won't have these set
    if si.soi_state & 0x0002 == 0 && si.soi_state & 0x0004 == 0 && si.soi_qlen >= 0 {
        // If qlimit > 0, this is definitely a listening socket
        if si.soi_qlimit > 0 {
            return TCPS_LISTEN;
        }
    }

    -1 // Unknown state
}

/// Extracts local port from socket info.
fn extract_local_port(si: &SocketInfo) -> u16 {
    // The local port is in the InSockInfo at the start of soi_proto
    // At offset 2 (after fport which is at offset 0)
    if si.soi_proto.len() >= 4 {
        // lport is at offset 2, big-endian
        let lport = u16::from_be_bytes([si.soi_proto[2], si.soi_proto[3]]);
        return lport;
    }
    0
}

/// Gets the name of a process by PID.
fn get_process_name(pid: i32) -> Option<String> {
    let mut buffer = vec![0u8; 1024];

    let result = unsafe { proc_name(pid, buffer.as_mut_ptr() as *mut c_void, buffer.len() as u32) };

    if result <= 0 {
        return None;
    }

    // Find null terminator
    let len = buffer.iter().position(|&b| b == 0).unwrap_or(result as usize);
    String::from_utf8(buffer[..len].to_vec()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_all_pids() {
        let pids = list_all_pids().unwrap();
        // Should have at least a few processes
        assert!(pids.len() > 10);
        // PID 1 (launchd) should exist
        assert!(pids.contains(&1));
    }

    #[test]
    fn test_get_process_name() {
        // Use current process - we definitely have permission to query ourselves
        let pid = std::process::id() as i32;
        let name = get_process_name(pid);
        // Should be able to get our own name
        assert!(name.is_some());
    }

    #[test]
    fn test_get_listening_ports() {
        // This test may find ports or not depending on what's running
        let result = get_listening_ports();
        assert!(result.is_ok());
        // Just verify we don't crash - actual ports depend on system state
    }
}
