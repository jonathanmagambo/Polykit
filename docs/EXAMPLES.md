# Examples

## Multi-Language Monorepo

A monorepo with packages in different languages:

```
packages/
  frontend/          # TypeScript
  api-server/        # Rust
  shared-utils/      # Rust (used by api-server)
  data-processor/    # Python
  cli-tool/          # Go
```

### Frontend Package

```toml
# packages/frontend/polykit.toml
name = "frontend"
language = "ts"
public = true

[deps]
internal = ["shared-utils"]

[tasks]
build = "npm run build"
test = "npm test"
dev = "npm run dev"
```

### API Server Package

```toml
# packages/api-server/polykit.toml
name = "api-server"
language = "rust"
public = true

[deps]
internal = ["shared-utils"]

[tasks]
build = "cargo build --release"
test = "cargo test"
```

### Shared Utils Package

```toml
# packages/shared-utils/polykit.toml
name = "shared-utils"
language = "rust"
public = true

[deps]
internal = []

[tasks]
build = "cargo build --lib"
test = "cargo test"
```

## Complex Dependency Graph

```
packages/
  core/              # No dependencies
  database/          # Depends on core
  auth/              # Depends on core
  api/               # Depends on database, auth
  frontend/          # Depends on api
```

Build order: `core` → `database`, `auth` → `api` → `frontend`

## CI/CD Integration

### GitHub Actions

```yaml
name: CI

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install Polykit
        run: cargo install --path .
      - name: Build affected packages
        run: |
          affected=$(polykit affected --git)
          if [ -n "$affected" ]; then
            polykit build $affected
          fi
      - name: Test affected packages
        run: |
          affected=$(polykit affected --git)
          if [ -n "$affected" ]; then
            polykit test $affected
          fi
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

affected=$(polykit affected --git)
if [ -n "$affected" ]; then
  echo "Building affected packages: $affected"
  polykit build $affected
  polykit test $affected
fi
```

## Release Workflow

### 1. Plan Release

```bash
polykit release api-server --bump minor --dry-run
```

Output:
```
Release plan:
  api-server: Some("1.2.0") -> 1.3.0 (Minor)
  frontend: Some("2.1.0") -> 2.1.1 (Patch)
```

Note: `Some("version")` indicates the old version existed. `None` would indicate a new package without a version.

### 2. Execute Release

```bash
polykit release api-server --bump minor
```

Automatically updates versions in:
- `packages/api-server/Cargo.toml`
- `packages/frontend/package.json`

### 3. Publish Packages

After version bumps, publish in order:

```bash
polykit graph  # Shows publish order
```

## Advanced Patterns

### Parallel Execution

```bash
# Build with 8 parallel workers
polykit build --parallel 8

# Test with 4 parallel workers
polykit test --parallel 4
```

### Change Detection

```bash
# From git (compare to main)
polykit affected --git --base main

# From specific files
polykit affected \
  packages/api/src/handlers.rs \
  packages/shared/src/lib.rs
```

## Troubleshooting

### Circular Dependencies

Polykit detects and reports circular dependencies:

```bash
$ polykit graph
Error: Circular dependency detected: Cycle involving package-a
```

### Missing Dependencies

```bash
$ polykit build
Error: Package not found: missing-package. Available packages: pkg-a, pkg-b
```

### Invalid Configuration

```bash
$ polykit validate
All packages are valid
```

### Validation Errors

If you see validation errors, check that:
- Package and task names follow naming rules (alphanumeric with `-`, `_`, `.`, `@` only)
- Commands don't contain null bytes or embedded newlines
- Names don't start with `.` or `-`
