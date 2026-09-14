# Multi-stage Dockerfile for HookRelay.
#
# Stage 1: Build the admin dashboard (Node).
# Stage 2: Build the Rust binaries (server + agent).
# Stage 3: Minimal runtime image with the server and dashboard assets.
#
# The agent is built but shipped as a separate artifact; the runtime image
# only runs the server.

# --- Stage 1: Dashboard build ---
FROM node:22-slim AS dashboard-builder
WORKDIR /build
COPY dashboards/admin/package.json dashboards/admin/package-lock.json* ./dashboards/admin/
RUN cd dashboards/admin && npm ci
COPY dashboards/admin/ ./dashboards/admin/
RUN cd dashboards/admin && npm run build

# --- Stage 2: Rust build ---
FROM rust:1-bookworm AS rust-builder
WORKDIR /build
# Install sqlite dev headers for sqlx build.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsqlite3-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
COPY apps/ ./apps/
# Build the server binary.
RUN cargo build --release -p hookrelay-server
# Build the agent binary.
RUN cargo build --release -p hookrelay-agent

# --- Stage 3: Runtime ---
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libsqlite3-0 wget \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for the server process.
RUN groupadd --system hookrelay && useradd --system --gid hookrelay --home-dir /app hookrelay

WORKDIR /app

# Copy the server binary.
COPY --from=rust-builder /build/target/release/hookrelay-server /usr/local/bin/hookrelay-server

# Copy the agent binary (for users who want to extract it).
COPY --from=rust-builder /build/target/release/hookrelay /usr/local/bin/hookrelay

# Copy the built dashboard.
COPY --from=dashboard-builder /build/dashboards/admin/dist /app/dashboards/admin/dist

# Data directory for SQLite.
RUN mkdir -p /app/data && chown -R hookrelay:hookrelay /app
VOLUME ["/app/data"]

ENV HOOKRELAY_DATA_DIR=/app/data
ENV HOOKRELAY_DASHBOARD_DIR=/app/dashboards/admin/dist
ENV RUST_LOG="hookrelay=info,tower_http=info"

EXPOSE 8080

USER hookrelay

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:8080/healthz || exit 1

ENTRYPOINT ["hookrelay-server"]
