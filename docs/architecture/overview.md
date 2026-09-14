# Architecture Overview

HookRelay is a webhook capture, inspection, and local replay platform. It has two main components: a public server and a local agent.

## Components

### Server (`apps/server`)

An Axum HTTP server that:
- Receives inbound webhooks at `/i/{public_identifier}`
- Persists events to SQLite
- Serves the admin dashboard API under `/api/*`
- Serves the admin dashboard frontend
- Maintains an authenticated WebSocket gateway for agents at `/agent/ws`
- Dispatches delivery instructions to connected agents

The server never initiates connections to a developer's local machine.

### Agent (`apps/agent`)

A Rust CLI that:
- Connects outbound to the server via WebSocket
- Authenticates with a revocable token
- Subscribes to projects
- Receives delivery instructions
- Validates targets against a local allowlist
- Delivers webhooks to local HTTP targets
- Records delivery history in a local SQLite database
- Optionally serves a local web UI on localhost

### Protocol (`crates/protocol`)

Shared, typed message definitions for the WebSocket connection. Both server and agent depend on this crate. The protocol only supports explicitly defined message types. The server can never instruct an agent to fetch an arbitrary URL or execute commands.

## Communication flow

```
Provider --HTTPS--> Server (capture + persist)
                      ^
                      | outbound WSS (initiated by agent)
                      |
                   Agent --local HTTP--> Local application
```

1. A provider sends a webhook to `https://hooks.example.com/i/{id}`.
2. The server validates the endpoint, enforces limits, captures the request, and stores the event.
3. The developer triggers a replay from the admin dashboard.
4. The server sends a delivery instruction referencing a target identifier (not a URL).
5. The agent validates the instruction against its local allowlist.
6. The agent reconstructs a safe HTTP request and sends it to the local target.
7. The agent records the result locally and sends minimal metadata to the server.

## Trust boundaries

1. Internet: untrusted. Webhook payloads and headers are untrusted data.
2. Public server: trusted by the administrator, but cannot gain arbitrary access to a developer's local network.
3. Local agent: trusted by the machine owner. Independently validates every delivery instruction.
4. Dashboard user: an authenticated administrator. Dashboard access does not grant arbitrary remote execution.

The fundamental security property: the public server must never be able to arbitrarily browse a local network through connected agents. This is enforced by architecture, not by convention.

See [security-model.md](security-model.md) for details.
