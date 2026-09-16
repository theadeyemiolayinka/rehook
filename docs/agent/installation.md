# Agent Installation

## Install

### One-line install

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash
```

Detects your platform, downloads the latest release from GitHub, verifies the SHA-256 checksum (and the minisign signature when `minisign` is installed), and installs the `rehook` binary to `/usr/local/bin` or `~/.local/bin`. Works on macOS (Intel and Apple Silicon) and Linux (x86_64 and ARM64).

### Install a specific version

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash -s -- --version v1.0.0
```

### Install to a custom directory

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash -s -- --dir /opt/bin
```

### Update

Re-run the install command. It overwrites the existing binary.

```bash
rehook version                    # check current version
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-agent.sh | bash
```

### Build from source

```bash
git clone https://github.com/theadeyemiolayinka/rehook.git
cd rehook
cargo build --release -p rehook-agent
```

The binary is at `target/release/rehook`.

Or use the build script:

```bash
./scripts/build-agent.sh
```

## CLI reference

### web

Starts the local web UI. This is the easiest way to configure the agent.

```bash
rehook web
rehook web --port 9000
```

The web UI lets you log in, configure targets and routes, inspect events, replay events, and view delivery history. See [Web UI](web-ui.md) for details.

### login

```bash
rehook login \
  --server https://hooks.example.com \
  --agent-id 550e8400-e29b-41d4-a716-446655440000 \
  --token re_your_token_here \
  --name my-laptop
```

- `--server`: Rehook server URL
- `--agent-id`: Agent UUID from the admin dashboard
- `--token`: One-time token from the admin dashboard
- `--name`: Optional friendly name for this machine

### target add

```bash
rehook target add myapp http://localhost:8000/webhook
```

- First argument: a target ID (any string you choose)
- Second argument: the local URL (must be http or https)

### target remove

```bash
rehook target remove myapp
```

### target list

Lists all configured targets.

### route add

```bash
rehook route add 550e8400-e29b-41d4-a716-446655440000 myapp
```

- First argument: project UUID from the admin dashboard
- Second argument: target ID (must already exist)

### route remove

```bash
rehook route remove 550e8400-e29b-41d4-a716-446655440000
```

### route list

Lists all configured routes.

### config

Prints the current configuration. Does not print the token.

### start

Starts the agent. Connects to the server via WebSocket, subscribes to configured projects, and waits for delivery instructions. Reconnects automatically.

### history

```bash
rehook history
rehook history --limit 50
```

### logout

Deletes stored credentials and clears the configuration.

### version

Prints the agent version and protocol version.

## Allowed target schemes

The agent only allows `http://` and `https://` targets. See [Targets](targets.md) for details.
