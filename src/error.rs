//! Error types for the port manager CLI.

use std::path::PathBuf;

use thiserror::Error;

use crate::port::Port;

/// Main error type for port manager operations.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Registry error: {0}")]
    Registry(#[from] RegistryError),

    #[error("Port detection error: {0}")]
    PortDetection(#[from] PortDetectionError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to configuration file operations.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to determine config directory. Set PM_CONFIG_DIR environment variable or ensure ~/.config exists")]
    NoConfigDir,

    #[error("Failed to read config file at {path}: {source}")]
    ReadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write config file at {path}: {source}")]
    WriteFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse config file at {path}: {source}")]
    ParseFailed {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("Failed to serialize config: {0}")]
    SerializeFailed(#[from] toml::ser::Error),

    #[error("Failed to acquire lock on {path}: {source}")]
    LockFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Errors related to port registry operations.
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Project '{0}' not found. Run 'pm list' to see allocated projects")]
    ProjectNotFound(String),

    #[error("Port name '{name}' not found in project '{project}'. Run 'pm query {project}' to see available ports")]
    PortNameNotFound { project: String, name: String },

    #[error("Port {port} is already allocated to {project}.{name}. Run 'pm list' to see all allocations")]
    PortAlreadyAllocated {
        port: Port,
        project: String,
        name: String,
    },

    #[error("Port name '{name}' already exists in project '{project}'")]
    PortNameExists { project: String, name: String },

    #[error("No available ports in range {start}-{end}. Try 'pm free <project>' to release ports or expand the range with 'pm config'")]
    NoAvailablePorts { start: u16, end: u16 },

    #[error("Port {port} is in use by {process_name} (PID {pid})")]
    PortInUse {
        port: Port,
        pid: i32,
        process_name: String,
    },

    #[error("Invalid range format: expected 'type=start-end' (e.g., web=8000-8999)")]
    InvalidRangeFormat,

    #[error("Invalid port number: '{0}'. Port must be between 1 and 65535")]
    InvalidPortNumber(String),

    #[error("Invalid range: start port ({start}) must be less than end port ({end})")]
    InvalidPortRange { start: u16, end: u16 },
}

/// Errors related to port detection via system calls.
#[derive(Error, Debug)]
pub enum PortDetectionError {
    #[error("Failed to enumerate processes: {0}. Try running with elevated privileges (sudo)")]
    ProcessEnumFailed(String),

    #[error("Platform not supported")]
    #[allow(dead_code)] // Used in #[cfg(not(target_os = "macos"))] branch
    PlatformNotSupported,
}

pub type Result<T> = std::result::Result<T, Error>;
