//! Output formatting and display utilities.

use std::collections::HashMap;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, ContentArrangement, Table};

use crate::config::Registry;
use crate::ports::ListeningPort;

/// Status of an allocated port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortStatus {
    /// Port is allocated but not currently listening.
    Idle,
    /// Port is allocated and currently listening.
    Active,
}

/// Information about an allocated port for display.
#[derive(Debug)]
pub struct AllocatedPortInfo {
    pub project: String,
    pub name: String,
    pub port: u16,
    pub status: PortStatus,
    pub pid: Option<i32>,
    pub process_name: Option<String>,
}

/// Displays the allocated ports table.
pub fn display_allocated_ports(ports: &[AllocatedPortInfo]) {
    if ports.is_empty() {
        println!("No ports allocated.");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["PROJECT", "NAME", "PORT", "STATUS", "PID", "PROCESS"]);

    for port in ports {
        let status_cell = match port.status {
            PortStatus::Active => Cell::new("ACTIVE").fg(Color::Green),
            PortStatus::Idle => Cell::new("IDLE").fg(Color::DarkGrey),
        };

        let pid_str = port
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "---".to_string());

        let process_str = port
            .process_name
            .clone()
            .unwrap_or_else(|| "---".to_string());

        table.add_row(vec![
            Cell::new(&port.project),
            Cell::new(&port.name),
            Cell::new(port.port),
            status_cell,
            Cell::new(&pid_str),
            Cell::new(&process_str),
        ]);
    }

    println!("{table}");
}

/// Displays the status table (all listening ports).
pub fn display_status(listening: &[ListeningPort], registry: &Registry) {
    if listening.is_empty() {
        println!("No listening ports detected.");
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["PORT", "PROJECT", "NAME", "PID", "PROCESS"]);

    for lp in listening {
        let (project, name) = registry
            .find_port_owner(lp.port)
            .map(|(p, n)| (p.to_string(), n.to_string()))
            .unwrap_or_else(|| ("---".to_string(), "---".to_string()));

        let pid_str = lp
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "---".to_string());

        let process_str = lp
            .process_name
            .clone()
            .unwrap_or_else(|| "---".to_string());

        table.add_row(vec![
            Cell::new(lp.port),
            Cell::new(&project),
            Cell::new(&name),
            Cell::new(&pid_str),
            Cell::new(&process_str),
        ]);
    }

    println!("{table}");
}

/// Displays suggested ports.
pub fn display_suggestions(ports: &[u16], port_type: &str) {
    if ports.is_empty() {
        println!("No available ports in the '{port_type}' range.");
        return;
    }

    if ports.len() == 1 {
        println!("{}", ports[0]);
    } else {
        for port in ports {
            println!("{port}");
        }
    }
}

/// Displays query output for scripting.
pub fn display_query(ports: &[(String, u16)], single_value: bool) {
    if single_value && ports.len() == 1 {
        // Just output the port number
        println!("{}", ports[0].1);
    } else {
        // Output key=value pairs
        let output: Vec<String> = ports.iter().map(|(k, v)| format!("{k}={v}")).collect();
        println!("{}", output.join(" "));
    }
}

/// Displays configuration information.
pub fn display_config(registry: &Registry, path: Option<&std::path::Path>) {
    if let Some(p) = path {
        println!("Config file: {}", p.display());
        println!();
    }

    println!("Default port ranges:");
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["TYPE", "RANGE"]);

    for (name, range) in &registry.defaults.ranges {
        table.add_row(vec![
            Cell::new(name),
            Cell::new(format!("{}-{}", range[0], range[1])),
        ]);
    }

    println!("{table}");
}

/// Builds the list of allocated ports with their status.
pub fn build_allocated_port_list(
    registry: &Registry,
    listening: &[ListeningPort],
    filter_active: bool,
) -> Vec<AllocatedPortInfo> {
    let listening_map: HashMap<u16, &ListeningPort> =
        listening.iter().map(|lp| (lp.port, lp)).collect();

    let mut result = Vec::new();

    for (project_name, project) in &registry.projects {
        for (port_name, &port) in &project.ports {
            let (status, pid, process_name) = if let Some(lp) = listening_map.get(&port) {
                (PortStatus::Active, lp.pid, lp.process_name.clone())
            } else {
                (PortStatus::Idle, None, None)
            };

            if filter_active && status != PortStatus::Active {
                continue;
            }

            result.push(AllocatedPortInfo {
                project: project_name.clone(),
                name: port_name.clone(),
                port,
                status,
                pid,
                process_name,
            });
        }
    }

    // Sort by project, then by name
    result.sort_by(|a, b| (&a.project, &a.name).cmp(&(&b.project, &b.name)));

    result
}
