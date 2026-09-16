# Getting Started: Agent

The Rehook agent runs on your local machine. It connects to your Rehook server and delivers webhooks to your local applications.

You can set up the agent entirely from the web UI, or use the terminal. Both paths work together.

## Install the agent

### One-line install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash
```

This detects your OS and architecture, downloads the latest release from GitHub, and installs the `rehook` binary. It works on:

- macOS (Intel and Apple Silicon)
- Linux (x86_64 and ARM64)

To install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash -s -- --version v1.0.0
```

### Update the agent

Re-run the install command. It overwrites the existing binary:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash
```

Check your current version first:

```bash
rehook version
```

### Build from source

If you have Rust installed:

```bash
git clone https://github.com/theadeyemiolayinka/rehook.git
cd rehook
cargo build --release -p rehook-agent
```

The binary is at `target/release/rehook`.

## Concepts

Before setting up the agent, understand these three concepts:

- **Target**: A local HTTP destination on your machine (e.g. `http://localhost:8000/webhook`). You give each target a short ID like `myapp`. The agent only ever sends requests to URLs you configure here. The server never sends URLs.
- **Endpoint**: An inbound webhook URL on your Rehook server (e.g. a "paystack" endpoint in a "payments" project). Each endpoint has its own URL and can route to a different local target.
- **Route**: A mapping from an endpoint to a target. When the server sends a delivery for an event on that endpoint, the agent uses the route to find the right local target and delivers there.

## Set up the agent

### Option A: Web UI (recommended)

Start the web UI and connection loop:

```bash
rehook web
```

Open `http://localhost:8787` in your browser. If the agent is not yet configured, you will see a login page.

Enter the details from your server admin dashboard (Agents page):

- **Server URL**: your Rehook server URL
- **Agent ID**: the UUID shown when you created the agent
- **Agent Token**: the one-time token shown when you created the agent
- **Agent Name**: a friendly name for this machine (optional)

Click Connect. The agent validates the server, stores your credentials, and connects automatically. You do not need to run a separate command.

Then configure your local targets and routes:

1. Go to the **Targets** page. Add a target (e.g. ID `myapp`, URL `http://localhost:8000/webhook`).
2. Go to the **Routes** page. Click New route. Pick an endpoint from the dropdown (grouped by project) and connect it to the target you just created.

The agent subscribes to the endpoints you have routes for. When the server receives a webhook on one of those endpoints, it sends a delivery instruction to the agent, which resolves the target and delivers to your local application.

The Connection page shows your connection status and the endpoints you are subscribed to.

### Option B: Terminal

If you prefer the terminal, here are the equivalent commands:

```bash
# Log in (get the agent ID and token from the admin dashboard)
rehook login \
  --server https://hooks.example.com \
  --agent-id 550e8400-e29b-41d4-a716-446655440000 \
  --token re_your_token_here \
  --name my-laptop

# Add a local target
rehook target add myapp http://localhost:8000/webhook

# Map an endpoint to the target (use the endpoint ID from the admin dashboard)
rehook route add 550e8400-e29b-41d4-a716-446655440000 myapp

# Start the web UI and connection loop
rehook web
```

The web UI and terminal share the same configuration. Changes made in one are visible in the other.

## How the connection works

`rehook web` starts both the local web UI and the WebSocket connection to the server. The agent:

1. Connects outbound to the server.
2. Authenticates with your agent ID and token.
3. Subscribes to the endpoints you have routes for.
4. Waits for delivery instructions from the server.
5. Reconnects automatically if the connection drops.

The token is loaded from the OS keychain.

If you only want the connection loop without the web UI, use `rehook start` instead.

## What happens when you replay an event

1. You trigger a replay from the admin dashboard or the agent web UI.
2. The server checks that the agent is subscribed to the event's endpoint.
3. The server sends a delivery instruction containing the event data and a target ID.
4. The agent looks up the target ID in its local configuration.
5. The agent delivers the webhook to your local application.
6. The agent records the result locally (status, duration, errors).
7. The agent reports the outcome to the server (minimal metadata only, no response bodies).

## Inspect events and replay locally

Open the web UI and go to the Events page. You can inspect any captured event, view its headers and payload, and replay it to any configured target.

Go to the History page to see delivery results: HTTP status, duration, and any errors.

## CLI reference

```
rehook web                          # start the web UI and connection loop
rehook start                        # start the connection loop only
rehook login --server ... --agent-id ... --token ... [--name ...]
rehook target add <id> <url>        # add a local target
rehook target remove <id>
rehook target list
rehook route add <endpoint-id> <target-id>
rehook route remove <endpoint-id>
rehook route list
rehook history                      # show recent deliveries
rehook config                       # print current configuration
rehook logout                       # delete stored credentials
rehook version                      # print version
```

See [Agent Installation](../agent/installation.md) for full command details.
