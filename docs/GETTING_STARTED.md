# Getting Started

## Installation

### From Source

```bash
git clone https://github.com/jonathanmagambo/polykit.git
cd polykit
cargo build --release
```

The binary will be available at `target/release/polykit`.

### Using Cargo

```bash
cargo install --path .
```

## Your First Monorepo

### 1. Create Package Structure

Organize your monorepo with packages in the `packages/` directory:

```
packages/
  my-api/
    polykit.toml
    package.json  # or Cargo.toml, pyproject.toml, etc.
  my-utils/
    polykit.toml
    Cargo.toml
```

### 2. Define Package Configuration

Each package needs a `polykit.toml`:

```toml
name = "my-api"
language = "js"
public = true

[deps]
internal = ["my-utils"]

[tasks]
build = "npm run build"
test = "npm test"
lint = "npm run lint"
```

### 3. Scan Packages

```bash
polykit scan
```

This discovers all packages and displays them.

### 4. View Dependency Graph

```bash
polykit graph
```

Shows packages in topological order (dependencies before dependents).

### 5. Build Packages

```bash
# Build all packages
polykit build

# Build specific packages
polykit build my-api my-utils

# Build with parallel execution
polykit build --parallel 4
```

### 6. Run Tests

```bash
# Test all packages
polykit test

# Test specific packages
polykit test my-api

# Continue on error
polykit test --continue-on-error
```

## Common Workflows

### Change Detection

Find packages affected by file changes:

```bash
# From git diff
polykit affected --git

# From specific files
polykit affected packages/my-api/src/index.ts
```

### Release Management

Plan and execute version bumps:

```bash
# Dry run to see what would change
polykit release my-api --bump minor --dry-run

# Execute the release
polykit release my-api --bump minor
```

The release engine automatically:
- Bumps the target package version
- Updates dependent packages (patch version)
- Ensures correct publish order

### Dependency Analysis

Explore dependency relationships:

```bash
# See what a package depends on and what depends on it
polykit why my-api
```

## Configuration Reference

### Required Fields

- `name`: Unique package name
- `language`: One of `js`, `ts`, `python`, `go`, `rust`
- `public`: Whether the package is published

### Optional Sections

- `[deps.internal]`: List of internal package dependencies
- `[tasks]`: Task name to shell command mapping

### Full Example

```toml
name = "api-server"
language = "rust"
public = true

[deps]
internal = ["shared-utils", "database-client", "auth-middleware"]

[tasks]
build = "cargo build --release"
test = "cargo test --all-features"
lint = "cargo clippy --all-targets"
format = "cargo fmt --check"
```

## Next Steps

- Check out `docs/EXAMPLES.md` for real-world patterns
- Explore the `packages/` directory for example configurations
