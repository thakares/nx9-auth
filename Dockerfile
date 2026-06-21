# --- Stage 1: Build the binary ---
FROM rust:1.85-bookworm AS builder

WORKDIR /usr/src/nx9-auth

# 1. Pre-build dependencies for caching
COPY Cargo.toml Cargo.lock ./
# Create dummy main.rs, lib.rs, and src/bin/bench.rs to compile dependencies first
RUN mkdir -p src/bin src/security src/identity src/db src/api src/audit src/config src/middleware src/error && \
    echo "fn main() {}" > src/main.rs && \
    echo "fn main() {}" > src/bin/bench.rs && \
    echo "" > src/lib.rs && \
    cargo build --release && \
    rm -rf src/

# 2. Copy the actual source files and build
COPY . .
# Touch main.rs, lib.rs and src/bin/bench.rs to force cargo to rebuild them with the actual contents
RUN touch src/main.rs src/lib.rs src/bin/bench.rs && \
    cargo build --release

# --- Stage 2: Run the binary ---
FROM debian:bookworm-slim AS runtime

# Install CA certificates, curl (for healthcheck), and SQLite CLI
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl sqlite3 && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root group and user
RUN groupadd -g 10001 nx9-auth && \
    useradd -u 10001 -g nx9-auth -m -s /usr/sbin/nologin nx9-auth

# Create standard system directories (system mode)
RUN mkdir -p /etc/nx9-auth /var/lib/nx9-auth /var/log/nx9-auth /var/backups/nx9-auth && \
    chown -R nx9-auth:nx9-auth /etc/nx9-auth /var/lib/nx9-auth /var/log/nx9-auth /var/backups/nx9-auth

# Copy the compiled release binary from builder
COPY --from=builder /usr/src/nx9-auth/target/release/nx9-auth /usr/local/bin/nx9-auth

# Switch to the non-root user
USER nx9-auth

# Set standard environment variables
ENV NX9_AUTH_CONFIG=/etc/nx9-auth/config.toml

# Expose server port
EXPOSE 8655

# Set entrypoint
ENTRYPOINT ["/usr/local/bin/nx9-auth"]
CMD ["serve"]
