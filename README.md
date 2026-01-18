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

<h1 align="center">Contributing And License</h1>

> [!IMPORTANT]  
> The project is in active development and being crafted meticulously. **If you plan to contribute to the project, now is the time to provide a helping hand for the hardworking team**. Polykit is written in Rust and orchestrates monorepos across multiple languages (JavaScript, TypeScript, Python, Go, and Rust). We're continuously improving language adapter support and adding new features.
>
> In addition, the project uses the **[Apache-2.0 License](LICENSE.md)**, which allows you to:
> - View the source, learn from it, and use it freely
> - Modify and distribute the software
> - Use it in commercial and private projects
> - The only requirement is to include the license and copyright notice

When it comes to contributing and forking, Polykit is free and open source to use, released under the <strong>Apache-2.0 License</strong>. 
Contributions are welcome with wide open arms as Polykit is looking to foster a community. Proceed to take a look at 
[CONTRIBUTING.md](./CONTRIBUTING.md) for more information on how to get started as well as the codebase to learn
from it. We sincerely and deeply are grateful and thankful for your efforts.

![Alt](https://repobeats.axiom.co/api/embed/9ceb1134a6a3465f667ee910f58adec438bd48a2.svg "Repobeats analytics image")
