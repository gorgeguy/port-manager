# Port Manager (`pm`)

[![Crates.io](https://img.shields.io/crates/v/port-manager.svg)](https://crates.io/crates/port-manager)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![CI](https://github.com/gorgeguy/port-manager/actions/workflows/ci.yml/badge.svg)](https://github.com/gorgeguy/port-manager/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/gorgeguy/port-manager/branch/main/graph/badge.svg)](https://codecov.io/gh/gorgeguy/port-manager)

A CLI tool for managing port allocations across projects with named ports, auto-suggestion, and active port detection.

## Why Port Manager?

**The Problem**: When working on multiple projects or microservices, port conflicts are a constant annoyance. You start your API server and discover port 3000 is already taken. Was it your other project? A forgotten background process? You resort to random port numbers, then forget which port each service uses.

**The Solution**: Port Manager gives you a persistent registry of named port allocations. Assign `webapp.api = 3000` once, and your team always knows where to find it. The tool auto-suggests ports from configurable ranges, detects what's actually running, and integrates seamlessly into shell scripts.

## Prerequisites

- **Rust toolchain** (1.70+): Install via [rustup](https://rustup.rs/)
- **macOS**: Currently macOS only (uses native `libproc` for port detection)

## Installation

### From crates.io

```bash
cargo install port-manager
```

### From source

```bash
git clone https://github.com/gorgeguy/port-manager.git
cd port-manager
cargo install --path .
```

## Usage

### Allocate a port

```bash
# Auto-suggest a port based on type
pm allocate webapp web
# Allocated webapp.web = 8000

# Specify a specific port
pm allocate webapp api 3000
# Allocated webapp.api = 3000
```

### List allocated ports

```bash
pm list
# ╭─────────┬──────┬──────┬────────╮
# │ PROJECT │ NAME │ PORT │ STATUS │
# ├─────────┼──────┼──────┼────────┤
# │ webapp  │ api  │ 3000 │ IDLE   │
# │ webapp  │ web  │ 8000 │ ACTIVE │
# ╰─────────┴──────┴──────┴────────╯

# Only show active ports
pm list --active
```

### Check system status

```bash
pm status
# ╭──────┬─────────┬──────┬───────┬─────────╮
# │ PORT │ PROJECT │ NAME │ PID   │ PROCESS │
# ├──────┼─────────┼──────┼───────┼─────────┤
# │ 3000 │ webapp  │ api  │ 12345 │ node    │
# │ 8000 │ webapp  │ web  │ 12346 │ python  │
# │ 9000 │ ---     │ ---  │ 12347 │ java    │
# ╰──────┴─────────┴──────┴───────┴─────────╯
```

### Query ports (for scripting)

```bash
# Get all ports for a project
pm query webapp
# web=8000 api=3000

# Get a single port
pm query webapp web
# 8000

# Use in shell scripts
PORT=$(pm query webapp web)
```

### Free ports

```bash
# Free a specific port
pm free webapp api
# Freed webapp.api (was 3000)

# Free all ports for a project
pm free webapp
```

### Suggest available ports

```bash
# Suggest a web port
pm suggest --type web
# 8001

# Suggest multiple ports
pm suggest --type api 3
# 3000
# 3001
# 3002
```

### Configuration

```bash
# Show config
pm config

# Show config file path
pm config --path

# Set a custom range
pm config --set cache=6000-6099
```

## JSON Output

All commands support `--json` for machine-readable output, useful for scripting and integrations:

```bash
# List allocations as JSON
pm list --json

# Query with JSON output
pm query webapp --json

# Config as JSON
pm config --json

# Suggestions as JSON
pm suggest --type web --json
```

## Port Ranges

Default ranges by type:

| Type    | Range       |
|---------|-------------|
| web     | 8000-8999   |
| api     | 3000-3999   |
| db      | 5400-5499   |
| cache   | 6300-6399   |
| default | 9000-9999   |

## Storage

Configuration is stored at `~/.config/port-manager/registry.toml`:

```toml
[defaults.ranges]
web = [8000, 8999]
api = [3000, 3999]
db = [5400, 5499]
cache = [6300, 6399]
default = [9000, 9999]

[projects.webapp]
web = 8080
api = 3000

[projects.backend]
api = 3001
```

Override the config location with `PM_CONFIG_DIR` environment variable.

## Platform Support

Currently macOS only. Uses native syscalls (`libproc`) for port detection.

## Documentation

- **[docs/ARCHITECTURE_REVIEW.md](docs/ARCHITECTURE_REVIEW.md)** - Comprehensive architecture analysis with module responsibilities, data flow diagrams, and historical refactoring roadmap
- **[docs/CODE_REVIEW_PROMPT.md](docs/CODE_REVIEW_PROMPT.md)** - Template prompt for generating architecture reviews

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

MIT
