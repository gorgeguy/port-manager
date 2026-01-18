# Port-Manager Architecture Review Report

## Executive Summary

The port-manager is a well-structured Rust CLI tool with clean separation of concerns. The codebase demonstrates solid Rust idioms, proper error handling with `thiserror`, and effective use of platform-specific code isolation. However, several areas warrant attention for production readiness and future extensibility.

**Overall Assessment: B+** - Good foundation with room for improvement in validation, FFI safety documentation, and test coverage.

---

## 1. Architecture Report: Module Analysis

### Directory Structure
```
src/
├── main.rs       # Entry point + command dispatch (181 LOC)
├── cli.rs        # Clap CLI definitions (96 LOC)
├── config.rs     # TOML persistence + Registry struct (193 LOC)
├── registry.rs   # Port allocation business logic (288 LOC)
├── display.rs    # Table formatting with comfy-table (196 LOC)
├── error.rs      # thiserror hierarchy (97 LOC)
└── ports/
    ├── mod.rs    # Platform abstraction (42 LOC)
    └── macos.rs  # sysctl + libproc FFI (280 LOC)
```

### Module Responsibilities

| Module | Responsibility | Cohesion Rating |
|--------|---------------|-----------------|
| `cli.rs` | Clap derive definitions only | **High** - Single responsibility |
| `config.rs` | TOML loading/saving + Registry struct | **Medium** - Dual responsibility (persistence + domain model) |
| `registry.rs` | Port allocation algorithms | **High** - Pure business logic |
| `display.rs` | Table formatting for CLI output | **High** - Presentation layer |
| `error.rs` | Error type hierarchy | **High** - Centralized error handling |
| `ports/` | Platform-specific port detection | **High** - Clean abstraction boundary |
| `main.rs` | Command dispatch + orchestration | **Medium** - Contains config parsing logic |

### Areas of Concern

1. **`main.rs:129-171`** - `cmd_config()` contains business logic (range parsing/validation) that should be in `registry.rs` or a dedicated module
2. **`config.rs`** conflates domain model (`Registry`, `Project`) with persistence concerns (file I/O)
3. **Unused code** detected by clippy:
   - `AllocatedPortInfo.pid` and `process_name` fields are populated but never displayed
   - `is_port_in_use()` function never called
   - Several error variants unused: `InvalidPort`, `InvalidRange`, `SysctlFailed`, etc.

---

## 2. Dependency/Data-Flow Map

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER INPUT                               │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  cli.rs                                                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ Cli::parse() → Command enum                              │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  main.rs (Command Dispatch)                                      │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ run() → match command → cmd_* handlers                   │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                │               │               │
                ▼               ▼               ▼
┌───────────────────┐ ┌─────────────────┐ ┌─────────────────────┐
│  config.rs        │ │  registry.rs    │ │  ports/             │
│  ┌─────────────┐  │ │  ┌───────────┐  │ │  ┌───────────────┐  │
│  │load_registry│  │ │  │allocate_  │  │ │  │get_listening_ │  │
│  │save_registry│  │ │  │port()     │  │ │  │ports()        │  │
│  │Registry     │  │ │  │free_port()│  │ │  │               │  │
│  │Project      │  │ │  │suggest_   │  │ │  │[macos.rs]     │  │
│  │Defaults     │  │ │  │port()     │  │ │  │  sysctl FFI   │  │
│  └─────────────┘  │ │  │query_     │  │ │  │  libproc FFI  │  │
│        │          │ │  │ports()    │  │ │  └───────────────┘  │
│        ▼          │ │  └───────────┘  │ └─────────────────────┘
│  ~/.config/       │ └─────────────────┘
│  port-manager/    │
│  registry.toml    │
└───────────────────┘
                │               │               │
                └───────────────┼───────────────┘
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│  display.rs                                                      │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ display_allocated_ports(), display_status()              │    │
│  │ display_suggestions(), display_query(), display_config() │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                          STDOUT                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. State Diagram: Port Allocation Lifecycle

```
                    ┌──────────────┐
                    │ UNALLOCATED  │
                    │   (free)     │
                    └──────┬───────┘
                           │
          ┌────────────────┼────────────────┐
          │                │                │
          ▼                ▼                ▼
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │ Check    │    │ Check    │    │ Check    │
    │ registry │    │ active   │    │ name     │
    │ conflict │    │ ports    │    │ exists   │
    └────┬─────┘    └────┬─────┘    └────┬─────┘
         │               │               │
         ▼               ▼               ▼
    ┌─────────────────────────────────────────┐
    │            VALIDATION GATE              │
    │  - Port not in registry                 │
    │  - Port not actively listening          │
    │  - Name not already in project          │
    └────────────────┬────────────────────────┘
                     │ (all pass)
                     ▼
              ┌──────────────┐
              │  ALLOCATED   │
              │  (in registry)│
              └──────┬───────┘
                     │
      ┌──────────────┼──────────────┐
      │              │              │
      ▼              ▼              ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│  IDLE    │  │  ACTIVE  │  │  FREED   │
│ (not     │◄─┤ (process │  │ (removed │
│ listening)│  │ listening)│  │  from    │
└──────────┘  └──────────┘  │ registry) │
      │              │       └──────────┘
      └──────────────┘              │
             ▲                      │
             │                      ▼
             │              ┌──────────────┐
             └──────────────│ UNALLOCATED  │
                            └──────────────┘

TOCTOU Window: ───────────────────────────────────────
Between check and allocate, port status can change.
No file locking or atomic operations prevent races.
```

