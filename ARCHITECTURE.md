# Architecture

HookRelay is a webhook capture, inspection, and local replay platform. It is
composed of three cooperating parts: a public server, a local agent, and a
dashboard. The dashboard is served by the server from the same origin.

## Components

### Server (`apps/server`)

A single Axum process. Responsibilities:

- Receive inbound webhooks at `POST /i/{public_identifier}` (and a small
  explicit set of other methods).
- Apply body size limits, rate limits, and abuse protections before consuming
  request bodies.
- Persist events and metadata to SQLite.
- Serve the dashboard API under `/api/*` (authenticated, cookie sessions).
- Serve the compiled dashboard frontend from the same origin.
- Maintain an authenticated WebSocket gateway for agents at `/agent/ws`.
- Dispatch delivery instructions to connected agents on capture and on
  resubscribe (catch-up for events missed while offline). The server never
  contacts a developer's local machine directly.

The server is the only component that listens on a public port. It listens on
`0.0.0.0:8080` and expects HTTPS termination from the deployment environment
(Coolify, Dokploy, Caddy, Cloudflare, etc.).

### Agent (`apps/agent`)

A Rust CLI. Responsibilities:

- Authenticate to the server using a revocable agent token (separate from
  dashboard sessions).
- Establish and maintain an outbound WebSocket connection to the server.
- Subscribe to one or more endpoints, declaring which local target each
  endpoint's deliveries route to.
- Hold an explicit allowlist of local delivery targets.
- Receive delivery instructions, validate them against the local allowlist,
  reconstruct a safe HTTP request, and send it to the local application.
- Record delivery attempts in a local SQLite database.
- Send only minimal delivery metadata back to the server (status code,
  duration, success). Response bodies are never sent to the server by default.

The agent rejects unsupported URL schemes (`file`, `ftp`, `gopher`, `data`,
`javascript`, etc.) and only allows `http`/`https` to explicitly configured
destinations.

### Dashboard (`dashboard/`)

React + TypeScript + Vite. Built into static assets that the server serves in
production. Same-origin as the API, so no wildcard CORS is required. The
server serves the dashboard as a single-page application: any path that does
not match a static file or API route falls back to `index.html` so React
Router can handle client-side routing. The dashboard directory is validated at
startup to prevent misconfiguration (see SECURITY.md).

### Protocol (`crates/protocol`)

Shared, strongly typed message definitions used over the WebSocket. Both the
server and the agent depend on this crate. Messages are versioned. The protocol
only supports explicitly defined message types; it is not a generic command
execution protocol.

## Trust boundaries

1. **Internet.** Untrusted. Webhook payloads, headers, and remote addresses
   are untrusted data. Remote addresses are metadata only.
2. **Public server.** Trusted by the administrator, but it must never gain
   arbitrary access to a developer's local network. It can only request
   delivery of a webhook event through an explicitly configured endpoint and
   target relationship.
3. **Local agent.** Trusted by the machine owner. It independently validates
   every delivery instruction against its local configuration. The server
   cannot instruct it to fetch an arbitrary URL.
4. **Dashboard user.** An authenticated administrator. Dashboard access does
   not grant arbitrary remote execution.

The fundamental security property: the public server must never be able to
arbitrarily browse a local network through connected agents. This is enforced
by architecture, not by convention.

## Communication flow

```
Provider --HTTPS--> Server (capture + persist)
                      ^
                      | outbound WSS (initiated by agent)
                      |
                   Agent --local HTTP--> Local application (.test / localhost)
```

1. Provider sends a webhook to `https://hooks.example.com/i/{id}`.
2. Server validates the endpoint (including optional signature validation),
   enforces limits, captures the request, and stores the event. The provider
   gets an appropriate response immediately.
3. Server looks up `agent_subscriptions` for that endpoint and dispatches a
   delivery instruction to each subscribed, connected agent. The instruction
   references the event, the endpoint, and the target identifier the agent
   declared when it subscribed.
4. If the agent is offline, the event stays stored with no delivery row.
   When the agent reconnects and resubscribes, the server dispatches every
   event on that endpoint that has no prior delivery attempt for that agent
   (catch-up). Events already delivered are not re-sent.
5. A developer can also trigger a manual replay from the dashboard to a
   connected agent and a chosen target.
6. Agent validates the instruction against its local allowlist.
7. Agent reconstructs a safe HTTP request and sends it to the local target.
8. Agent records the result locally and sends minimal metadata to the server.

## Database

SQLite initially. WAL mode, busy timeout, a small connection pool sized for
SQLite. Migrations are embedded in the binary via `sqlx::migrate!`. The
schema avoids SQLite-only types where reasonable to keep a future PostgreSQL
port low-friction. Payloads are stored inline initially (bounded by
`MAX_WEBHOOK_BODY_SIZE`); the storage interface is abstracted enough to allow
filesystem or external storage later without rewriting the capture path.

## Authentication

- **Dashboard**: Argon2id password hash, HTTP-only secure cookies, session
  expiration and invalidation. No public registration.
- **Agent**: separate revocable agent tokens, never session cookies.
- **Bootstrap**: the first administrator is created from `ADMIN_USERNAME` and
  `ADMIN_PASSWORD` environment variables on first startup only. Existing
  administrators are never overwritten. Credentials are never logged.

## Configuration

All runtime configuration comes from environment variables with safe
defaults. See `.env.example`. Trusted proxy configuration is explicit; the
server does not blindly trust `X-Forwarded-For` or `CF-Connecting-IP`.

## Deployment

Single Docker image (multi-stage) builds both Rust binaries and the frontend.
SQLite persists through a mounted `/data` volume. The server listens on
`8080`; the deployment environment terminates TLS. `docker compose up -d` is
the intended deployment path. Coolify and Dokploy are supported through standard Docker
plus environment variables and a persistent volume.
