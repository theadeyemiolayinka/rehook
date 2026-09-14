# Getting Started: Server

The HookRelay server receives webhooks, stores them, and serves the admin dashboard. It runs as a Docker container in production.

## Quick start with Docker Compose

### 1. Get the compose file

If you have the repository cloned:

```bash
git clone https://github.com/theadeyemiolayinka/hookrelay.git
cd hookrelay
```

Or download just the compose file and env example:

```bash
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/compose.yml -o compose.yml
curl -fsSL https://raw.githubusercontent.com/theadeyemiolayinka/hookrelay/main/.env.example -o .env
```

### 2. Configure environment

Edit the `.env` file. At minimum, set these two values:

```bash
# A strong password for the admin user (set on first boot only)
ADMIN_PASSWORD=change-me-to-a-strong-password

# Session signing key. Generate one with: openssl rand -hex 32
HOOKRELAY_SESSION_KEY=paste-your-32-byte-hex-key-here
```

Other useful settings:

```bash
# Your public HTTPS URL. Set this to your domain.
HOOKRELAY_PUBLIC_BASE_URL=https://hooks.example.com

# Number of trusted proxy hops (e.g. 1 if behind a single reverse proxy)
HOOKRELAY_TRUSTED_PROXY_HOPS=1
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

Set `HOOKRELAY_PUBLIC_BASE_URL` to your HTTPS URL so webhook URLs and session cookies are configured correctly.

### With Coolify

1. Create a new resource in Coolify and select Docker Compose.
2. Point Coolify at the repository or paste the compose file.
3. Set the environment variables in the Coolify UI.
4. Deploy.

See [Coolify deployment](../deployment/coolify.md) for details.

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
