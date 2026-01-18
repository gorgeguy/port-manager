//! Error types for the port manager CLI.

use std::path::PathBuf;
use thiserror::Error;

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
    #[error("Failed to determine config directory")]
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
}

/// Errors related to port registry operations.
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Project '{0}' not found")]
    ProjectNotFound(String),

    #[error("Port name '{name}' not found in project '{project}'")]
    PortNameNotFound { project: String, name: String },

    #[error("Port {0} is already allocated")]
    PortAlreadyAllocated(u16),

    #[error("Port name '{name}' already exists in project '{project}'")]
    PortNameExists { project: String, name: String },

    #[error("No available ports in range {start}-{end}")]
    NoAvailablePorts { start: u16, end: u16 },

    #[error("Port {port} is in use by {process_name} (PID {pid})")]
    PortInUse {
        port: u16,
        pid: i32,
        process_name: String,
    },
}

/// Errors related to port detection via system calls.
#[derive(Error, Debug)]
pub enum PortDetectionError {
    #[error("Failed to enumerate processes: {0}")]
    ProcessEnumFailed(String),

    #[error("Platform not supported")]
    #[allow(dead_code)] // Used in #[cfg(not(target_os = "macos"))] branch
    PlatformNotSupported,
}

pub type Result<T> = std::result::Result<T, Error>;
