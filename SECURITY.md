# Security

This document describes the trust model and security boundaries of HookRelay.
It is kept aligned with the implementation. If behavior differs from this
document, the implementation is the source of truth and this document must be
updated.

## Trust boundaries

1. **Internet.** Untrusted. Webhook payloads, headers, and remote addresses
   are untrusted data. Remote addresses are metadata only and are never
   forwarded into local applications as trusted proxy headers by default.
2. **Public server.** Trusted by the administrator. It must never gain
   arbitrary access to a developer's local network. It can only request
   delivery of a webhook event through an explicitly configured project and
   target relationship.
3. **Local agent.** Trusted by the machine owner. It independently validates
   every delivery instruction against its local configuration.
4. **Dashboard user.** An authenticated administrator. Dashboard access does
   not grant arbitrary remote execution.

## Fundamental property

The public server must never be able to arbitrarily browse a local network
through connected agents. This is enforced by architecture:

- The server only sends delivery instructions that reference a known project
  and a known target identifier.
- The agent resolves that identifier to a locally configured URL.
- The agent rejects any instruction whose target is not in its allowlist.
- The agent rejects unsupported URL schemes.
- The agent never executes shell commands or arbitrary code from the server.

## Endpoint security

Inbound endpoint identifiers are generated with sufficient entropy and are not
sequential. Unguessability is not authentication. Endpoints may be disabled.
Disabled and unknown endpoints respond identically to avoid leaking existence.

## Request handling

- Bodies are bounded by `MAX_WEBHOOK_BODY_SIZE` and rejected before excessive
  memory is consumed.
- Rate limiting is scoped per route class (login, dashboard API, inbound
  endpoints, WebSocket).
- Hop-by-hop headers and transport headers (`Host`, `Connection`,
  `Transfer-Encoding`, `Content-Length`, `Keep-Alive`, `Proxy-Authorization`,
  `TE`, `Trailer`, `Upgrade`) are not replayed blindly.
- Sensitive headers may be masked in the dashboard.

## Trusted proxy handling

The server does not blindly trust `X-Forwarded-For`, `X-Real-IP`, or
`CF-Connecting-IP`. Trusted proxy configuration is explicit. When no trusted
proxy is configured, the direct socket peer is used as the remote address.

## Authentication

- No public registration.
- Dashboard sessions use HTTP-only secure cookies. Tokens are not stored in
  `localStorage`.
- Passwords are hashed with Argon2id.
- Agent tokens are separate, revocable, and scoped. They do not grant
  administrative dashboard access.
- Bootstrap credentials (`ADMIN_USERNAME`, `ADMIN_PASSWORD`) are read only on
  first startup and never overwrite an existing administrator. Credentials
  are never written to logs.

## Local delivery privacy

By default the full local application response body is not sent to the public
server. The server receives only minimal delivery metadata (status code,
duration, success). Full response inspection is available locally on the
agent. This prevents the server from becoming a collection point for
potentially sensitive local responses (stack traces, debug output, internal
API responses).

## Logging

Structured logging via `tracing`. Full webhook payloads, authorization
secrets, and cookies are never logged by default. Logs contain operational
metadata: event id, endpoint id, payload size, duration, request id.

## Error handling

Internal errors (SQL errors, backtraces, internal paths) are never exposed to
external users. Production responses do not include stack traces.

## Dashboard static file serving

The server serves the built admin dashboard from the directory configured by
`HOOKRELAY_DASHBOARD_DIR` (default: `./dashboards/admin/dist`). This directory
is served to any HTTP client via `tower_http::services::ServeDir`.

Misconfiguring this value is a security risk:

- Pointing it at the filesystem root would expose the entire filesystem.
- Pointing it at the data directory would expose the SQLite database.
- Including `..` components could traverse outside the intended directory.

The server validates this path at startup and refuses to start if the path
contains parent traversal components or points at the filesystem root. If
`index.html` is missing, a warning is logged.

When deploying behind a reverse proxy, ensure the proxy forwards all
non-API, non-webhook paths to the server so the SPA fallback to `index.html`
works correctly. Direct requests to dashboard routes like `/settings` or
`/events` must reach the server, which serves `index.html` for any path that
does not match a static file or API route.

The agent web UI uses the same serving strategy for the agent dashboard, but
binds to `127.0.0.1` only, so only the local machine can access it.
