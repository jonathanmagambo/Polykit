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
  
  [![Build Status](https://github.com/jonathanmagambo/polykit/workflows/CI/badge.svg)](https://github.com/jonathanmagambo/polykit/actions)
</div>

<h1 align="center">What is Polykit?</h1>

Polykit is a production-grade monorepo orchestration tool written in Rust. It manages cross-language dependencies, executes tasks in dependency order, and handles semantic versioning across your entire monorepo.

**Think of Polykit as the brain that orchestrates your monorepo—it doesn't manage dependencies, it orchestrates them.**

<h1 align="center">Goals</h1>

⚡ **Fast** Parallel execution, smart caching, optimized graph operations

🔗 **Cross-language** Works with JavaScript, TypeScript, Python, Go, and Rust

📊 **Graph-first** Dependency relationships drive all operations

🎯 **Simple** Minimal TOML configuration, convention over configuration

🛡️ **Safe** Deterministic execution, circular dependency detection

🚀 **Zero overhead** Delegates to native tools, no dependency installation logic

<h1 align="center">Quick Start</h1>

```bash
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit
cargo build --release
./target/release/polykit scan
```

See the [Getting Started Guide](docs/GETTING_STARTED.md) for detailed instructions.

<h1 align="center">Example</h1>

```toml
# packages/api-server/polykit.toml
name = "api-server"
language = "rust"
public = true

[deps]
internal = ["shared-utils", "database-client"]

[tasks]
build = "cargo build --release"
test = "cargo test"
```

```bash
# Build all packages in dependency order
polykit build

# Run tests with parallel execution
polykit test --parallel 4

# Release with automatic version bumps
polykit release api-server --bump minor

# Watch for changes and rebuild
polykit watch build
```

<h1 align="center">Commands</h1>

**`polykit scan`** Discover packages across your monorepo

**`polykit graph`** Visualize dependency relationships

**`polykit affected`** Find packages affected by changes (git-aware)

**`polykit build`** Execute build tasks in dependency order

**`polykit test`** Run tests with parallel execution

**`polykit release`** Plan and execute semantic version bumps

**`polykit watch`** Watch for changes and rebuild automatically

**`polykit why`** Show dependency relationships for a package

**`polykit validate`** Validate configuration and dependency graph

**`polykit list`** List available tasks per package

<h1 align="center">Supported Languages</h1>

**JavaScript/TypeScript** Reads `package.json`, bumps `version`

**Python** Reads `pyproject.toml`, supports Poetry and PEP 621

**Go** Detects `go.mod` (versioning not supported; Go uses semantic import versioning)

**Rust** Reads `Cargo.toml`, bumps `package.version`

Language adapters are pluggable—add support for any language by implementing the `LanguageAdapter` trait.

<h1 align="center">Configuration</h1>

Each package requires a `polykit.toml`:

```toml
name = "api-server"
language = "rust"
public = true

[deps]
internal = ["shared-utils", "database-client"]

[tasks]
build = "cargo build --release"
test = "cargo test"
test.depends_on = ["build"]
```

Optional workspace config (`polykit.toml` at repo root):

```toml
[workspace]
cache_dir = ".polykit/cache"
default_parallel = 4
```

<h1 align="center">Contributing and License</h1>

Polykit is free and open source, released under the **Apache-2.0 License**. Contributions are welcome! ❤️
