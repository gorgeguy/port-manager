//! Port detection module.
//!
//! Provides platform-specific implementations for detecting listening ports
//! and mapping them to processes.

#[cfg(target_os = "macos")]
mod macos;

use std::path::PathBuf;

use serde::Serialize;

use crate::error::Result;
use crate::port::Port;

/// Information about a listening port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListeningPort {
    /// The port number.
    pub port: Port,
    /// The process ID that owns this port (if detectable).
    pub pid: Option<i32>,
    /// The process name (if detectable).
    pub process_name: Option<String>,
    /// The process's current working directory (if detectable).
    pub process_cwd: Option<PathBuf>,
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
