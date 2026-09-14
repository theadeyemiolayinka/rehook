# Getting Started: Agent

The HookRelay agent runs on your local machine. It connects to your HookRelay server and delivers webhooks to your local applications.

You can set up the agent entirely from the web UI, or use the terminal. Both paths work together.

## Install the agent

### One-line install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/scripts/install-agent.sh | bash
```

This detects your OS and architecture, downloads the latest release from GitHub, and installs the `hookrelay` binary. It works on:

- macOS (Intel and Apple Silicon)
- Linux (x86_64 and ARM64)

To install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/scripts/install-agent.sh | bash -s -- --version v0.1.0
```

### Update the agent

Re-run the install command. It overwrites the existing binary:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/scripts/install-agent.sh | bash
```

Check your current version first:

```bash
hookrelay version
```

### Build from source

If you have Rust installed:

```bash
git clone https://github.com/theadeyemiolayinka/hookrelay.git
cd hookrelay
cargo build --release -p hookrelay-agent
```

The binary is at `target/release/hookrelay`.

## Set up the agent

### Option A: Web UI (recommended)

Start the web UI:

```bash
hookrelay web
```

Open `http://localhost:8787` in your browser. If the agent is not yet configured, you will see a login page.

Enter the details from your server admin dashboard (Agents page):

- **Server URL**: your HookRelay server URL
- **Agent ID**: the UUID shown when you created the agent
- **Agent Token**: the one-time token shown when you created the agent
- **Agent Name**: a friendly name for this machine (optional)

Click Connect. The agent validates the server and stores your credentials.

Then configure your local targets and routes from the same web UI:

1. Go to the **Targets** page.
2. Add a target (e.g. ID `myapp`, URL `http://localhost:8000/webhook`).
3. Add a route: select your project and the target you just created.

That is it. The agent is now configured. Start the connection in a terminal:

```bash
hookrelay start
```

The web UI shows your connection status, subscribed projects, and lets you inspect events and trigger replays.

### Option B: Terminal

If you prefer the terminal, here are the equivalent commands:

```bash
# Log in (get the agent ID and token from the admin dashboard)
hookrelay login \
  --server https://hooks.example.com \
  --agent-id 550e8400-e29b-41d4-a716-446655440000 \
  --token hr_your_token_here \
  --name my-laptop

# Add a local target
hookrelay target add myapp http://localhost:8000/webhook

# Map a project to the target
hookrelay route add 550e8400-e29b-41d4-a716-446655440000 myapp

# Start the connection
hookrelay start
```

The web UI and terminal share the same configuration. Changes made in one are visible in the other.

## Start the agent connection

The WebSocket connection to the server is managed by the `start` command:

```bash
hookrelay start
```

The agent connects to the server, subscribes to your configured projects, and waits for delivery instructions. It reconnects automatically if the connection drops.

Keep this running in a terminal while you work. The web UI (`hookrelay web`) is for configuration and inspection. The `start` command is for the live connection.

## What happens when you replay an event

1. You trigger a replay from the admin dashboard or the agent web UI.
2. The server sends a delivery instruction to the agent.
3. The agent looks up the target in its local configuration.
4. The agent delivers the webhook to your local application.
5. The agent records the result locally.

## Inspect events and replay locally

Open the web UI and go to the Events page. You can inspect any captured event, view its headers and payload, and replay it to any configured target.

Go to the History page to see delivery results: HTTP status, duration, and any errors.

## CLI reference

```
hookrelay web                          # start the local web UI
hookrelay login --server ... --agent-id ... --token ...
hookrelay target add <id> <url>        # add a local target
hookrelay target remove <id>
hookrelay target list
hookrelay route add <project-id> <target-id>
hookrelay route remove <project-id>
hookrelay route list
hookrelay start                        # start the server connection
hookrelay history                      # show recent deliveries
hookrelay config                       # print current configuration
hookrelay logout                       # delete stored credentials
hookrelay version                      # print version
```

See [Agent Installation](../agent/installation.md) for full command details.
