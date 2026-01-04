# Contributing to Track

Thank you for your interest in contributing to Track! This guide will help you get started with development.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Quality](#code-quality)
- [Testing](#testing)
- [WebUI Development](#webui-development)
- [Submitting Changes](#submitting-changes)
- [Coding Standards](#coding-standards)

## Getting Started

### Prerequisites

- **Rust**: Install the latest stable version from [rustup.rs](https://rustup.rs/)
- **Git**: For version control
- **SQLite**: Required for database operations (usually pre-installed)

### Initial Setup

1. **Fork the repository** on GitHub
2. **Clone your fork**:
   ```bash
   git clone https://github.com/your-username/track.git
   cd track
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/original-owner/track.git
   ```
4. **Build the project**:
   ```bash
   cargo build
   ```
5. **Run tests** to verify setup:
   ```bash
   cargo test
   ```

## Development Workflow

### Creating a Feature Branch

```bash
# Update your main branch
git checkout master
git pull upstream master

# Create a feature branch
git checkout -b feature/my-feature
```

### Making Changes

1. **Make your changes** in the appropriate files
2. **Format your code**:
   ```bash
   cargo fmt
   ```
3. **Run clippy** to catch common mistakes:
   ```bash
   cargo clippy -- -D warnings
   ```
4. **Run tests**:
   ```bash
   cargo test
   ```
5. **Test manually** if applicable:
   ```bash
   cargo run -- <command>
   ```

### Keeping Your Branch Updated

```bash
git fetch upstream
git rebase upstream/master
```

## Code Quality

### Formatting

All code must be formatted with `rustfmt`:

```bash
cargo fmt
```

**Tip**: Configure your editor to run `cargo fmt` on save.

### Linting with Clippy

We use `clippy` to catch common mistakes and enforce best practices:

```bash
# Run clippy
cargo clippy

# Run clippy with warnings as errors (CI requirement)
cargo clippy -- -D warnings
```

**Common clippy fixes**:
- Remove unused imports and variables
- Use `&str` instead of `&String` where appropriate
- Simplify boolean expressions
- Use iterator methods instead of manual loops

### Code Style Guidelines

- **Naming**:
  - Use `snake_case` for functions, variables, and modules
  - Use `PascalCase` for types and traits
  - Use `SCREAMING_SNAKE_CASE` for constants
- **Error Handling**:
  - Use `Result<T>` for fallible operations
  - Propagate errors with `?` operator
  - Use custom error types (`TrackError`) for domain errors
- **Documentation**:
  - Add doc comments (`///`) for public APIs
  - Include examples in doc comments where helpful
  - Document non-obvious behavior
- **Imports**:
  - Group imports: std, external crates, internal modules
  - Use `use` statements to avoid repetition

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run unit tests only
cargo test --lib
```

### Writing Tests

#### Unit Tests

Place unit tests in the same file as the code being tested:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = "test";
        
        // Act
        let result = my_function(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

#### Integration Tests

Place integration tests in `tests/` directory:

```rust
// tests/my_feature_test.rs
use track::*;

#[test]
fn test_end_to_end_workflow() {
    // Test complete workflows
}
```

### Test Coverage

- **New features**: Must include tests
- **Bug fixes**: Add regression tests
- **Refactoring**: Ensure existing tests pass

## WebUI Development

### Running the WebUI

```bash
# Start the WebUI server
cargo run -- webui

# Access at http://localhost:3000
```

### WebUI Architecture

- **Backend**: Rust with Axum framework
- **Frontend**: HTMX + Vanilla JavaScript
- **Styling**: Vanilla CSS (no frameworks)
- **Real-time updates**: Server-Sent Events (SSE)

### WebUI File Structure

```
src/webui/
├── mod.rs          # Main WebUI module
├── routes.rs       # HTTP route handlers
└── state.rs        # SSE state management

templates/
├── base.html       # Base template with CSS
├── index.html      # Main page
└── partials/       # HTMX partials
    ├── todo_list.html
    ├── links.html
    └── ...

static/
├── favicon.svg     # Favicon
└── track-icon.svg  # App icon
```

### WebUI Development Tips

1. **Hot Reload**: Restart `cargo run -- webui` after template changes
2. **CSS Changes**: Modify `templates/base.html` (CSS variables at top)
3. **HTMX Debugging**: Check browser console for `htmx:*` events
4. **SSE Testing**: Monitor Network tab for EventSource connections

### WebUI Testing

```bash
# Run with logging
RUST_LOG=debug cargo run -- webui

# Test SSE updates
# 1. Open WebUI in browser
# 2. Run CLI commands in another terminal
# 3. Verify UI updates automatically
```

## Submitting Changes

### Before Submitting

**Checklist**:
- [ ] Code is formatted (`cargo fmt`)
- [ ] Clippy passes (`cargo clippy -- -D warnings`)
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new features
- [ ] Documentation updated if needed
- [ ] Commit messages are clear and descriptive

### Creating a Pull Request

1. **Push your branch**:
   ```bash
   git push origin feature/my-feature
   ```

2. **Open a Pull Request** on GitHub

3. **PR Description** should include:
   - **What**: Brief description of changes
   - **Why**: Motivation and context
   - **How**: Implementation approach
   - **Testing**: How you tested the changes
   - **Screenshots**: For UI changes

### PR Review Process

- Maintainers will review your PR
- Address feedback by pushing new commits
- Once approved, your PR will be merged

## Coding Standards

### General Principles

- **Simplicity**: Prefer simple, readable code over clever solutions
- **Consistency**: Follow existing patterns in the codebase
- **Performance**: Optimize only when necessary, measure first
- **Safety**: Leverage Rust's type system for correctness

### Database

- Use parameterized queries to prevent SQL injection
- Handle database errors appropriately
- Use transactions for multi-step operations
- Add migrations for schema changes

### Error Messages

- Be specific and actionable
- Include context (what failed, why, how to fix)
- Use `TrackError` enum for domain errors

### Git Commit Messages

Format:
```
<type>: <short summary>

<detailed description>

<optional footer>
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

Example:
```
feat: Add link management to WebUI

Implemented add/delete functionality for links in the WebUI.
Links are displayed in a card with inline editing support.

Closes #123
```

## Questions or Issues?

- **Bug Reports**: Open an issue on GitHub
- **Feature Requests**: Open an issue with detailed description
- **Questions**: Check existing issues or open a new one

Thank you for contributing to Track! 🚀
