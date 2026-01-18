//! Configuration management for the port manager.
//!
//! Handles loading and saving the TOML registry file, including default port ranges.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Result};

/// The main registry configuration, stored as TOML.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Registry {
    /// Default port ranges for different port types.
    #[serde(default)]
    pub defaults: Defaults,

    /// Projects with their named port allocations.
    #[serde(default)]
    pub projects: BTreeMap<String, Project>,
}

/// Default settings including port ranges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    /// Port ranges by type name (e.g., "web" -> [8000, 8999]).
    #[serde(default = "default_ranges")]
    pub ranges: BTreeMap<String, [u16; 2]>,
}

/// A project with its named port allocations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Project {
    /// Named ports (e.g., "web" -> 8080).
    pub ports: BTreeMap<String, u16>,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            ranges: default_ranges(),
        }
    }
}


/// Returns the default port ranges for common port types.
fn default_ranges() -> BTreeMap<String, [u16; 2]> {
    let mut ranges = BTreeMap::new();
    ranges.insert("web".to_string(), [8000, 8999]);
    ranges.insert("api".to_string(), [3000, 3999]);
    ranges.insert("db".to_string(), [5400, 5499]);
    ranges.insert("cache".to_string(), [6300, 6399]);
    ranges.insert("default".to_string(), [9000, 9999]);
    ranges
}

/// Returns the path to the registry file.
pub fn registry_path() -> std::result::Result<PathBuf, ConfigError> {
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

/// Saves the registry to disk.
pub fn save_registry(registry: &Registry) -> Result<()> {
    let path = registry_path()?;

    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::WriteFailed {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let content = toml::to_string_pretty(registry).map_err(ConfigError::SerializeFailed)?;
    fs::write(&path, content).map_err(|source| ConfigError::WriteFailed { path, source })?;

    Ok(())
}

impl Registry {
    /// Gets the port range for a given type, falling back to "default".
    pub fn get_range(&self, port_type: &str) -> [u16; 2] {
        self.defaults
            .ranges
            .get(port_type)
            .copied()
            .or_else(|| self.defaults.ranges.get("default").copied())
            .unwrap_or([9000, 9999])
    }

    /// Returns all allocated ports across all projects.
    pub fn all_allocated_ports(&self) -> Vec<u16> {
        self.projects
            .values()
            .flat_map(|p| p.ports.values())
            .copied()
            .collect()
    }

    /// Finds which project and name owns a given port.
    pub fn find_port_owner(&self, port: u16) -> Option<(&str, &str)> {
        for (project_name, project) in &self.projects {
            for (port_name, &p) in &project.ports {
                if p == port {
                    return Some((project_name, port_name));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry() {
        let registry = Registry::default();
        assert!(registry.projects.is_empty());
        assert!(!registry.defaults.ranges.is_empty());
        assert_eq!(registry.get_range("web"), [8000, 8999]);
        assert_eq!(registry.get_range("unknown"), [9000, 9999]);
    }

    #[test]
    fn test_all_allocated_ports() {
        let mut registry = Registry::default();

        let mut project1 = Project::default();
        project1.ports.insert("web".to_string(), 8080);
        project1.ports.insert("api".to_string(), 3000);

        let mut project2 = Project::default();
        project2.ports.insert("web".to_string(), 8081);

        registry.projects.insert("p1".to_string(), project1);
        registry.projects.insert("p2".to_string(), project2);

        let mut ports = registry.all_allocated_ports();
        ports.sort();
        assert_eq!(ports, vec![3000, 8080, 8081]);
    }

    #[test]
    fn test_find_port_owner() {
        let mut registry = Registry::default();

        let mut project = Project::default();
        project.ports.insert("web".to_string(), 8080);
        registry.projects.insert("webapp".to_string(), project);

        assert_eq!(
            registry.find_port_owner(8080),
            Some(("webapp", "web"))
        );
        assert_eq!(registry.find_port_owner(9999), None);
    }
}
