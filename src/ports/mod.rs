//! Port detection module.
//!
//! Provides platform-specific implementations for detecting listening ports
//! and mapping them to processes.

#[cfg(target_os = "macos")]
mod macos;

use crate::error::Result;

/// Information about a listening port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningPort {
    /// The port number.
    pub port: u16,
    /// The process ID that owns this port (if detectable).
    pub pid: Option<i32>,
    /// The process name (if detectable).
    pub process_name: Option<String>,
}

/// Returns all TCP ports currently listening on the system.
///
/// On macOS, uses native syscalls (sysctl + libproc) to enumerate ports.
/// Returns ports sorted by port number.
pub fn get_listening_ports() -> Result<Vec<ListeningPort>> {
    #[cfg(target_os = "macos")]
    {
        macos::get_listening_ports()
    }

    #[cfg(not(target_os = "macos"))]
    {
        Err(crate::error::PortDetectionError::PlatformNotSupported.into())
    }
}

/// Checks if a specific port is currently in use.
pub fn is_port_in_use(port: u16) -> Result<bool> {
    let ports = get_listening_ports()?;
    Ok(ports.iter().any(|p| p.port == port))
}
