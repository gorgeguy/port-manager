//! Data models for the port manager.
//!
//! Contains the registry structure and related types for port allocations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::port::Port;

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
    pub ports: BTreeMap<String, Port>,
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
    pub fn all_allocated_ports(&self) -> Vec<Port> {
        self.projects
            .values()
            .flat_map(|p| p.ports.values())
            .copied()
            .collect()
    }

    /// Finds which project and name owns a given port.
    pub fn find_port_owner(&self, port: Port) -> Option<(&str, &str)> {
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
        project1
            .ports
            .insert("web".to_string(), Port::new(8080).unwrap());
        project1
            .ports
            .insert("api".to_string(), Port::new(3000).unwrap());

        let mut project2 = Project::default();
        project2
            .ports
            .insert("web".to_string(), Port::new(8081).unwrap());

        registry.projects.insert("p1".to_string(), project1);
        registry.projects.insert("p2".to_string(), project2);

        let mut ports: Vec<u16> = registry
            .all_allocated_ports()
            .into_iter()
            .map(Port::as_u16)
            .collect();
        ports.sort();
        assert_eq!(ports, vec![3000, 8080, 8081]);
    }

    #[test]
    fn test_find_port_owner() {
        let mut registry = Registry::default();

        let mut project = Project::default();
        project
            .ports
            .insert("web".to_string(), Port::new(8080).unwrap());
        registry.projects.insert("webapp".to_string(), project);

        assert_eq!(
            registry.find_port_owner(Port::new(8080).unwrap()),
            Some(("webapp", "web"))
        );
        assert_eq!(registry.find_port_owner(Port::new(9999).unwrap()), None);
    }
}
