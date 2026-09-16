# Changelog

All notable changes to Rehook will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.0.0] - Unreleased

### Added

- Webhook capture at `/i/{public_identifier}` with body size limits and non-enumerating behavior for unknown endpoints.
- SQLite persistence with WAL mode for events, projects, endpoints, agents, sessions, and deliveries.
- Administrator bootstrap from environment variables on first startup.
- Argon2id password hashing with HTTP-only session cookies.
- Dashboard API for projects, endpoints, events, agents, and replay.
- Admin dashboard (React) with event inspection, header masking, and payload viewer.
- Agent WebSocket gateway with protocol versioning, heartbeats, and subscriptions.
- Agent CLI with login, target configuration, route mapping, and connection management.
- Local target allowlist with scheme validation (http/https only, rejects file/ftp/gopher/data/javascript).
- Local delivery with hop-by-hop header filtering, no redirect following, and 30s timeout.
- Local SQLite delivery history on the agent.
- End-to-end replay: dashboard triggers replay, server dispatches instruction, agent validates and delivers locally.
- Agent web UI with local HTTP server bound to localhost.
- Shared design system between admin and agent dashboards.
- Multi-stage Dockerfile with non-root runtime user.
- Docker Compose configuration with healthcheck and persistent volume.
- Security headers middleware (X-Robots-Tag, X-Content-Type-Options, X-Frame-Options, Referrer-Policy).
- Shared hop-by-hop header constants in the protocol crate.
