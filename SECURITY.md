# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in Port Manager, please report it by opening a GitHub issue. For sensitive vulnerabilities, please email the maintainers directly (contact information in Cargo.toml).

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response Timeline

- Initial response: within 48 hours
- Status update: within 7 days
- Fix timeline: depends on severity

## Known Limitations

### TOCTOU Race Condition

Port Manager has a known time-of-check to time-of-use (TOCTOU) race condition in port allocation. The sequence is:

1. Check if port is available
2. Allocate port in registry
3. Application binds to port

Between steps 1-3, another process could bind to the same port. This is an inherent limitation of user-space port management and is documented behavior.

**Mitigation**: Applications should handle `EADDRINUSE` errors gracefully and re-request a port if binding fails.

### Local System Scope

Port Manager operates on the local system only:

- Registry is stored in user-accessible location (`~/.config/port-manager/`)
- No authentication or access control
- Designed for development environments, not production

### Process Detection

Port detection relies on macOS `libproc` syscalls:

- Requires appropriate permissions to enumerate processes
- Some system processes may not be visible
- Information may be slightly stale due to polling

## Security Best Practices

When using Port Manager:

1. **Don't commit the registry**: Add `~/.config/port-manager/` to global gitignore if needed
2. **Use for development only**: Not designed for production port management
3. **Handle binding failures**: Your application should handle port conflicts gracefully
