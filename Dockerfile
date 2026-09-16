# Multi-stage Dockerfile for Rehook.
#
# Stage 1: Build the admin and agent dashboards (Node).
# Stage 2: Build the Rust binaries (server + agent). The dashboards are
#          embedded into the binaries at compile time, so they must be
#          built first.
# Stage 3: Minimal runtime image. Runs the server as a non-root user with
#          the SQLite data directory on a mounted volume.
#
# The agent binary is included in the image for convenience; it is also
# distributed as a standalone release artifact.

# --- Stage 1: Dashboard builds ---
FROM node:22-bookworm-slim AS dashboard-builder
WORKDIR /build

# Install dependencies first for layer caching. The dashboards share a
# local package (@rehook/shared) referenced as file:../shared.
COPY dashboards/shared/package.json ./dashboards/shared/
COPY dashboards/admin/package.json dashboards/admin/package-lock.json* ./dashboards/admin/
COPY dashboards/agent/package.json dashboards/agent/package-lock.json* ./dashboards/agent/

COPY dashboards/shared/ ./dashboards/shared/
RUN cd dashboards/admin && npm ci
RUN cd dashboards/agent && npm ci

COPY dashboards/admin/ ./dashboards/admin/
COPY dashboards/agent/ ./dashboards/agent/
RUN cd dashboards/admin && npm run build
RUN cd dashboards/agent && npm run build

# --- Stage 2: Rust build ---
FROM rust:1.88-bookworm AS rust-builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY apps/ ./apps/
# The dashboards are embedded into the binaries at compile time.
COPY --from=dashboard-builder /build/dashboards ./dashboards

RUN cargo build --release --locked -p rehook-server -p rehook-agent

# --- Stage 3: Runtime ---
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libsqlite3-0 wget \
    && rm -rf /var/lib/apt/lists/*

# Non-root user for the server process.
RUN groupadd --system rehook && useradd --system --gid rehook --home-dir /app rehook

WORKDIR /app

COPY --from=rust-builder /build/target/release/rehook-server /usr/local/bin/rehook-server
COPY --from=rust-builder /build/target/release/rehook /usr/local/bin/rehook

# Data directory for SQLite. Mount a named volume or host path here to
# persist events, sessions, and configuration across container restarts.
RUN mkdir -p /app/data && chown -R rehook:rehook /app
VOLUME ["/app/data"]

ENV REHOOK_DATA_DIR=/app/data
ENV RUST_LOG="rehook=info,tower_http=info"

EXPOSE 8080

USER rehook

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["rehook-server"]
