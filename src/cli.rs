//! CLI command definitions using clap.

use clap::{Parser, Subcommand};

use crate::port::Port;

/// Port Manager - manage port allocations across projects.
#[derive(Parser, Debug)]
#[command(name = "pm")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Allocate a named port to a project.
    ///
    /// If no port is specified, one will be auto-suggested based on the port type.
    #[command(visible_alias = "a")]
    Allocate {
        /// Project name (e.g., "webapp", "backend")
        project: String,

        /// Port name/type (e.g., "web", "api", "db")
        name: String,

        /// Specific port number to allocate (optional - auto-suggest if omitted)
        port: Option<Port>,
    },

    /// Free port(s) from a project.
    ///
    /// If no name is specified, frees all ports from the project.
    #[command(visible_alias = "f")]
    Free {
        /// Project name
        project: String,

        /// Port name to free (optional - frees all if omitted)
        name: Option<String>,
    },

    /// List allocated ports with their status.
    #[command(visible_alias = "l", visible_alias = "ls")]
    List {
        /// Only show active (listening) ports
        #[arg(long)]
        active: bool,

        /// Only show unassigned listening ports (for status-like output)
        #[arg(long)]
        unassigned: bool,

        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },

    /// Query port(s) for a project (for scripting).
    ///
    /// Outputs in key=value format for easy parsing.
    #[command(visible_alias = "q")]
    Query {
        /// Project name
        project: String,

        /// Port name (optional - shows all if omitted)
        name: Option<String>,

        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },

    /// Show all listening ports on the system.
    ///
    /// Displays both assigned and unassigned ports.
    #[command(visible_alias = "s")]
    Status {
        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },

    /// Suggest available ports.
    #[command(visible_alias = "sg")]
    Suggest {
        /// Port type for range selection (e.g., "web", "api", "db")
        #[arg(long, short = 't', default_value = "default")]
        r#type: String,

        /// Number of ports to suggest
        #[arg(default_value = "1")]
        count: usize,

        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },

    /// Show or edit configuration.
    #[command(visible_alias = "c")]
    Config {
        /// Show the config file path
        #[arg(long)]
        path: bool,

        /// Set a port range for a type (format: type=start-end, e.g., "web=8000-8999")
        #[arg(long)]
        set: Option<String>,

        /// Output as JSON for scripting
        #[arg(long)]
        json: bool,
    },
}
