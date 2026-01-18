# Contributing

## Development Setup

```bash
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit
cargo build
cargo test
```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy --all-targets --all-features` and fix warnings
- Write tests for new features
- Never use `unwrap()` or `expect()` in library code

## Adding a Language Adapter

1. Create `polykit-adapters/src/<language>.rs`
2. Implement `LanguageAdapter` trait
3. Register in `polykit-adapters/src/lib.rs`
4. Add to `Language` enum in `polykit-core/src/package.rs`
5. Add tests

## Pull Requests

- Keep PRs focused on a single feature
- Update documentation if needed
- Ensure all tests pass
