<div align="center">
  <h1>Polykit</h1>
  
  [<img alt="github" src="https://img.shields.io/badge/github-jonathanmagambo/polykit-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/jonathanmagambo/polykit)
  [<img alt="crates.io" src="https://img.shields.io/crates/v/polykit.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/polykit)
  [<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/jonathanmagambo/Polykit/rust.yml?branch=main&style=for-the-badge" height="20">](https://github.com/jonathanmagambo/Polykit/actions?query=branch%3Amain)
  [<img alt="discord" src="https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white" height="20">](https://discord.gg/5Y9jmtysua)
</div>

Fast, language-agnostic monorepo orchestration tool.

## Installation

Install from crates.io:

```bash
cargo install polykit
```

Or build from source:

```bash
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit
cargo install --path polykit
```

## Quick Start

1. Create a `polykit.toml` in each package:

```toml
name = "my-package"
language = "rust"
public = true

[deps]
internal = ["other-package"]

[tasks]
build = "cargo build --release"
test = "cargo test"
```

2. Run commands:

```bash
polykit scan          # Discover packages
polykit build         # Build all packages
polykit test          # Run tests
polykit graph         # Show dependency order
```

## Commands

- `polykit scan` - Discover packages
- `polykit graph` - Show dependency order
- `polykit build [packages...]` - Build packages
- `polykit test [packages...]` - Run tests
- `polykit affected --git` - Find changed packages
- `polykit release <package> --bump <major|minor|patch>` - Bump versions
- `polykit watch <task>` - Watch and rebuild
- `polykit why <package>` - Show dependencies
- `polykit validate` - Validate configuration
- `polykit list` - List all tasks

## Supported Languages

- JavaScript/TypeScript (`js`, `ts`)
- Python (`python`)
- Go (`go`)
- Rust (`rust`)

## Documentation

- [Getting Started](../docs/GETTING_STARTED.md)
- [Examples](../docs/EXAMPLES.md)
- [Remote Cache](../docs/REMOTE_CACHE.md)

## License

Licensed under the Apache-2.0 license. See [LICENSE.md](../LICENSE.md) for details.
