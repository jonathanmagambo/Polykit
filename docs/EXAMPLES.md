# Examples

## Multi-Language Monorepo

```
packages/
  frontend/      # TypeScript
  api-server/    # Rust
  shared-utils/  # Rust
  processor/     # Python
```

### Frontend (TypeScript)

```toml
name = "frontend"
language = "ts"
public = true

[deps]
internal = ["shared-utils"]

[tasks]
build = "npm run build"
test = "npm test"
```

### API Server (Rust)

```toml
name = "api-server"
language = "rust"
public = true

[deps]
internal = ["shared-utils"]

[tasks]
build = "cargo build --release"
test = "cargo test"
```

### Shared Utils (Rust)

```toml
name = "shared-utils"
language = "rust"
public = true

[deps]
internal = []

[tasks]
build = "cargo build --lib"
test = "cargo test"
```

## Task Dependencies

```toml
[tasks]
build = "cargo build"
test = "cargo test"
test.depends_on = ["build"]  # test runs after build
```

## Parallel Execution

```bash
polykit build --parallel 8
polykit test --parallel 4
```

## CI/CD

### GitHub Actions

```yaml
- name: Install Polykit
  run: cargo install --path .

- name: Build affected packages
  run: polykit affected --git | xargs polykit build

- name: Test affected packages
  run: polykit affected --git | xargs polykit test
```

## Release Workflow

```bash
# Preview changes
polykit release api-server --bump minor --dry-run

# Execute release
polykit release api-server --bump minor
```

This automatically bumps versions and updates dependents.
