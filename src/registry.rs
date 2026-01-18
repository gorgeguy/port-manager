//! Port allocation and management logic.

use std::collections::HashSet;

use crate::config::Registry;
use crate::error::{RegistryError, Result};
use crate::ports::ListeningPort;

/// Allocates a port to a project with a given name.
///
/// If `port` is `None`, automatically suggests a port based on the port type.
pub fn allocate_port(
    registry: &mut Registry,
    project: &str,
    name: &str,
    port: Option<u16>,
    active_ports: &[ListeningPort],
) -> Result<u16> {
    // Check if port name already exists in project
    if let Some(proj) = registry.projects.get(project) {
        if proj.ports.contains_key(name) {
            return Err(RegistryError::PortNameExists {
                project: project.to_string(),
                name: name.to_string(),
            }
            .into());
        }
    }

    let allocated_port = match port {
        Some(p) => {
            // Verify port is not already allocated
            if registry.find_port_owner(p).is_some() {
                return Err(RegistryError::PortAlreadyAllocated(p).into());
            }
            // Verify port is not currently in use
            if let Some(active) = active_ports.iter().find(|ap| ap.port == p) {
                return Err(RegistryError::PortInUse {
                    port: p,
                    pid: active.pid.unwrap_or(0),
                    process_name: active.process_name.clone().unwrap_or_else(|| "unknown".to_string()),
                }
                .into());
            }
            p
        }
        None => {
            // Auto-suggest based on port type (name)
            suggest_port(registry, name, 1, active_ports)?
                .first()
                .copied()
                .ok_or_else(|| {
                    let range = registry.get_range(name);
                    RegistryError::NoAvailablePorts {
                        start: range[0],
                        end: range[1],
                    }
                })?
        }
    };

    // Get or create the project
    let proj = registry
        .projects
        .entry(project.to_string())
        .or_default();

    proj.ports.insert(name.to_string(), allocated_port);

    Ok(allocated_port)
}

/// Frees a port from a project.
///
/// If `name` is `None`, frees all ports from the project.
/// Returns the freed ports as (name, port) pairs.
pub fn free_port(
    registry: &mut Registry,
    project: &str,
    name: Option<&str>,
) -> Result<Vec<(String, u16)>> {
    let proj = registry
        .projects
        .get_mut(project)
        .ok_or_else(|| RegistryError::ProjectNotFound(project.to_string()))?;

    let freed = match name {
        Some(n) => {
            let port = proj.ports.remove(n).ok_or_else(|| RegistryError::PortNameNotFound {
                project: project.to_string(),
                name: n.to_string(),
            })?;
            vec![(n.to_string(), port)]
        }
        None => {
            let all_ports: Vec<_> = std::mem::take(&mut proj.ports).into_iter().collect();
            all_ports
        }
    };

    // Remove project if empty
    if proj.ports.is_empty() {
        registry.projects.remove(project);
    }

    Ok(freed)
}

/// Suggests available ports in the given type's range.
///
/// Returns up to `count` ports that are:
/// - Within the range for the given port type
/// - Not already allocated in the registry
/// - Not currently in use on the system
pub fn suggest_port(
    registry: &Registry,
    port_type: &str,
    count: usize,
    active_ports: &[ListeningPort],
) -> Result<Vec<u16>> {
    let range = registry.get_range(port_type);

    // Collect all ports to exclude
    let allocated: HashSet<u16> = registry.all_allocated_ports().into_iter().collect();
    let active: HashSet<u16> = active_ports.iter().map(|p| p.port).collect();

    let mut suggestions = Vec::new();
    for port in range[0]..=range[1] {
        if !allocated.contains(&port) && !active.contains(&port) {
            suggestions.push(port);
            if suggestions.len() >= count {
                break;
            }
        }
    }

    if suggestions.is_empty() {
        return Err(RegistryError::NoAvailablePorts {
            start: range[0],
            end: range[1],
        }
        .into());
    }

    Ok(suggestions)
}