---

## 4. Critical Logic Audit

### 4.1 Port Allocation Algorithm (`registry.rs:12-62`)

**Strengths:**
- Clean separation of auto-suggest vs explicit allocation
- Proper use of `HashSet` for efficient exclusion checking
- Early return on validation failures

**Issues:**
1. **TOCTOU race condition** (`registry.rs:30-51`): Between checking system ports and allocating, another process could bind to the port
2. **No validation of explicit port** against active ports - only checks registry
3. **Range exhaustion** returns error but provides no actionable suggestion

**Code Reference:**
```rust
// registry.rs:30-37 - Explicit port only checks registry, not active
Some(p) => {
    if registry.find_port_owner(p).is_some() {
        return Err(RegistryError::PortAlreadyAllocated(p).into());
    }
    p
}
```

### 4.2 macOS Port Detection (`ports/macos.rs`)

**Strengths:**
- Documented struct layouts with header offsets
- Two-phase sysctl call pattern (size query → data fetch)
- Proper network byte order handling for ports
- Early exit optimization in PID mapping

**FFI Safety Assessment:**

| Check | Status | Notes |
|-------|--------|-------|
| Null pointer checks | ⚠️ | `libproc` crate handles internally |
| Buffer size validation | ✅ | Extra 4096 bytes allocated |
| Error code checking | ✅ | `ret < 0` checks present |
| Union access safety | ⚠️ | Relies on `soi_kind` guard |

**Concern at `macos.rs:241`:**
```rust
// SAFETY: We've verified soi_kind == 2 (TCP), so pri_tcp is valid
let tcp_info = unsafe { socket.psi.soi_proto.pri_tcp };
```
The safety comment is present but the union access could benefit from a dedicated safe wrapper.

### 4.3 TOML Persistence (`config.rs:76-112`)

**Strengths:**
- Parent directory creation before write
- `toml::to_string_pretty` for human-readable output
- `BTreeMap` for deterministic key ordering

**Missing:**
- **Atomic writes** - Corruption possible if crash during write
- **File locking** - Concurrent `pm` instances can corrupt registry
- **Backup/recovery** - No mechanism to recover from corrupted files

---

## 5. Rust Best Practices Checklist

### Memory Safety & Ownership
| Check | Status | Location |
|-------|--------|----------|
| No unnecessary `clone()` | ✅ | - |
| Proper lifetimes | ✅ | Functions use `&` appropriately |
| `Cow<str>` usage | ❌ | Not needed for current scope |
| No `unwrap()` in library code | ⚠️ | `unwrap_or_else` in `main.rs:148-156` but exits |

### Error Handling
| Check | Status | Location |
|-------|--------|----------|
| Custom thiserror types | ✅ | `error.rs` |
| Error context | ⚠️ | Missing context in some `?` chains |
| No panics in release paths | ✅ | - |
| Proper `?` propagation | ✅ | - |

### API Design
| Check | Status | Notes |
|-------|--------|-------|
| Builder pattern | N/A | Not needed |
| `impl Into<T>` | ❌ | Could benefit `allocate_port()` |
| `Default` implementations | ✅ | Present for `Registry`, `Project`, `Defaults` |
| `Display` + `Debug` | ⚠️ | `ListeningPort` has `Debug` only |

### FFI Safety
| Check | Status | Location |
|-------|--------|----------|
| Safe abstractions | ⚠️ | Direct sysctl call, libproc crate |
| Null checks | ⚠️ | Implicit via libproc |
| Buffer validation | ✅ | `macos.rs:142-144` |
| Error code checking | ✅ | `macos.rs:94, 120` |

### Testing
| Check | Status | Notes |
|-------|--------|-------|
| Unit tests | ✅ | 14 tests pass |
| Integration tests | ❌ | No `tests/` directory |
| Doc tests | ❌ | No `# Examples` sections |
| Property-based testing | ❌ | No proptest/quickcheck |

