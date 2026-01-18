<div align="center">
  <h1>Polykit</h1>
</div>

<h3 align="center">
  <a href="docs/GETTING_STARTED.md"><b>Getting Started</b></a>
  &nbsp;&#183;&nbsp;
  <a href="docs/EXAMPLES.md"><b>Examples</b></a>
</h3>

<div align="center">
  <strong>Fast, language-agnostic monorepo orchestration.</strong>
  <br><br>
  
  [![Status](https://github.com/jonathanmagambo/Polykit/actions/workflows/rust.yml/badge.svg)](https://github.com/jonathanmagambo/Polykit/actions/workflows/rust.yml)
</div>

## What is Polykit?

Polykit orchestrates monorepos across multiple languages. It manages dependencies, executes tasks in order, and handles versioning.

**Think of Polykit as the brain that orchestrates your monorepo: it doesn't manage dependencies, it orchestrates them.**

<h1 align="center">Goals</h1>

- ⚡ **Fast** - Parallel execution, smart caching, optimized for 10k+ packages
- 🔗 **Cross-language** - Works with JavaScript, TypeScript, Python, Go, and Rust
- 📊 **Graph-first** - Dependency-driven execution ensures correct order
- 🎯 **Simple** - Minimal TOML configuration, convention over complexity
- 🛡️ **Safe** - Deterministic runs, automatic cycle detection
- 🚀 **Zero overhead** - Delegates to native tools, no reinventing wheels

## Installation

```bash
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit
cargo install --path .
```

Verify installation:

```bash
polykit scan
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

## Configuration

Each package needs a `polykit.toml`:

```toml
name = "package-name"
language = "rust"
public = true

[deps]
internal = ["dep1", "dep2"]

[tasks]
build = "cargo build"
test = "cargo test"
test.depends_on = ["build"]
```

Optional workspace config at repo root:

```toml
[workspace]
cache_dir = ".polykit/cache"
default_parallel = 4
```

<h1 align="center">Contributing and License</h1>

Polykit is free and open source, released under the **Apache-2.0 License**. Contributions are welcome! ❤️
