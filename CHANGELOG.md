# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-18

### Added

- **Core Commands**
  - `pm allocate <project> <name> [port]` - Allocate ports with optional auto-suggestion
  - `pm list [--active]` - List all allocated ports with status detection
  - `pm status` - Show all active ports on the system with process info
  - `pm query <project> [name]` - Query ports for scripting use
  - `pm free <project> [name]` - Free allocated ports
  - `pm suggest --type <type> [count]` - Suggest available ports from ranges
  - `pm config [--path] [--set <range>]` - View and modify configuration

- **JSON Output**
  - `--json` flag on `list`, `query`, `config`, and `suggest` commands
  - Machine-readable output for scripting and integrations

- **Port Ranges**
  - Configurable port ranges by type (web, api, db, cache, default)
  - Auto-suggestion respects configured ranges
  - Custom ranges via `pm config --set type=start-end`

- **Active Port Detection**
  - Real-time detection of which allocated ports are in use
  - Process name and PID reporting
  - macOS support via native `libproc` syscalls

- **Storage**
  - TOML-based configuration at `~/.config/port-manager/registry.toml`
  - File locking for safe concurrent access
  - `PM_CONFIG_DIR` environment variable for custom location

- **User Experience**
  - Clean table output with solid borders
  - Actionable error messages with suggested commands
  - Consistent exit codes for scripting

[Unreleased]: https://github.com/gorgeguy/port-manager/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/gorgeguy/port-manager/releases/tag/v0.1.0
