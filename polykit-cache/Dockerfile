FROM rust:1.75-bookworm as builder

WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY polykit/Cargo.toml polykit/Cargo.toml
COPY polykit-core/Cargo.toml polykit-core/Cargo.toml
COPY polykit-adapters/Cargo.toml polykit-adapters/Cargo.toml
COPY polykit-cache/Cargo.toml polykit-cache/Cargo.toml

# Copy source code
COPY polykit/ polykit/
COPY polykit-core/ polykit-core/
COPY polykit-adapters/ polykit-adapters/
COPY polykit-cache/ polykit-cache/

# Build release binary
RUN cargo build --release --package polykit-cache

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 polykit && \
    mkdir -p /var/cache/polykit && \
    chown -R polykit:polykit /var/cache/polykit

# Copy binary from builder
COPY --from=builder /app/target/release/polykit-cache /usr/local/bin/polykit-cache

# Switch to non-root user
USER polykit

# Expose port
EXPOSE 8080

# Default command
ENTRYPOINT ["polykit-cache"]
CMD ["--storage-dir", "/var/cache/polykit", "--bind", "0.0.0.0", "--port", "8080"]
