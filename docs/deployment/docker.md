# Docker Deployment

## Quick start

```bash
cp .env.example .env
# Edit .env:
#   ADMIN_PASSWORD - set a strong password for the admin user
#   REHOOK_SESSION_KEY - generate with: openssl rand -hex 32
#   REHOOK_PUBLIC_BASE_URL - set to your domain (e.g. https://hooks.example.com)
docker compose up -d
```

## What the Compose file does

- Builds the server from source (multi-stage Dockerfile).
- Persists SQLite data in a named volume (`rehook-data`).
- Exposes port 8080.
- Runs as a non-root user.
- Healthcheck via `GET /healthz`.
- Restart policy: `unless-stopped`.

## TLS termination

The server listens on plain HTTP. Use a reverse proxy (Caddy, Traefik, Cloudflare, Coolify, Dokploy) for TLS termination. Set `REHOOK_PUBLIC_BASE_URL` to your HTTPS URL so the dashboard constructs correct webhook URLs.

When configuring the reverse proxy, ensure all non-API, non-webhook paths are forwarded to the server. The server serves the dashboard as a single-page application: routes like `/settings` or `/events` do not correspond to static files and fall back to `index.html`. A reverse proxy that only forwards specific paths will break client-side routing.

Example Caddyfile:

```
hooks.example.com {
    reverse_proxy localhost:8080
}
```

Example nginx configuration:

```nginx
server {
    listen 443 ssl;
    server_name hooks.example.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

Do not use `try_files` with nginx. The server handles SPA fallback internally.

## Environment variables

See the `.env.example` file in the repository root for all options. Required variables:

- `ADMIN_PASSWORD`: password for the initial admin user (set on first boot only).
- `REHOOK_SESSION_KEY`: 32-byte hex key for session signing.

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
docker compose exec server cat /usr/local/bin/rehook > rehook
chmod +x rehook
```

Or build it separately:

```bash
cargo build --release -p rehook-agent
# Binary at target/release/rehook
```
