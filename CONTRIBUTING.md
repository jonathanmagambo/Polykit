# Contributing to Polykit

Thank you for your interest in contributing to Polykit! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/polykit.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Run clippy: `cargo clippy --all-targets --all-features`
7. Commit your changes: `git commit -m "Add feature"`
8. Push to your fork: `git push origin feature/your-feature-name`
9. Open a pull request

## Development Setup

```bash
# Clone the repository
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit

# Build the project
cargo build

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets --all-features

# Run the CLI
cargo run -- scan
```

## Code Style

- Follow Rust standard formatting: `cargo fmt`
- Follow clippy suggestions: `cargo clippy`
- Write tests for new features
- Keep documentation comments concise and useful
- Use meaningful variable and function names
- Validate user input before use (see `polykit-core/src/command_validator.rs` for patterns)
- Never use `unwrap()` or `expect()` in library code; use proper error handling

## Adding a New Language Adapter

1. Create a new file in `polykit-adapters/src/` (e.g., `ruby.rs`)
2. Implement the `LanguageAdapter` trait:

```rust
pub struct RubyAdapter;

impl LanguageAdapter for RubyAdapter {
    fn language(&self) -> &'static str { "ruby" }
    fn detect(&self, path: &Path) -> bool {
        path.join("Gemfile").exists()
    }
    fn read_metadata(&self, path: &Path) -> Result<LangMetadata> {
        // Read version from gemspec or version.rb
    }
    fn bump_version(&self, path: &Path, new_version: &str) -> Result<()> {
        // Update version in gemspec or version.rb
    }
}
```

3. Register in `polykit-adapters/src/lib.rs`
4. Add the language to `Language` enum in `polykit-core/src/package.rs`
5. Add tests

## Writing Tests

- Unit tests go in the same file with `#[cfg(test)]`
- Integration tests go in `tests/` directories
- Use descriptive test names
- Test both success and error cases
- Include tests for input validation (see `polykit-core/tests/command_validator_test.rs` for examples)

## Commit Messages

- Use clear, descriptive commit messages
- Reference issues when applicable: `Fix #123`
- Use present tense: "Add feature" not "Added feature"

## Pull Requests

- Keep PRs focused on a single feature or fix
- Update documentation if needed
- Ensure all tests pass
- Request review from maintainers

## Questions?

Open an issue or start a discussion. We're happy to help!