/// Queries ports for a project.
///
/// If `name` is `None`, returns all ports for the project.
/// Returns (name, port) pairs.
pub fn query_ports(
    registry: &Registry,
    project: &str,
    name: Option<&str>,
) -> Result<Vec<(String, u16)>> {
    let proj = registry
        .projects
        .get(project)
        .ok_or_else(|| RegistryError::ProjectNotFound(project.to_string()))?;

    match name {
        Some(n) => {
            let port = proj.ports.get(n).ok_or_else(|| RegistryError::PortNameNotFound {
                project: project.to_string(),
                name: n.to_string(),
            })?;
            Ok(vec![(n.to_string(), *port)])
        }
        None => Ok(proj.ports.iter().map(|(k, v)| (k.clone(), *v)).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_registry() -> Registry {
        Registry::default()
    }

    #[test]
    fn test_allocate_explicit_port() {
        let mut registry = empty_registry();
        let active = vec![];

        let port = allocate_port(&mut registry, "webapp", "web", Some(8080), &active).unwrap();
        assert_eq!(port, 8080);
        assert_eq!(registry.projects["webapp"].ports["web"], 8080);
    }

    #[test]
    fn test_allocate_auto_suggest() {
        let mut registry = empty_registry();
        let active = vec![];

        let port = allocate_port(&mut registry, "webapp", "web", None, &active).unwrap();
        assert_eq!(port, 8000); // First port in web range
    }

    #[test]
    fn test_allocate_avoids_active() {
        let mut registry = empty_registry();
        let active = vec![
            ListeningPort {
                port: 8000,
                pid: Some(123),
                process_name: Some("python".to_string()),
            },
            ListeningPort {
                port: 8001,
                pid: Some(124),
                process_name: Some("node".to_string()),
            },
        ];

        let port = allocate_port(&mut registry, "webapp", "web", None, &active).unwrap();
        assert_eq!(port, 8002); // Skips 8000 and 8001
    }

    #[test]
    fn test_allocate_duplicate_port() {
        let mut registry = empty_registry();
        let active = vec![];

        allocate_port(&mut registry, "webapp", "web", Some(8080), &active).unwrap();
        let result = allocate_port(&mut registry, "backend", "api", Some(8080), &active);

        assert!(matches!(
            result,
            Err(crate::error::Error::Registry(RegistryError::PortAlreadyAllocated(8080)))
        ));
    }

    #[test]
    fn test_allocate_explicit_port_in_use() {
        let mut registry = empty_registry();
        let active = vec![ListeningPort {
            port: 8080,
            pid: Some(999),
            process_name: Some("python".to_string()),
        }];

        let result = allocate_port(&mut registry, "webapp", "web", Some(8080), &active);

        assert!(matches!(
            result,
            Err(crate::error::Error::Registry(RegistryError::PortInUse {
                port: 8080,
                pid: 999,
                process_name: _,
            }))
        ));
    }

    #[test]
    fn test_free_single_port() {
        let mut registry = empty_registry();
        let active = vec![];

        allocate_port(&mut registry, "webapp", "web", Some(8080), &active).unwrap();
        allocate_port(&mut registry, "webapp", "api", Some(3000), &active).unwrap();

        let freed = free_port(&mut registry, "webapp", Some("web")).unwrap();
        assert_eq!(freed, vec![("web".to_string(), 8080)]);
        assert!(!registry.projects["webapp"].ports.contains_key("web"));
        assert!(registry.projects["webapp"].ports.contains_key("api"));
    }

    #[test]
    fn test_free_all_ports() {
        let mut registry = empty_registry();
        let active = vec![];

        allocate_port(&mut registry, "webapp", "web", Some(8080), &active).unwrap();
        allocate_port(&mut registry, "webapp", "api", Some(3000), &active).unwrap();

        let freed = free_port(&mut registry, "webapp", None).unwrap();
        assert_eq!(freed.len(), 2);
        assert!(!registry.projects.contains_key("webapp"));
    }

    #[test]
    fn test_query_all_ports() {
        let mut registry = empty_registry();
        let active = vec![];

        allocate_port(&mut registry, "webapp", "web", Some(8080), &active).unwrap();
        allocate_port(&mut registry, "webapp", "api", Some(3000), &active).unwrap();

        let ports = query_ports(&registry, "webapp", None).unwrap();
        assert_eq!(ports.len(), 2);
    }

    #[test]
    fn test_query_single_port() {
        let mut registry = empty_registry();
        let active = vec![];

        allocate_port(&mut registry, "webapp", "web", Some(8080), &active).unwrap();

        let ports = query_ports(&registry, "webapp", Some("web")).unwrap();
        assert_eq!(ports, vec![("web".to_string(), 8080)]);
    }

    #[test]
    fn test_suggest_ports() {
        let mut registry = empty_registry();
        let active = vec![];

        // Allocate first few ports
        allocate_port(&mut registry, "p1", "web", Some(8000), &active).unwrap();
        allocate_port(&mut registry, "p2", "web", Some(8001), &active).unwrap();

        let suggestions = suggest_port(&registry, "web", 3, &active).unwrap();
        assert_eq!(suggestions, vec![8002, 8003, 8004]);
    }
}
