//! Output formatting and display utilities.

use std::collections::HashMap;

use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL_CONDENSED;
use comfy_table::{Cell, Color, ContentArrangement, Table, TableComponent};
use serde::Serialize;

use crate::model::Registry;
use crate::port::Port;
use crate::ports::ListeningPort;

/// Creates a table with clean styling: solid borders, no row separators.
fn create_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL_CONDENSED)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    // Use solid vertical bars instead of dotted
    table.set_style(TableComponent::VerticalLines, '│');
    // Use single-line header separator instead of double
    table.set_style(TableComponent::MiddleHeaderIntersections, '┼');
    table.set_style(TableComponent::HeaderLines, '─');
    table.set_style(TableComponent::LeftHeaderIntersection, '├');
    table.set_style(TableComponent::RightHeaderIntersection, '┤');
    table
}

/// Status of an allocated port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PortStatus {
    /// Port is allocated but not currently listening.
    Idle,
    /// Port is allocated and currently listening.
    Active,
}

/// Information about an allocated port for display.
#[derive(Debug, Serialize)]
pub struct AllocatedPortInfo {
    pub project: String,
    pub name: String,
    pub port: Port,
    pub status: PortStatus,
    pub pid: Option<i32>,
    #[serde(rename = "process")]
    pub process_name: Option<String>,
}

/// Information about a listening port for JSON status output.
#[derive(Debug, Serialize)]
pub struct StatusPortInfo {
    pub port: Port,
    pub project: Option<String>,
    pub name: Option<String>,
    pub pid: Option<i32>,
    pub process: Option<String>,
}

/// Displays the allocated ports table.
pub fn display_allocated_ports(ports: &[AllocatedPortInfo]) {
    if ports.is_empty() {
        println!("No ports allocated.");
        return;
    }

    let mut table = create_table();
    table.set_header(vec!["PROJECT", "NAME", "PORT", "STATUS", "PID", "PROCESS"]);

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

    let mut table = create_table();
    table.set_header(vec!["PORT", "PROJECT", "NAME", "PID", "PROCESS"]);

    for lp in listening {
        let (project, name) = registry
            .find_port_owner(lp.port)
            .map(|(p, n)| (p.to_string(), n.to_string()))
            .unwrap_or_else(|| ("---".to_string(), "---".to_string()));

        let pid_str = lp
            .pid
            .map(|p| p.to_string())
            .unwrap_or_else(|| "---".to_string());

        let process_str = lp.process_name.clone().unwrap_or_else(|| "---".to_string());

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
pub fn display_suggestions(ports: &[Port], port_type: &str) {
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
pub fn display_query(ports: &[(String, Port)], single_value: bool) {
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
    let mut table = create_table();
    table.set_header(vec!["TYPE", "RANGE"]);

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
    let listening_map: HashMap<Port, &ListeningPort> =
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

/// Builds the list of listening ports with ownership info for JSON status output.
pub fn build_status_port_list(
    listening: &[ListeningPort],
    registry: &Registry,
) -> Vec<StatusPortInfo> {
    listening
        .iter()
        .map(|lp| {
            let (project, name) = registry
                .find_port_owner(lp.port)
                .map(|(p, n)| (Some(p.to_string()), Some(n.to_string())))
                .unwrap_or((None, None));

            StatusPortInfo {
                port: lp.port,
                project,
                name,
                pid: lp.pid,
                process: lp.process_name.clone(),
            }
        })
        .collect()
}

/// Displays allocated ports as JSON.
pub fn display_allocated_ports_json(ports: &[AllocatedPortInfo]) {
    let json = serde_json::to_string_pretty(ports).expect("Failed to serialize to JSON");
    println!("{json}");
}

/// Displays status (listening ports) as JSON.
pub fn display_status_json(ports: &[StatusPortInfo]) {
    let json = serde_json::to_string_pretty(ports).expect("Failed to serialize to JSON");
    println!("{json}");
}

/// Configuration info for JSON output.
#[derive(Debug, Serialize)]
pub struct ConfigInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_file: Option<String>,
    pub ranges: Vec<RangeInfo>,
}

/// Port range info for JSON output.
#[derive(Debug, Serialize)]
pub struct RangeInfo {
    pub name: String,
    pub start: u16,
    pub end: u16,
}

/// Displays configuration as JSON.
pub fn display_config_json(registry: &Registry, path: Option<&std::path::Path>) {
    let ranges: Vec<RangeInfo> = registry
        .defaults
        .ranges
        .iter()
        .map(|(name, range)| RangeInfo {
            name: name.clone(),
            start: range[0],
            end: range[1],
        })
        .collect();

    let config = ConfigInfo {
        config_file: path.map(|p| p.display().to_string()),
        ranges,
    };

    let json = serde_json::to_string_pretty(&config).expect("Failed to serialize to JSON");
    println!("{json}");
}

/// Query result for JSON output.
#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub name: String,
    pub port: Port,
}

/// Displays query results as JSON.
pub fn display_query_json(ports: &[(String, Port)]) {
    let results: Vec<QueryResult> = ports
        .iter()
        .map(|(name, port)| QueryResult {
            name: name.clone(),
            port: *port,
        })
        .collect();

    let json = serde_json::to_string_pretty(&results).expect("Failed to serialize to JSON");
    println!("{json}");
}

/// Displays suggested ports as JSON.
pub fn display_suggestions_json(ports: &[Port]) {
    let json = serde_json::to_string_pretty(ports).expect("Failed to serialize to JSON");
    println!("{json}");
}