### Documentation
| Check | Status | Notes |
|-------|--------|-------|
| Module-level `//!` docs | ✅ | All modules documented |
| Function docs | ⚠️ | Most documented, some missing |
| `# Errors` section | ❌ | Not present |
| `# Panics` section | N/A | No documented panics |

### Clippy & Formatting
| Check | Status | Notes |
|-------|--------|-------|
| `cargo clippy -D warnings` | ❌ | 4 errors (dead code) |
| `cargo fmt` | ✅ | Formatted |
| Justified `#[allow]` | N/A | None present |

---

## 6. Prioritized Refactor Roadmap

### P0: Critical (Address Immediately)

1. **Fix dead code warnings** (`error.rs`, `display.rs`, `ports/mod.rs`)
   - Either use the fields/variants or remove them
   - `AllocatedPortInfo.pid`/`process_name` should be displayed in table
   - Remove or feature-gate `is_port_in_use()`

2. **Add atomic file writes** (`config.rs`)
   ```rust
   // Write to temp file, then rename
   let temp_path = path.with_extension("toml.tmp");
   fs::write(&temp_path, content)?;
   fs::rename(&temp_path, &path)?;
   ```

3. **Validate explicit port against active ports** (`registry.rs:30-37`)

### P1: High Priority (Next Sprint)

4. **Extract config parsing from `main.rs`**
   - Move `cmd_config` range parsing to `registry.rs` or new `commands.rs`

5. **Add port number newtype**
   ```rust
   #[derive(Clone, Copy, PartialEq, Eq, Hash)]
   pub struct Port(u16);

   impl Port {
       pub fn new(value: u16) -> Option<Self> {
           (value > 0).then_some(Port(value))
       }
   }
   ```

6. **Add integration tests**
   - Test full CLI flows with temp config directories
   - Test error scenarios (corrupted config, permission denied)

### P2: Medium Priority (Backlog)

7. **Split `config.rs` into `model.rs` + `persistence.rs`**
   - Domain types in `model.rs`
   - File I/O in `persistence.rs`

8. **Add file locking for concurrent access**
   ```rust
   use fs2::FileExt;
   let file = File::open(&path)?;
   file.lock_exclusive()?;
   // ... read/write ...
   file.unlock()?;
   ```

9. **Enhance error messages with suggestions**
   ```rust
   #[error("Port {0} is already allocated to {project}.{name}. Try: pm suggest")]
   PortAlreadyAllocated(u16, String project, String name),
   ```

### P3: Future (Platform Expansion)

10. **Linux support** (`ports/linux.rs`)
    - Parse `/proc/net/tcp` and `/proc/net/tcp6`
    - Use `netlink` for process-to-port mapping

11. **Windows support** (`ports/windows.rs`)
    - Use `GetExtendedTcpTable()` Win32 API
    - Requires `windows-sys` crate

12. **Cross-platform abstraction trait**
    ```rust
    pub trait PortScanner: Send + Sync {
        fn get_listening_ports(&self) -> Result<Vec<ListeningPort>>;
    }
    ```

---

## 7. Actionable Recommendations

### Immediate Actions

| # | Action | File | LOC Impact |
|---|--------|------|------------|
| 1 | Display PID/process in allocated ports table | `display.rs:44` | +5 |
| 2 | Remove `is_port_in_use()` or mark `#[allow(dead_code)]` | `ports/mod.rs:39` | -5 |
| 3 | Remove unused error variants or add `#[allow(dead_code)]` with justification | `error.rs:72-94` | ~0 |
| 4 | Check active ports for explicit allocation | `registry.rs:33` | +5 |

### Documentation Actions

| # | Action | Files |
|---|--------|-------|
| 1 | Add `# Errors` section to public functions | `registry.rs`, `config.rs` |
| 2 | Add `# Examples` doc tests | `registry.rs` |
| 3 | Document FFI safety invariants | `ports/macos.rs:241` |

### Testing Actions

| # | Action | Priority |
|---|--------|----------|
| 1 | Add integration tests with temp config | High |
| 2 | Add property tests for port allocation | Medium |
| 3 | Add fuzzing for TOML parsing | Low |

---

## 8. Conclusion

The port-manager codebase demonstrates thoughtful design with clear separation between CLI, business logic, persistence, and platform-specific code. The use of Rust idioms (thiserror, serde derive, clap derive) is appropriate and consistent.

**Key Strengths:**
- Clean module boundaries
- Good error type hierarchy
- Effective platform abstraction via `#[cfg(target_os)]`
- Deterministic TOML output via `BTreeMap`

**Key Weaknesses:**
- TOCTOU race conditions in port allocation
- No atomic writes or file locking
- Dead code that should be addressed
- Missing integration tests

The refactor roadmap prioritizes safety-critical issues (atomic writes, validation) over feature expansion (Linux/Windows support), which is the correct approach for a tool managing system resources.

---

*Report generated: January 2026*
