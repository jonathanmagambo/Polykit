<div align="center">
  <img src="assets/logo/logo.png" alt="Polykit Logo" width="120">
  
  <h1>Polykit</h1>
  
  [<img alt="github" src="https://img.shields.io/badge/github-jonathanmagambo/polykit-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/jonathanmagambo/polykit)
  [<img alt="crates.io" src="https://img.shields.io/crates/v/polykit.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/polykit)
  [<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/jonathanmagambo/Polykit/rust.yml?branch=main&style=for-the-badge" height="20">](https://github.com/jonathanmagambo/Polykit/actions?query=branch%3Amain)
  [<img alt="discord" src="https://img.shields.io/badge/Discord-%235865F2.svg?style=for-the-badge&logo=discord&logoColor=white" height="20">](https://discord.gg/5Y9jmtysua)
</div>

<h3 align="center">
  <a href="docs/GETTING_STARTED.md"><b>Getting Started</b></a>
  &nbsp;&#183;&nbsp;
  <a href="docs/EXAMPLES.md"><b>Examples</b></a>
</h3>

<div align="center">
  <strong>Fast, lightweight, monorepo orchestrator.</strong>
</div>

## Installation

Install from crates.io (recommended):

```bash
cargo install polykit
```

Or build from source:

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
