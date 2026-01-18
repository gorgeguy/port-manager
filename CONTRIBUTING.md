# Contributing to Port Manager

Thank you for your interest in contributing to Port Manager! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- Rust 1.70 or later (install via [rustup](https://rustup.rs/))
- macOS (required for port detection features)

### Building from Source

```bash
git clone https://github.com/gorgeguy/port-manager.git
cd port-manager
cargo build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Code Quality

Before submitting a PR, ensure your code passes all checks:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings

# Run tests
cargo test
```

## Making Changes

### Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests and lints
5. Commit with a descriptive message
6. Push to your fork
7. Open a Pull Request

### Commit Messages

Use conventional commit format:

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `test:` Test additions or changes
- `chore:` Maintenance tasks

Examples:
```
feat: add --json flag to list command
fix: handle empty project names gracefully
docs: update README with installation instructions
```

### Code Style

- Follow Rust conventions and idioms
- Use `rustfmt` for formatting
- Address all `clippy` warnings
- Add tests for new functionality
- Update documentation as needed

## Project Structure

```
src/
├── main.rs          # CLI entry point and command parsing
├── lib.rs           # Library exports
├── commands/        # Command implementations
│   ├── mod.rs
│   ├── allocate.rs
│   ├── config.rs
│   ├── free.rs
│   ├── list.rs
│   ├── query.rs
│   ├── status.rs
│   └── suggest.rs
├── config.rs        # Configuration management
├── error.rs         # Error types
├── port.rs          # Port type and utilities
├── ports.rs         # Port detection (macOS)
├── registry.rs      # Port registry operations
└── output.rs        # Output formatting (table/JSON)
```

## Testing

- Unit tests are co-located with source files
- Integration tests are in `tests/`
- Use `tempfile` for tests that need filesystem isolation
- Mock system state where possible

## Questions?

Open an issue for questions or discussion about potential changes.
