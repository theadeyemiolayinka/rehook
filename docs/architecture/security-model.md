# Security Model

## Fundamental property

The public server must never be able to arbitrarily browse a local network through connected agents. This is enforced by architecture:

- The server only sends delivery instructions that reference a known project and a target identifier.
- The agent resolves that identifier to a locally configured URL from its own allowlist.
- The agent rejects any instruction whose target is not in its allowlist.
- The agent rejects unsupported URL schemes (`file`, `ftp`, `gopher`, `data`, `javascript`, `ws`, `wss`).
- The agent only allows `http` and `https` schemes.
- The agent never executes shell commands or arbitrary code from the server.

## Endpoint security

Inbound endpoint identifiers are generated with 120 bits of entropy and are not sequential. Unguessability is not authentication. Disabled and unknown endpoints respond identically (HTTP 202) to avoid leaking existence.

## Request handling

- Bodies are bounded by `REHOOK_MAX_WEBHOOK_BODY_SIZE` (default 1 MB).
- Hop-by-hop and transport headers are not replayed. The shared list is in `crates/protocol`.
- Sensitive headers (Authorization, Cookie, Set-Cookie, X-API-Key, etc.) are masked in the dashboard by default. The raw view is available to authenticated administrators.

## Trusted proxy handling

The server does not blindly trust `X-Forwarded-For`, `X-Real-IP`, or `CF-Connecting-IP`. Set `REHOOK_TRUSTED_PROXY_HOPS` to the number of trusted proxy hops. When 0 (default), the direct socket peer is used.

## Authentication

- No public registration.
- Dashboard sessions use HTTP-only cookies. Tokens are stored hashed (HMAC-SHA256) in the database.
- Passwords are hashed with Argon2id.
- Agent tokens are separate from dashboard sessions, revocable, and shown once at creation.
- Bootstrap credentials are read only on first startup and never overwrite an existing administrator.

## Local delivery privacy

The agent does not send response bodies to the server. The server receives only: success/failure, HTTP status code, duration, and a short error category. Full response inspection is available locally on the agent.

## Agent web UI security

- Binds to `127.0.0.1` by default. Does not expose the UI publicly.
- Does not expose agent tokens in API responses.
- Target validation reuses the same security policy as CLI delivery.
- No credentials in URL query strings.
- The browser talks only to the local agent, never to the public server directly.

## Logging

Structured logging via `tracing`. Full webhook payloads, authorization headers, cookies, and agent tokens are never logged. Logs contain operational metadata only: event ID, endpoint ID, payload size, duration.

## Error handling

Internal errors (SQL errors, backtraces, internal paths) are never exposed to external users. Production responses do not include stack traces.

## Reporting

See [reporting.md](../security/reporting.md) for vulnerability reporting instructions.
