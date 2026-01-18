//! Persistence layer for the port manager.
//!
//! Handles loading and saving the TOML registry file.

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use crate::error::{ConfigError, Result};
use crate::model::Registry;

/// Returns the path to the registry file.
///
/// Respects the `PM_CONFIG_PATH` environment variable if set,
/// otherwise uses the system config directory.
pub fn registry_path() -> std::result::Result<PathBuf, ConfigError> {
    if let Ok(path) = std::env::var("PM_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(config_dir.join("port-manager").join("registry.toml"))
}

/// Loads the registry from disk, creating a default one if it doesn't exist.
pub fn load_registry() -> Result<Registry> {
    let path = registry_path()?;

    if !path.exists() {
        let registry = Registry::default();
        save_registry(&registry)?;
        return Ok(registry);
    }

    let content = fs::read_to_string(&path).map_err(|source| ConfigError::ReadFailed {
        path: path.clone(),
        source,
    })?;

    let registry: Registry =
        toml::from_str(&content).map_err(|source| ConfigError::ParseFailed { path, source })?;

    Ok(registry)
}

/// Saves the registry to disk using atomic write.
///
/// Writes to a temporary file first, syncs to disk, then atomically renames
/// to the target path. This prevents data corruption if the process is
/// interrupted during the write.
pub fn save_registry(registry: &Registry) -> Result<()> {
    let path = registry_path()?;

    // Ensure the parent directory exists
    let parent = path.parent().ok_or(ConfigError::NoConfigDir)?;
    fs::create_dir_all(parent).map_err(|source| ConfigError::WriteFailed {
        path: parent.to_path_buf(),
        source,
    })?;

    let content = toml::to_string_pretty(registry).map_err(ConfigError::SerializeFailed)?;

    // Create temp file in the same directory (required for atomic rename)
    let temp_path = parent.join(".registry.toml.tmp");

    // Write to temp file
    let mut file = File::create(&temp_path).map_err(|source| ConfigError::WriteFailed {
        path: temp_path.clone(),
        source,
    })?;

    file.write_all(content.as_bytes())
        .map_err(|source| ConfigError::WriteFailed {
            path: temp_path.clone(),
            source,
        })?;

    // Sync to disk to ensure data is persisted
    file.sync_all().map_err(|source| ConfigError::WriteFailed {
        path: temp_path.clone(),
        source,
    })?;

    // Atomically rename temp file to target
    fs::rename(&temp_path, &path).map_err(|source| ConfigError::WriteFailed { path, source })?;

    Ok(())
}
