# Claude Code Guide for Track

This document provides context and guidelines for AI assistants working on the Track project. For complete contributing guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Project Overview

Track is a command-line task management tool written in Rust that helps developers manage todos, links, and notes directly from the terminal. It features:

- **CLI Interface**: Fast, intuitive command-line operations
- **Web UI**: Browser-based interface with real-time updates
- **Database**: SQLite for persistent storage
- **Shell Integration**: Completions for bash/zsh/fish

## Technology Stack

- **Language**: Rust (stable)
- **CLI Framework**: `clap` for argument parsing
- **Database**: `rusqlite` for SQLite operations
- **Web Framework**: `axum` for HTTP server
- **Frontend**: HTMX + Vanilla JavaScript (no frameworks)
- **Real-time**: Server-Sent Events (SSE) for live updates

## Project Structure

```
track/
├── src/
│   ├── main.rs           # CLI entry point
│   ├── commands/         # CLI command handlers
│   ├── db/              # Database operations
│   ├── models/          # Data structures
│   ├── webui/           # Web server implementation
│   └── lib.rs           # Library exports
├── templates/           # HTML templates
│   ├── base.html        # Base template with CSS
│   ├── index.html       # Main page
│   └── partials/        # HTMX partials
├── static/              # Static assets
├── tests/               # Integration tests
└── CONTRIBUTING.md      # Full contribution guide
```

## Key Development Commands

Before making any changes, always:

1. **Format code**: `cargo fmt`
2. **Run linter**: `cargo clippy -- -D warnings`
3. **Run tests**: `cargo test`
4. **Build project**: `cargo build`

## Code Quality Requirements

All changes must pass:
- ✅ `cargo fmt` (no formatting issues)
- ✅ `cargo clippy -- -D warnings` (no warnings)
- ✅ `cargo test` (all tests passing)

## Common Patterns

### Error Handling
```rust
use crate::TrackError;

fn operation() -> Result<T, TrackError> {
    let result = fallible_operation()?;
    Ok(result)
}
```

### Database Operations
```rust
// Use parameterized queries
conn.execute(
    "INSERT INTO todos (description) VALUES (?1)",
    params![description],
)?;
```

### CLI Commands
```rust
// Commands are in src/commands/
pub fn handle_command(args: &Args) -> Result<()> {
    // Implementation
}
```

## Testing Guidelines

### Unit Tests
Place tests in the same file:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange, Act, Assert
    }
}
```

### Integration Tests
Place in `tests/` directory for end-to-end workflows.

### Manual Testing
```bash
# Test CLI commands
cargo run -- todo add "Test task"
cargo run -- todo list

# Test WebUI
cargo run -- webui
# Open http://localhost:3000
```

## WebUI Development

When working on the WebUI:

1. **Backend routes**: Edit `src/webui/routes.rs`
2. **Templates**: Edit files in `templates/`
3. **Styling**: Modify CSS in `templates/base.html`
4. **Real-time updates**: SSE state in `src/webui/state.rs`

Restart the server after changes:
```bash
cargo run -- webui
```

## Coding Standards

Follow Rust conventions:
- `snake_case` for functions, variables, modules
- `PascalCase` for types and traits
- `SCREAMING_SNAKE_CASE` for constants

See [CONTRIBUTING.md](CONTRIBUTING.md#coding-standards) for complete style guide.

## Commit Message Format

```
<type>: <short summary>

<detailed description>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Example:
```
feat: add priority field to todos

Added priority field (1-5) to todo items with database migration.
Updated CLI commands and WebUI to support priority filtering.
```

## Before Submitting Changes

Checklist:
- [ ] Read relevant code before modifying
- [ ] Run `cargo fmt`
- [ ] Run `cargo clippy -- -D warnings` (must pass)
- [ ] Run `cargo test` (all tests pass)
- [ ] Add tests for new features
- [ ] Update documentation if needed
- [ ] Follow commit message format
- [ ] Verify manual testing for UI changes

## Important Notes

- **Always read files before editing**: Never propose changes to code you haven't read
- **Follow existing patterns**: Check similar code in the codebase for consistency
- **Keep it simple**: Avoid over-engineering, only make necessary changes
- **Test thoroughly**: Both automated tests and manual verification
- **Security**: Prevent SQL injection, XSS, and other vulnerabilities

## Getting Help

- Review [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines
- Check existing code for patterns and examples
- Look at recent commits for style reference
- Test changes locally before submitting

## Quick Reference

```bash
# Build and test
cargo build
cargo test
cargo fmt
cargo clippy -- -D warnings

# Run CLI
cargo run -- <command>

# Run WebUI
cargo run -- webui

# Run specific test
cargo test test_name

# View test output
cargo test -- --nocapture
```

---

For complete details, see [CONTRIBUTING.md](CONTRIBUTING.md).
