<div align="center">
  <h1>Polykit Cache Server</h1>
  
  [<img alt="github" src="https://img.shields.io/badge/github-jonathanmagambo/polykit-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/jonathanmagambo/polykit)
  [<img alt="crates.io" src="https://img.shields.io/crates/v/polykit-cache.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/polykit-cache)
  [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-polykit--cache-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/polykit-cache)
  [<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/jonathanmagambo/Polykit/rust.yml?branch=main&style=for-the-badge" height="20">](https://github.com/jonathanmagambo/Polykit/actions?query=branch%3Amain)
</div>

Self-hosted HTTP cache server for Polykit remote caching.

**Features:** Self-hosted, single binary, content-addressed storage, directory sharding, atomic operations, integrity verification, streaming I/O, graceful shutdown.

## Installation

Install from crates.io (recommended):

```bash
cargo install polykit-cache
```

Or build from source:

```bash
cargo build --release --package polykit-cache
```

Binary: `target/release/polykit-cache`

## Usage

### Starting the Server

Basic usage with defaults (port 8080, `./cache` storage):

```bash
polykit-cache
```

### Configuration Options

```bash
polykit-cache \
  --storage-dir /var/cache/polykit \
  --max-size 5368709120 \
  --bind 0.0.0.0 \
  --port 8080 \
  --log-level info
```

**Options:**
- `--storage-dir`: Directory for artifact storage (default: `./cache`)
- `--max-size`: Maximum artifact size in bytes (default: 1GB)
- `--bind`: Bind address (default: `127.0.0.1`)
- `--port`: Port number (default: `8080`)
- `--log-level`: Log level - trace, debug, info, warn, error (default: `info`)

### Client Configuration

**CLI:** `polykit build --remote-cache-url http://localhost:8080`

**Config file (`polykit.toml`):**
```toml
[remote_cache]
url = "http://localhost:8080"
```

## Deployment

### Docker

**Build:**
```bash
docker build -f polykit-cache/Dockerfile -t polykit-cache .
docker run -p 8080:8080 -v /path/to/cache:/var/cache/polykit polykit-cache
```

**Or use docker-compose:**
```bash
docker-compose -f polykit-cache/docker-compose.yml up -d
```

### Systemd Service

Create `/etc/systemd/system/polykit-cache.service`:

```ini
[Unit]
Description=Polykit Cache Server
After=network.target

[Service]
Type=simple
User=polykit
Group=polykit
ExecStart=/usr/local/bin/polykit-cache \
  --storage-dir /var/cache/polykit \
  --bind 0.0.0.0 \
  --port 8080 \
  --log-level info
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl enable polykit-cache
sudo systemctl start polykit-cache
```

### Reverse Proxy (Nginx)

For production deployments, use a reverse proxy:

```nginx
upstream polykit_cache {
    server 127.0.0.1:8080;
}

server {
    listen 80;
    server_name cache.example.com;

    client_max_body_size 1G;

    location / {
        proxy_pass http://polykit_cache;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## API

### Upload Artifact

```http
PUT /v1/artifacts/{cache_key}
Content-Type: application/octet-stream

<zstd-compressed artifact data>
```

**Response:**
- `201 Created` - Upload successful
- `400 Bad Request` - Invalid cache key format
- `409 Conflict` - Artifact already exists
- `413 Payload Too Large` - Artifact exceeds size limit
- `422 Unprocessable Entity` - Verification failed

### Download Artifact

```http
GET /v1/artifacts/{cache_key}
```

**Response:**
- `200 OK` - Returns compressed artifact with headers:
  - `Content-Type: application/zstd`
  - `Content-Length: <size>`
  - `X-Artifact-Hash: <sha256>`
- `404 Not Found` - Artifact does not exist

### Check Artifact Existence

```http
HEAD /v1/artifacts/{cache_key}
```

**Response:**
- `200 OK` - Artifact exists (includes same headers as GET)
- `404 Not Found` - Artifact does not exist

## Storage Layout

Artifacts are stored with directory sharding:

```
<storage-dir>/
  aa/
    bb/
      <cache_key>.zst      # Compressed artifact
      <cache_key>.json     # Metadata
```

The first 4 characters of the cache key determine the directory structure (`aa/bb/`).

## Security

**No built-in auth** - Deploy behind firewall/VPN or use reverse proxy (Nginx/Caddy) with TLS and auth.

**Storage limits** - Set `--max-size` to prevent exhaustion. Run as dedicated user with restricted permissions.

## Monitoring

Logs to stdout. Monitor uploads/downloads, verification failures, storage errors, shutdown signals. Integrate with journald, fluentd, or Loki.

## Performance

**Concurrent uploads** - No global locks, scales with CPU cores. **Streaming** - No full buffering. **Atomic writes** - Temp file + rename. **Directory sharding** - Prevents filesystem slowdowns.

## License

See LICENSE.md in the repository root.
