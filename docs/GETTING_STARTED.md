# Getting Started

## Table of Contents

- [Installation](#installation)
- [Your First Monorepo](#your-first-monorepo)
  - [Create Package Structure](#1-create-package-structure)
  - [Configure Packages](#2-configure-packages)
  - [Use Polykit](#3-use-polykit)
- [Common Workflows](#common-workflows)
  - [Change Detection](#change-detection)
  - [Release Management](#release-management)
  - [Watch Mode](#watch-mode)
  - [Dependency Analysis](#dependency-analysis)
- [Configuration](#configuration)
  - [Required Fields](#required-fields)
  - [Optional Fields](#optional-fields)
  - [Workspace Configuration](#workspace-configuration)

## Installation

Install Polykit globally so you can use the `polykit` command from anywhere:

```bash
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit
cargo install --path .
```

After installation, verify it works:

```bash
polykit scan
```

The `polykit` command is now available in your PATH.

## Your First Monorepo

### 1. Create Package Structure

Organize your monorepo with packages in a `packages/` directory (or any directory you prefer):

```
packages/
  my-api/
    polykit.toml
    package.json
  my-utils/
    polykit.toml
    Cargo.toml
```

Each package directory should contain:
- A `polykit.toml` configuration file
- The package's native files (e.g., `package.json`, `Cargo.toml`, `pyproject.toml`)

### 2. Configure Packages

Each package needs a `polykit.toml` file in its directory. This file defines the package's metadata, dependencies, and tasks:

```toml
name = "my-api"
language = "js"
public = true

[deps]
internal = ["my-utils"]

[tasks]
build = "npm run build"
test = "npm test"
```

- `name` - Unique identifier for your package
- `language` - The programming language (`js`, `ts`, `python`, `go`, or `rust`)
- `public` - Whether this package is published
- `[deps.internal]` - List of other packages in your monorepo this package depends on
- `[tasks]` - Commands to run for this package

### 3. Use Polykit

Now you can use Polykit commands to work with your monorepo:

```bash
polykit scan              # Discover all packages
polykit graph             # View packages in dependency order
polykit build             # Build all packages (respects dependencies)
polykit build my-api      # Build specific package and its dependencies
polykit test --parallel 4 # Run tests in parallel (4 workers)
```

Polykit automatically:
- Discovers all packages by scanning for `polykit.toml` files
- Builds a dependency graph
- Executes tasks in the correct order (dependencies first)
- Runs independent packages in parallel when possible

## Common Workflows

### Remote Caching

Share build artifacts across machines and team members using Polykit's self-hosted remote cache:

```bash
# Start the cache server (in one terminal)
polykit-cache --storage-dir ./cache --bind 0.0.0.0 --port 8080

# Use remote cache in builds (in another terminal)
polykit build --remote-cache-url http://localhost:8080
```

Or configure in `polykit.toml`:

```toml
[workspace]
cache_dir = ".polykit/cache"
default_parallel = 4

[remote_cache]
url = "http://localhost:8080"
```

**Benefits:**
- Share build artifacts across your team
- Speed up CI pipelines by reusing cached builds
- Avoid rebuilding unchanged packages

See [Remote Cache Guide](./REMOTE_CACHE.md) for detailed setup and deployment instructions.

### Change Detection

Find which packages are affected by file changes. This is useful for CI/CD to only build/test what changed:

```bash
polykit affected --git                    # Detect from git diff (compares to HEAD)
polykit affected --git --base main        # Compare to main branch
polykit affected packages/api/src/file.ts # Check specific files
```

Returns a list of packages that need to be rebuilt/tested.

### Release Management

Automatically bump package versions and update dependents:

```bash
polykit release my-api --bump minor --dry-run  # Preview what will change
polykit release my-api --bump minor             # Execute the version bump
```

The release command:
- Bumps the target package version (major, minor, or patch)
- Automatically bumps dependent packages (patch version)
- Updates version files (`package.json`, `Cargo.toml`, `pyproject.toml`) based on language

### Watch Mode

Automatically rebuild when files change:

```bash
polykit watch build              # Watch and rebuild all packages
polykit watch build my-api       # Watch and rebuild specific packages
polykit watch test --debounce 500 # Custom debounce delay in milliseconds
```

The watch command monitors your packages directory and re-runs the specified task when files change.

### Dependency Analysis

Understand package relationships:

```bash
polykit why my-api  # Show what my-api depends on and what depends on it
```

This command displays:
- Direct dependencies (what `my-api` needs)
- Direct dependents (what packages need `my-api`)

## Configuration

### Required Fields

Each package must have these fields in `polykit.toml`:

- `name` - Unique package name (alphanumeric with `-`, `_`, `.`, `@` only)
- `language` - One of: `js`, `ts`, `python`, `go`, `rust`
- `public` - Boolean indicating if the package is published

### Optional Fields

- `[deps.internal]` - Array of internal package dependencies
- `[tasks]` - Task definitions mapping task names to shell commands
- `task.depends_on` - Array of task names that must run before this task

### Workspace Configuration

Create a `polykit.toml` at your repository root to configure workspace settings:

```toml
[workspace]
cache_dir = ".polykit/cache"
default_parallel = 4
```

- `cache_dir` - Directory for caching scan results (speeds up subsequent scans)
- `default_parallel` - Default number of parallel workers for build/test commands
- `[remote_cache]` - Remote cache configuration (see [Remote Cache Guide](./REMOTE_CACHE.md))

See `docs/EXAMPLES.md` for more examples.
