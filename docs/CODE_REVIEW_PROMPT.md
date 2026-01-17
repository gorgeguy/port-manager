# Port-Manager Architecture Review Prompt

## Context
I need a deep architectural evaluation of the **port-manager** project. This is a **CLI tool** for managing port allocations across multiple projects, built using **Rust with Clap, Serde, and native macOS syscalls**.

---

## GOALS

1. **Structure & Flow**: Understand the directory structure, data flow, and responsibilities across the major layers:
   - CLI parsing (`cli.rs`)
   - Business logic (`registry.rs`)
   - Persistence (`config.rs`)
   - Platform abstraction (`ports/`)
   - Display formatting (`display.rs`)

2. **Coupling & Cohesion**: Assess whether CLI handling, port allocation logic, system port detection, and TOML persistence are properly separated. Check for domain logic leaking across layers.

3. **Domain Logic**: Evaluate the implementation of:
   - Port allocation algorithm (finding available ports in type-specific ranges)
   - System port detection via macOS sysctl/libproc FFI
   - Registry conflict resolution (allocated vs. active ports)

4. **Lifecycle & State**: Review how port allocations transition through their states:
   - Unallocated → Allocated (via `allocate` command)
   - Allocated → Freed (via `free` command)
   - System detection of active/listening ports

5. **Standardization**: Review cross-cutting concerns for consistency:
   - Error handling (thiserror hierarchy)
   - Result/Option propagation
   - Platform abstraction patterns
   - CLI output formatting

6. **Scalability**: Identify technical debt or patterns that may limit future goals:
   - Linux/Windows platform support
   - Network namespace awareness
   - Remote port registry synchronization

---

## TASKS

### 1. Repo Scan & Component Analysis

- Walk the directory structure to establish a mental map
- Summarize major modules and their responsibilities:
  - `main.rs` - Entry point, command dispatch
  - `cli.rs` - Clap derive definitions
  - `config.rs` - TOML loading/saving, path resolution
  - `registry.rs` - Port allocation business logic
  - `display.rs` - Table formatting
  - `error.rs` - Error type hierarchy
  - `ports/` - Platform-specific port detection
- Identify areas with unclear ownership, circular dependencies, or "god objects"

### 2. Key Entity & State Lifecycle Evaluation

Focus on the lifecycle of:
- **Registry** - The central port allocation database
- **Project** - Named collection of port allocations
- **ListeningPort** - System-detected active port with process info

Trace how state transitions occur:
- Port allocation: check registry → check system → allocate → persist
- Port freeing: lookup → remove → persist
- Port suggestion: find range → filter allocated → filter active → return first

Identify inconsistencies, race conditions (TOCTOU between check and allocate), or redundant state logic.

### 3. Critical Logic & Subsystem Audit

Locate and evaluate:

**Port Allocation Algorithm** (`registry.rs`):
- Is it pure, cohesive, and testable?
- Edge cases: range exhaustion, concurrent modifications, invalid ranges

**macOS Port Detection** (`ports/macos.rs`):
- sysctl buffer parsing correctness
- libproc FFI safety (null checks, buffer sizes)
- Error handling for permission denied, missing processes

**TOML Persistence** (`config.rs`):
- File locking considerations
- Atomic write patterns
- Default handling

### 4. Interface Layer Review

Review entry points:
- `main()` function and command dispatch
- Clap argument definitions in `cli.rs`
- Output formatting in `display.rs`

Check if the CLI layer contains too much business logic. Recommend proper layering:
```
CLI (cli.rs) → Commands (main.rs) → Services (registry.rs) → Platform (ports/) → Persistence (config.rs)
```

### 5. Data Persistence & Model Review

Review how data is defined and stored:
- `Registry` struct with `Defaults` and `Projects`
- `BTreeMap` usage for deterministic TOML ordering
- Serde derive patterns

Check for:
- File I/O safety (atomic writes, error recovery)
- Schema evolution considerations (missing fields, defaults)
- Config directory creation

### 6. Cross-Cutting Concerns (Consistency Check)

Review how these are handled globally:
- **Error types**: Consistent use of thiserror, descriptive messages
- **Result propagation**: `?` operator usage, error context
- **Logging**: Currently minimal - evaluate need for tracing/log
- **Exit codes**: Proper error code returns

Check for mixing of standards or inconsistent patterns.

### 7. Resilience & Validation

Identify brittle failure paths:
- What happens if config file is corrupted?
- What if sysctl returns unexpected data?
- What if process disappears during enumeration?

Review input validation:
- CLI argument validation (port ranges, project names)
- TOML deserialization error handling
- FFI buffer size validation

Suggest where types should enforce invariants:
- Port number newtype (0-65535)
- Project name validation
- Range validation structs

### 8. Refactor & Roadmap

Propose improvements for long-term maintenance:
- Ideal module organization
- Platform abstraction improvements
- Test coverage gaps

Create a prioritized refactor roadmap addressing:
- Linux support implementation
- Potential Windows support
- Enhanced error messages with suggestions

---

## Rust-Specific Best Practices Checklist

### Memory Safety & Ownership
- [ ] No unnecessary `clone()` calls
- [ ] Proper lifetime annotations where needed
- [ ] `Cow<str>` usage for flexible string ownership
- [ ] No `unwrap()` in library code (only tests)

### Error Handling
- [ ] Custom error types with `thiserror`
- [ ] Error context via `anyhow` or manual context
- [ ] No panics in release code paths
- [ ] Proper `?` propagation (not manual matching)

### API Design
- [ ] Builder pattern for complex construction
- [ ] `impl Into<T>` for flexible parameters
- [ ] `Default` implementations where sensible
- [ ] `Display` and `Debug` for all public types

### FFI Safety
- [ ] All FFI calls wrapped in safe abstractions
- [ ] Null pointer checks before dereferencing
- [ ] Buffer size validation
- [ ] Error code checking from C APIs

### Performance
- [ ] Avoid allocation in hot paths
- [ ] Use `&str` over `String` where possible
- [ ] Consider `SmallVec` for small collections
- [ ] Profile before optimizing

### Testing
- [ ] Unit tests with `#[cfg(test)]` modules
- [ ] Integration tests in `tests/` directory
- [ ] Doc tests for public API examples
- [ ] Property-based testing for algorithms

### Documentation
- [ ] Module-level `//!` documentation
- [ ] Function docs with `# Examples` sections
- [ ] `# Errors` section for fallible functions
- [ ] `# Panics` section if any panic conditions exist

### Clippy & Formatting
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` applied
- [ ] No `#[allow(clippy::...)]` without justification

---

## DELIVERABLES

1. **Architecture Report**: Detailed analysis of current structure
2. **Dependency/Data-Flow Map**: Text or Mermaid diagram showing module interactions
3. **State Diagram**: Port allocation lifecycle visualization
4. **Prioritized Refactor Roadmap**: What to fix now vs. later
5. **Actionable Recommendations**: Specific code quality improvements

---

## Getting Started

Begin by scanning the directory structure:
```bash
tree -I target
```

Then read the core modules in this order:
1. `Cargo.toml` - Dependencies and features
2. `src/cli.rs` - Understand command interface
3. `src/registry.rs` - Core business logic
4. `src/ports/macos.rs` - Platform implementation
5. `src/error.rs` - Error handling patterns
