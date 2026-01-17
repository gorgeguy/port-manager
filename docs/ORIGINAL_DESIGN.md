# Port Manager - Original Design Document

## Overview

A Rust CLI tool (`pm`) for managing port allocations across projects with named ports, auto-suggestion, and active port detection.

## Design Summary

- **Storage**: TOML file at `~/.config/port-manager/registry.toml`
- **Model**: Projects have named ports (e.g., `webapp.web = 8080`)
- **Auto-suggest**: Configurable ranges per port type (web, api, db, etc.)
- **Port detection**: Native macOS syscalls (sysctl + libproc FFI), no subprocess calls

## CLI Commands

```
pm allocate <project> <name> [port]   # Allocate named port (auto-suggest if no port given)
pm free <project> [name]              # Free port(s) - all if no name given
pm list [--active] [--unassigned]     # List allocated ports with status
pm query <project> [name]             # Output port(s) for scripting
pm status                             # Show all listening ports (assigned + unassigned)
pm suggest [--type <type>] [count]    # Suggest available ports
pm config                             # Show/edit default ranges
```

## Storage Format

```toml
# ~/.config/port-manager/registry.toml

[defaults.ranges]
web = [8000, 8999]
api = [3000, 3999]
db = [5400, 5499]
cache = [6300, 6399]
default = [9000, 9999]

[projects.webapp]
web = 8080
api = 3000
db = 5432

[projects.backend]
api = 3001
worker = 9001
```

## Technical Design

### Crate Dependencies

- `clap` - CLI argument parsing with derive macros
- `toml` + `serde` - Config serialization
- `dirs` - Cross-platform config directory
- `comfy-table` - Formatted table output
- `libc` - FFI bindings for macOS syscalls (sysctl, libproc)
- `thiserror` - Error handling

### Module Structure

```
src/
├── main.rs              # Entry point, CLI setup
├── cli.rs               # Clap command definitions
├── config.rs            # TOML loading/saving, defaults
├── registry.rs          # Port allocation logic
├── display.rs           # Table formatting, output
├── error.rs             # Custom error types
└── ports/
    ├── mod.rs           # Platform-agnostic trait + types
    └── macos.rs         # macOS syscall implementation
                         # (libproc, sysctl FFI)
```

### Port Detection Strategy (macOS-native)

Uses macOS syscalls directly via FFI:

1. **Enumerate processes**: Use `proc_listallpids()` to get all PIDs
2. **Get file descriptors**: Use `proc_pidinfo()` with `PROC_PIDLISTFDS`
3. **Check socket info**: Use `proc_pidinfo()` with `PROC_PIDFDSOCKETINFO`
4. **Filter for listeners**: Check if socket is TCP in LISTEN state
5. **Get process names**: Use `proc_name()` to get process name

### Auto-Suggestion Algorithm

1. Get port type's range from config (or use `default` range)
2. Get all allocated ports from registry
3. Get all active ports from system
4. Find first port in range not in either set

## Future Enhancements

- Linux support via `/proc/net/tcp` + `/proc/{pid}/fd`
- Windows support via native APIs
- Port conflict warnings
- Project templates
