# Docker Deployment

## Quick start

```bash
cp .env.example .env
# Edit .env:
#   ADMIN_PASSWORD - set a strong password for the admin user
#   HOOKRELAY_SESSION_KEY - generate with: openssl rand -hex 32
#   HOOKRELAY_PUBLIC_BASE_URL - set to your domain (e.g. https://hooks.example.com)
docker compose up -d
```

## What the Compose file does

- Builds the server from source (multi-stage Dockerfile).
- Persists SQLite data in a named volume (`hookrelay-data`).
- Exposes port 8080.
- Runs as a non-root user.
- Healthcheck via `GET /healthz`.
- Restart policy: `unless-stopped`.

## TLS termination

The server listens on plain HTTP. Use a reverse proxy (Caddy, Traefik, Cloudflare, Coolify, Dokploy) for TLS termination. Set `HOOKRELAY_PUBLIC_BASE_URL` to your HTTPS URL so the dashboard constructs correct webhook URLs.

## Environment variables

See the `.env.example` file in the repository root for all options. Required variables:

- `ADMIN_PASSWORD`: password for the initial admin user (set on first boot only).
- `HOOKRELAY_SESSION_KEY`: 32-byte hex key for session signing.

## Updating

```bash
git pull
docker compose build
docker compose up -d
```

The SQLite database is preserved across restarts via the volume.

## Extracting the agent binary

The Docker image includes the agent binary. To extract it:

```bash
docker compose exec server cat /usr/local/bin/hookrelay > hookrelay
chmod +x hookrelay
```

Or build it separately:

```bash
cargo build --release -p hookrelay-agent
# Binary at target/release/hookrelay
```
