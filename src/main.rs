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
    build_allocated_port_list, display_allocated_ports, display_config, display_query,
    display_status, display_suggestions,
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

        Command::List { active, unassigned } => cmd_list(active, unassigned),

        Command::Query { project, name } => cmd_query(&project, name.as_deref()),

        Command::Status => cmd_status(),

        Command::Suggest { r#type, count } => cmd_suggest(&r#type, count),

        Command::Config { path, set } => cmd_config(path, set),
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

fn cmd_list(active_only: bool, unassigned_only: bool) -> Result<()> {
    let registry = load_registry()?;
    let listening = get_listening_ports().unwrap_or_default();

    if unassigned_only {
        // Show only unassigned listening ports
        let unassigned: Vec<_> = listening
            .iter()
            .filter(|lp| registry.find_port_owner(lp.port).is_none())
            .cloned()
            .collect();
        display_status(&unassigned, &registry);
    } else {
        let ports = build_allocated_port_list(&registry, &listening, active_only);
        display_allocated_ports(&ports);
    }

    Ok(())
}

fn cmd_query(project: &str, name: Option<&str>) -> Result<()> {
    let registry = load_registry()?;

    let ports = query_ports(&registry, project, name)?;

    if ports.is_empty() {
        // No output for scripting - exit success but empty
        return Ok(());
    }

    display_query(&ports, name.is_some());
    Ok(())
}

fn cmd_status() -> Result<()> {
    let registry = load_registry()?;
    let listening = get_listening_ports()?;

    display_status(&listening, &registry);
    Ok(())
}

fn cmd_suggest(port_type: &str, count: usize) -> Result<()> {
    let registry = load_registry()?;
    let active_ports = get_listening_ports().unwrap_or_default();

    let suggestions = suggest_port(&registry, port_type, count, &active_ports)?;
    display_suggestions(&suggestions, port_type);

    Ok(())
}

fn cmd_config(show_path: bool, set_range: Option<String>) -> Result<()> {
    let path = registry_path()?;

    if let Some(range_spec) = set_range {
        let (type_name, start, end) =
            with_registry_mut(|registry| set_port_range(registry, &range_spec))?;
        println!("Set {type_name} range to {start}-{end}");
        return Ok(());
    }

    let registry = load_registry()?;
    if show_path {
        display_config(&registry, Some(&path));
    } else {
        display_config(&registry, None);
    }

    Ok(())
}
