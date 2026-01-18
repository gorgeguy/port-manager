//! Persistence layer for the port manager.
//!
//! Handles loading and saving the TOML registry file with file locking
//! for safe concurrent access.

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use fs2::FileExt;

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

/// Returns the path to the lock file used for concurrent access protection.
fn lock_file_path() -> std::result::Result<PathBuf, ConfigError> {
    let registry = registry_path()?;
    let parent = registry.parent().ok_or(ConfigError::NoConfigDir)?;
    Ok(parent.join(".registry.lock"))
}

/// Creates and opens the lock file, creating parent directories if needed.
fn open_lock_file() -> std::result::Result<File, ConfigError> {
    let lock_path = lock_file_path()?;

    // Ensure the parent directory exists
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::WriteFailed {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    File::create(&lock_path).map_err(|source| ConfigError::WriteFailed {
        path: lock_path,
        source,
    })
}

/// Loads the registry from disk, creating a default one if it doesn't exist.
///
/// Acquires an exclusive lock since loading may need to create the default
/// registry file. This ensures safe concurrent access.
pub fn load_registry() -> Result<Registry> {
    let path = registry_path()?;

    // Acquire exclusive lock (we may need to write if file doesn't exist)
    let lock_file = open_lock_file()?;
    let lock_path = lock_file_path()?;
    lock_file
        .lock_exclusive()
        .map_err(|source| ConfigError::LockFailed {
            path: lock_path,
            source,
        })?;

    // Lock is held until lock_file is dropped at end of function
    if !path.exists() {
        let registry = Registry::default();
        save_registry_inner(&registry)?;
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
/// Acquires an exclusive lock to prevent concurrent access, then writes to a
/// temporary file, syncs to disk, and atomically renames to the target path.
///
/// Note: For read-modify-write operations, prefer `with_registry_mut` to ensure
/// the lock is held for the entire transaction.
#[allow(dead_code)]
pub fn save_registry(registry: &Registry) -> Result<()> {
    // Acquire exclusive lock for writing
    let lock_file = open_lock_file()?;
    let lock_path = lock_file_path()?;
    lock_file
        .lock_exclusive()
        .map_err(|source| ConfigError::LockFailed {
            path: lock_path,
            source,
        })?;

    // Lock is held until lock_file is dropped at end of function
    // Lock is automatically released when lock_file is dropped
    save_registry_inner(registry)
}

/// Executes a read-modify-write operation on the registry atomically.
///
/// This function acquires an exclusive lock, loads the registry, calls the
/// provided closure to modify it, and saves the result. The lock is held
/// for the entire duration to prevent concurrent modifications.
///
/// Use this for any operation that needs to read, modify, and write the registry.
pub fn with_registry_mut<F, T>(f: F) -> Result<T>
where
    F: FnOnce(&mut Registry) -> Result<T>,
{
    let path = registry_path()?;

    // Acquire exclusive lock for the entire read-modify-write cycle
    let lock_file = open_lock_file()?;
    let lock_path = lock_file_path()?;
    lock_file
        .lock_exclusive()
        .map_err(|source| ConfigError::LockFailed {
            path: lock_path,
            source,
        })?;

    // Load or create default registry
    let mut registry = if !path.exists() {
        let reg = Registry::default();
        save_registry_inner(&reg)?;
        reg
    } else {
        let content = fs::read_to_string(&path).map_err(|source| ConfigError::ReadFailed {
            path: path.clone(),
            source,
        })?;
        toml::from_str(&content).map_err(|source| ConfigError::ParseFailed { path, source })?
    };

    // Call the closure to modify the registry
    let result = f(&mut registry)?;

    // Save the modified registry
    save_registry_inner(&registry)?;

    // Lock is automatically released when lock_file is dropped
    Ok(result)
}

/// Inner implementation of save_registry without locking.
fn save_registry_inner(registry: &Registry) -> Result<()> {
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
