<div align="center">
  <h1>Polykit</h1>
</div>

<h3 align="center">
  <b>Getting Started</b>
  &nbsp;&#183;&nbsp;
  <b>Examples</b>
</h3>

<div align="center">
  <strong>Fast, language-agnostic monorepo orchestration.</strong>
</div>

<h1 align="center">What is Polykit?</h1>

Polykit is a production-grade monorepo orchestration tool written in Rust. It manages cross-language dependencies, executes tasks in dependency order, and handles semantic versioning across your entire monorepo.

**Think of Polykit as the brain that orchestrates your monorepo—it doesn't manage dependencies, it orchestrates them.**

<h1 align="center">Goals</h1>

- **Fast** – Parallel execution, smart caching, optimized graph operations
- **Cross-language** – Works with JavaScript, TypeScript, Python, Go, and Rust
- **Graph-first** – Dependency relationships drive all operations
- **Simple** – Minimal TOML configuration, convention over configuration
- **Safe** – Deterministic execution, circular dependency detection
- **Zero overhead** – Delegates to native tools, no dependency installation logic

<h1 align="center">Quick Start</h1>

```bash
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit
cargo build --release
./target/release/polykit scan
```

See the Getting Started guide for detailed instructions.

<h1 align="center">Features</h1>

### Core Capabilities

- **Package Discovery** – Automatically scans and discovers packages across your monorepo
- **Dependency Graph** – Builds and manages cross-language dependency relationships
- **Task Execution** – Runs tasks in topological order with parallel execution
- **Change Detection** – Identifies affected packages from file changes (git-aware)
- **Semantic Versioning** – Plans and executes version bumps across dependent packages
- **Smart Caching** – Fast incremental scans with bincode-based caching

### Performance

- Parallel file I/O with rayon
- Cached topological ordering
- Incremental scanning with mtime-based invalidation
- Fast path optimizations for common operations
- Streaming task output for real-time feedback

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
lint = "cargo clippy"
```

```bash
# Build all packages in dependency order
polykit build

# Build specific packages
polykit build api-server shared-utils

# Run tests with parallel execution
polykit test --parallel 4

# Release with automatic version bumps
polykit release api-server --bump minor

# Find affected packages from git changes
polykit affected --git
```

<h1 align="center">Commands</h1>

### `polykit scan`

Scans the `packages/` directory and lists all discovered packages.

```bash
polykit scan
polykit scan --json
```

### `polykit graph`

Displays the dependency graph in topological order.

```bash
polykit graph
polykit graph --json
```

### `polykit affected`

Determines which packages are affected by changed files.

```bash
polykit affected --git
polykit affected packages/my-package/src/file.ts
polykit affected --git --base main
```

### `polykit build`

Builds packages in dependency order.

```bash
polykit build                          # Build all packages
polykit build my-package               # Build specific packages
polykit build --parallel 4             # Build with parallel execution
polykit build --continue-on-error      # Continue on error
```

### `polykit test`

Runs tests in dependency order.

```bash
polykit test                           # Test all packages
polykit test my-package                # Test specific packages
polykit test --parallel 4              # Test with parallel execution
polykit test --continue-on-error       # Continue on error
```

### `polykit release`

Plans and executes semantic version bumps across the monorepo.

```bash
polykit release my-package --bump patch
polykit release my-package --bump minor
polykit release my-package --bump major
polykit release my-package --bump patch --dry-run
```

The release engine automatically bumps the target package version and updates dependent packages (patch version).

### `polykit why`

Shows dependency relationships for a package.

```bash
polykit why my-package
```

### `polykit validate`

Validates all `polykit.toml` files and the dependency graph.

```bash
polykit validate
polykit validate --json
```

### `polykit list`

Lists available tasks per package.

```bash
polykit list
polykit list --json
```

<h1 align="center">Supported Languages</h1>

Polykit includes adapters for:

- **JavaScript/TypeScript** – Reads `package.json`, bumps `version`
- **Python** – Reads `pyproject.toml`, supports Poetry and PEP 621
- **Go** – Detects `go.mod` (versioning not supported; Go uses semantic import versioning)
- **Rust** – Reads `Cargo.toml`, bumps `package.version`

Language adapters are pluggable—add support for any language by implementing the `LanguageAdapter` trait.

<h1 align="center">Configuration</h1>

### `polykit.toml`

Required fields:

- `name`: Unique package name
- `language`: One of `js`, `ts`, `python`, `go`, `rust`
- `public`: Whether the package is published

Optional sections:

- `[deps.internal]`: List of internal package dependencies
- `[tasks]`: Task name to shell command mapping

Example:

```toml
name = "api-server"
language = "rust"
public = true

[deps]
internal = ["shared-utils", "database-client"]

[tasks]
build = "cargo build --release"
test = "cargo test"
lint = "cargo clippy"
```

<h1 align="center">Documentation</h1>

<div align="center">

**Getting Started** – Installation, first project, CLI walkthrough.

**Examples** – Real-world usage examples and patterns.

</div>

<h1 align="center">Contributing and License</h1>

Polykit is free and open source, released under the **Apache-2.0 License**. Contributions are welcome!
