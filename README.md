# HookRelay

HookRelay is a self-hostable webhook capture, inspection, and local delivery platform for developers. It receives webhooks on a public server while your local machine is offline, stores them, and later delivers them to your local applications through an authenticated agent.

The public server never directly accesses your local machine. The agent connects outbound, validates every delivery instruction against a local allowlist, and delivers only to explicitly configured local targets.

## Why

Webhook providers (Stripe, GitHub, Shopify) send events to a public URL. During local development, your machine is often offline, behind NAT, or not publicly reachable. Solutions like ngrok expose your machine directly, which is fragile and a security risk.

HookRelay takes a different approach: the public server captures and stores webhooks. A local agent on your machine connects outbound when ready, receives authorized delivery instructions, and delivers to your local applications. The server never initiates inbound connections to your machine.

## Architecture

```
Provider --HTTPS--> HookRelay Server (capture + persist)
                       ^
                       | outbound WSS (initiated by agent)
                       |
                    Agent --local HTTP--> Local application
```

Two components:

- **Server**: Public Axum HTTP server. Receives webhooks, stores events in SQLite, serves the admin dashboard, manages agents, dispatches delivery instructions.
- **Agent**: Local Rust CLI. Connects outbound to the server via WebSocket, validates delivery instructions, delivers to local HTTP targets, records history locally.

The server sends only a target identifier, never a URL. The agent resolves the identifier against its local allowlist and validates the URL before delivery. Routing is at the endpoint level, so different endpoints in the same project can deliver to different local targets.

## Quick start

### Server

```bash
git clone https://github.com/theadeyemiolayinka/hookrelay.git
cd hookrelay
cp .env.example .env
# Set ADMIN_PASSWORD and HOOKRELAY_SESSION_KEY (openssl rand -hex 32)
docker compose up -d
```

Open `http://localhost:8080` and sign in. Create a project, an endpoint, and an agent. Copy the agent ID and token.

See [Server Setup](docs/getting-started/server.md) for full instructions.

### Agent

Install the agent on your local machine:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/scripts/install-agent.sh | bash
```

Start the web UI and connection loop:

```bash
hookrelay web
```

Open `http://localhost:8787`, enter your server URL, agent ID, and token. The agent connects to the server automatically after login. Configure your local targets and routes from the browser.

See [Agent Setup](docs/getting-started/agent.md) for full instructions.

## Using HookRelay

1. Deploy the server with Docker Compose.
2. Sign in to the admin dashboard at `http://localhost:8080`.
3. Create a project (e.g. "payments").
4. Create one or more endpoints in that project (e.g. "paystack", "stripe"). Each endpoint has its own webhook URL.
5. Point your webhook provider at the endpoint URL.
6. Send a test webhook. Inspect it in the Events page.
7. Create an agent. Copy the agent ID and one-time token.
8. Install the agent on your local machine with `hookrelay web`.
9. Open the agent web UI at `http://localhost:8787` and log in with the agent ID and token.
10. Go to the Targets page and add a local target (e.g. `myapp` pointing to `http://localhost:8000/webhook`).
11. Go to the Routes page and connect an endpoint to a target. For example, route the "paystack" endpoint to the "myapp" target. Different endpoints in the same project can route to different targets.
12. Replay an event from the admin dashboard. The server sends a delivery instruction to the agent, which resolves the target and delivers to your local application.

## Configuration

See [.env.example](.env.example) and [Server Configuration](docs/server/configuration.md).

## Deployment

- [Docker](docs/deployment/docker.md)
- [Coolify/Dokploy](docs/deployment/coolify.md)
- [Production](docs/deployment/production.md)

## Development

```bash
git clone https://github.com/theadeyemiolayinka/hookrelay.git
cd hookrelay
cargo build
cargo test --workspace
```

See [Development Setup](docs/development/setup.md).

## Documentation

Full documentation is available at https://theadeyemiolayinka.github.io/hookrelay/

## Project status

HookRelay is in early development. The core capture, inspection, and local delivery workflow is functional. The architecture is intentionally simple: SQLite, no external message brokers, no Kubernetes, no cloud dependencies.

## License

Dual-licensed under MIT or Apache-2.0, at your option. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security reports should follow [SECURITY.md](SECURITY.md).

## Author

[TheAdeyemiOlayinka](https://github.com/theadeyemiolayinka)
