# Remote Cache Guide

Self-hosted remote cache system for sharing build artifacts across machines and team members.

**Benefits:**
- Share build artifacts across your team
- Speed up CI pipelines by reusing cached builds
- Avoid rebuilding unchanged packages

## Quick Start

### 1. Start the Cache Server

```bash
# Build the cache server
cargo build --release --package polykit-cache

# Run it
./target/release/polykit-cache --storage-dir ./cache
```

Server runs on `http://localhost:8080` by default.

### 2. Configure Polykit Client

Add to your `polykit.toml`:

```toml
[remote_cache]
url = "http://localhost:8080"
```

Or use CLI flags:

```bash
polykit build --remote-cache-url http://localhost:8080
```

### 3. Build Your Project

```bash
polykit build
```

On first run, builds execute normally and upload artifacts to cache. On subsequent runs with unchanged inputs, builds are restored from cache.

## Configuration

### Workspace Configuration

In `polykit.toml`:

```toml
[remote_cache]
# Cache server URL (required)
url = "http://localhost:8080"

# Environment variables included in cache key
env_vars = ["NODE_ENV", "RUST_BACKTRACE"]

# File patterns for cache invalidation
input_files = ["src/**/*.ts", "**/*.rs", "package.json"]

# Maximum artifact size (bytes)
max_artifact_size = 1073741824  # 1GB

# Read-only mode (download only, no uploads)
read_only = false
```

### CLI Flags

Override config with CLI flags:

```bash
# Enable remote cache
polykit build --remote-cache-url http://cache.example.com

# Read-only mode (useful for PR builds)
polykit build --remote-cache-url http://cache.example.com --remote-cache-readonly

# Disable remote cache
polykit build --no-remote-cache
```

## Cache Key Generation

Cache keys are deterministically computed from:

1. **Package ID**: Unique identifier for the package
2. **Task name**: e.g., "build", "test"
3. **Command**: Exact command executed
4. **Environment variables**: Whitelisted env vars
5. **Input file hashes**: SHA-256 of all input files
6. **Dependency graph**: Hash of the dependency tree
7. **Toolchain version**: Language runtime version (node, rustc, go, python)

Any change to these inputs produces a different cache key, ensuring correctness.

## How It Works

**Cache Hit:** Polykit computes cache key → checks local cache → queries remote (`HEAD`) → downloads (`GET`) → extracts and verifies.

**Cache Miss:** Executes task → collects outputs → creates artifact (tar + zstd) → uploads (`PUT`) → stores locally.

**Graceful Degradation:** Network errors fall back to local execution. Upload failures don't fail builds.

## Deployment

**Local:** `polykit-cache --storage-dir ~/.polykit/cache`

**Team Server:** `polykit-cache --storage-dir /var/cache/polykit --bind 0.0.0.0 --port 8080`

**Docker:** `docker run -d -p 8080:8080 -v /path/to/cache:/var/cache/polykit polykit-cache:latest`

**Cloud:** Deploy to AWS EC2/ECS, GCP Compute/Cloud Run, Azure VM/Containers, or DigitalOcean.

See [polykit-cache README](../polykit-cache/README.md) for detailed deployment instructions.

## Production Considerations

**Authentication:** No built-in auth. Use reverse proxy (Nginx/Caddy) with HTTP Basic Auth, deploy behind VPN, or restrict via firewall.

**TLS:** Use reverse proxy for TLS termination (see [polykit-cache README](../polykit-cache/README.md)).

**Storage:** Local disk (single server), NFS (shared), or S3-compatible backends (scalable).

**Monitoring:** Track cache hit rate, throughput, storage usage, verification failures, network errors.

**Backup:** Artifacts are immutable and content-addressed. Can rebuild from source if cache is lost.

## Troubleshooting

**Cache misses:** Check env vars, toolchain version detection, input file patterns, system clock.

**Verification failures:** Network corruption (use TLS), disk corruption, cache key collision (rare).

**Storage growth:** Implement eviction (delete oldest, LRU), set filesystem limits.

**Slow transfers:** Faster network, reduce `max_artifact_size`, enable proxy compression, use CDN.

## Best Practices

- Start with build tasks, expand to tests later
- Monitor hit rates to justify infrastructure cost
- Use `--remote-cache-readonly` for PR builds
- Avoid including generated files in cache key
- Version cache URL to invalidate during migrations

## FAQ

**Q: Can I use S3 instead of local disk?**  
A: Reference implementation uses local disk. Extend storage module for S3 support.

**Q: Is there a hosted cache service?**  
A: No. Self-host only.

**Q: How is this different from TurboRepo's cache?**  
A: Similar concept, but self-hosted and designed for polyglot monorepos.

**Q: Can I share cache between different OSes?**  
A: Cache keys include toolchain version but not OS. Be careful with OS-specific outputs.

**Q: What happens if two builds upload the same key simultaneously?**  
A: First write wins, second gets 409 Conflict (non-fatal).

## Further Reading

- [polykit-cache README](../polykit-cache/README.md) - Server deployment guide
- [CONTRIBUTING](../CONTRIBUTING.md) - How to contribute
