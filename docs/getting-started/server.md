# Getting Started: Server

The Rehook server receives webhooks, stores them, and serves the admin dashboard. It runs as a single binary or a Docker container.

## Option A: Single binary

The `rehook-server` binary is fully self-contained: the admin dashboard is embedded, and SQLite is the only storage. There are no runtime dependencies.

### 1. Install

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-server.sh | bash
```

This verifies the release checksum, installs `rehook-server` to `/usr/local/bin` (or `~/.local/bin`), and creates a data directory (`/var/lib/rehook` as root, or `~/.local/share/rehook-server`).

To install and enable a systemd service in one step, run as root:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/scripts/install-server.sh | sudo bash -s -- --systemd
```

This writes `/etc/rehook/env` with a generated `REHOOK_SESSION_KEY` and a hardened systemd unit, then reloads systemd. Edit `/etc/rehook/env` to set your public URL and admin credentials, then:

```bash
systemctl enable --now rehook-server
```

### 2. Configure

Every setting has a flag and a matching environment variable. Flags take precedence.

```bash
rehook-server \
  --listen-addr 0.0.0.0:8080 \
  --data-dir /var/lib/rehook \
  --public-base-url https://hooks.example.com
```

Equivalent environment variables:

```bash
export REHOOK_LISTEN_ADDR=0.0.0.0:8080
export REHOOK_DATA_DIR=/var/lib/rehook
export REHOOK_PUBLIC_BASE_URL=https://hooks.example.com
export REHOOK_SESSION_KEY=$(openssl rand -hex 32)
export ADMIN_USERNAME=admin
export ADMIN_PASSWORD=change-me-to-a-strong-password
rehook-server
```

`REHOOK_PUBLIC_BASE_URL` is the URL webhook providers use to reach this server. Set it to your public HTTPS domain; it is used to build the inbound URLs shown in the dashboard and to decide whether session cookies get the `Secure` flag.

The admin credentials (`ADMIN_USERNAME`, `ADMIN_PASSWORD`) are read on first boot only and create the initial admin user. They are never read again after the user exists.

Run `rehook-server --help` to see every option.

## Option B: Docker Compose

### 1. Get the compose file

If you have the repository cloned:

```bash
git clone https://github.com/theadeyemiolayinka/rehook.git
cd rehook
```

Or download just the compose file and env example:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/compose.yml -o compose.yml
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/rehook/main/.env.example -o .env
```

### 2. Configure environment

Edit the `.env` file. At minimum, set these two values:

```bash
# A strong password for the admin user (set on first boot only)
ADMIN_PASSWORD=change-me-to-a-strong-password

# Session signing key. Generate one with: openssl rand -hex 32
REHOOK_SESSION_KEY=paste-your-32-byte-hex-key-here
```

Other useful settings:

```bash
# Your public HTTPS URL. Set this to your domain.
REHOOK_PUBLIC_BASE_URL=https://hooks.example.com

# Number of trusted proxy hops (e.g. 1 if behind a single reverse proxy)
REHOOK_TRUSTED_PROXY_HOPS=1
```

### 3. Start the server

```bash
docker compose up -d
```

The server starts and listens on port 8080. It creates the admin user on first boot using the credentials from your `.env` file.

### 4. Open the admin dashboard

If running locally: open `http://localhost:8080` in your browser.

If deployed to a server: open your configured HTTPS URL.

Sign in with the admin username and password from your `.env` file.

### 5. Create your first project and endpoint

1. Go to the Projects page and create a project.
2. Open the project and create an endpoint.
3. Copy the inbound URL. It looks like `https://hooks.example.com/i/abc123XYZ...`
4. Point your webhook provider at that URL.
5. Send a test webhook and check the Events page.

### 6. Create an agent

1. Go to the Agents page and create a new agent.
2. Select which projects it should have access to.
3. Copy the agent ID and the one-time token. You will need these to set up the agent on your local machine.

See [Agent Setup](../getting-started/agent.md) for the next steps.

## Deploying to a VPS

### With a reverse proxy (recommended)

Use Caddy, Traefik, or Nginx as a reverse proxy for TLS termination. The server listens on plain HTTP inside the container.

Caddy example:

```
hooks.example.com {
    reverse_proxy localhost:8080
}
```

Set `REHOOK_PUBLIC_BASE_URL` to your HTTPS URL so webhook URLs and session cookies are configured correctly.

### With Coolify or Dokploy

1. Create a new resource in Coolify or Dokploy and select Docker Compose.
2. Point the platform at the repository or paste the compose file.
3. Set the environment variables in the platform UI.
4. Deploy.

See [Coolify/Dokploy deployment](../deployment/coolify.md) for details.

## Updating the server

```bash
git pull
docker compose build
docker compose up -d
```

Your data is preserved in the Docker volume across restarts and updates.

## What you get

After setup, the server provides:

- A public webhook endpoint at `/i/{identifier}` for each endpoint you create
- An admin dashboard for managing projects, endpoints, events, and agents
- A WebSocket gateway for agents to connect and receive delivery instructions
- SQLite storage for all captured events

The server does not initiate connections to your local machine. Agents connect outbound. See [Architecture](../architecture/overview.md) for the full model.
