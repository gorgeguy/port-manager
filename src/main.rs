//! Port Manager CLI - manage port allocations across projects.

mod cli;
mod display;
mod error;
mod model;
mod persistence;
mod port;
mod ports;
mod registry;

use clap::Parser;

use cli::{Cli, Command};
use display::{
    build_allocated_port_list, build_status_port_list, display_allocated_ports,
    display_allocated_ports_json, display_config, display_config_json, display_query,
    display_query_json, display_status, display_status_json, display_suggestions,
    display_suggestions_json,
};
use error::Result;
use persistence::{load_registry, registry_path, with_registry_mut};
use port::Port;
use ports::get_listening_ports;
use registry::{allocate_port, free_port, query_ports, set_port_range, suggest_port};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Allocate {
            project,
            name,
            port,
        } => cmd_allocate(&project, &name, port),

        Command::Free { project, name } => cmd_free(&project, name.as_deref()),

        Command::List {
            active,
            unassigned,
            json,
        } => cmd_list(active, unassigned, json),

        Command::Query {
            project,
            name,
            json,
        } => cmd_query(&project, name.as_deref(), json),

        Command::Status { json, full } => cmd_status(json, full),

        Command::Suggest {
            r#type,
            count,
            json,
        } => cmd_suggest(&r#type, count, json),

        Command::Config { path, set, json } => cmd_config(path, set, json),
    }
}

fn cmd_allocate(project: &str, name: &str, port: Option<Port>) -> Result<()> {
    let active_ports = get_listening_ports().unwrap_or_default();

    let allocated =
        with_registry_mut(|registry| allocate_port(registry, project, name, port, &active_ports))?;

    println!("Allocated {project}.{name} = {allocated}");
    Ok(())
}

fn cmd_free(project: &str, name: Option<&str>) -> Result<()> {
    let freed = with_registry_mut(|registry| free_port(registry, project, name))?;

    for (port_name, port) in freed {
        println!("Freed {project}.{port_name} (was {port})");
    }

    Ok(())
}

fn cmd_list(active_only: bool, unassigned_only: bool, json: bool) -> Result<()> {
    let registry = load_registry()?;
    let listening = get_listening_ports().unwrap_or_default();

    if unassigned_only {
        // Show only unassigned listening ports
        let unassigned: Vec<_> = listening
            .iter()
            .filter(|lp| registry.find_port_owner(lp.port).is_none())
            .cloned()
            .collect();
        if json {
            let ports = build_status_port_list(&unassigned, &registry, false);
            display_status_json(&ports);
        } else {
            display_status(&unassigned, &registry, false);
        }
    } else {
        let ports = build_allocated_port_list(&registry, &listening, active_only);
        if json {
            display_allocated_ports_json(&ports);
        } else {
            display_allocated_ports(&ports);
        }
    }

    Ok(())
}

fn cmd_query(project: &str, name: Option<&str>, json: bool) -> Result<()> {
    let registry = load_registry()?;

    let ports = query_ports(&registry, project, name)?;

    if ports.is_empty() {
        if json {
            println!("[]");
        }
        // No output for scripting - exit success but empty
        return Ok(());
    }

    if json {
        display_query_json(&ports);
    } else {
        display_query(&ports, name.is_some());
    }
    Ok(())
}

fn cmd_status(json: bool, full: bool) -> Result<()> {
    let registry = load_registry()?;
    let listening = get_listening_ports()?;

    if json {
        let ports = build_status_port_list(&listening, &registry, full);
        display_status_json(&ports);
    } else {
        display_status(&listening, &registry, full);
    }
    Ok(())
}

fn cmd_suggest(port_type: &str, count: usize, json: bool) -> Result<()> {
    let registry = load_registry()?;
    let active_ports = get_listening_ports().unwrap_or_default();

    let suggestions = suggest_port(&registry, port_type, count, &active_ports)?;

    if json {
        display_suggestions_json(&suggestions);
    } else {
        display_suggestions(&suggestions, port_type);
    }

    Ok(())
}

fn cmd_config(show_path: bool, set_range: Option<String>, json: bool) -> Result<()> {
    let path = registry_path()?;

    if let Some(range_spec) = set_range {
        let (type_name, start, end) =
            with_registry_mut(|registry| set_port_range(registry, &range_spec))?;
        println!("Set {type_name} range to {start}-{end}");
        return Ok(());
    }

    let registry = load_registry()?;
    if json {
        if show_path {
            display_config_json(&registry, Some(&path));
        } else {
            display_config_json(&registry, None);
        }
    } else if show_path {
        display_config(&registry, Some(&path));
    } else {
        display_config(&registry, None);
    }

    Ok(())
}
