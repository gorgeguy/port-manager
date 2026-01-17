# Port Manager (`pm`)

A CLI tool for managing port allocations across projects with named ports, auto-suggestion, and active port detection.

## Installation

```bash
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

## Platform Support

Currently macOS only. Uses native syscalls (`libproc`) for port detection.

## License

MIT
