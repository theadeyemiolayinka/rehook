# Coolify Deployment

HookRelay deploys on Coolify using standard Docker Compose.

## Steps

1. Create a new resource in Coolify and select "Docker Compose".
2. Point Coolify at the repository root. It will use `compose.yml`.
3. Set the required environment variables in the Coolify UI:
   - `ADMIN_PASSWORD`: a strong password for the admin user.
   - `HOOKRELAY_SESSION_KEY`: generate with `openssl rand -hex 32`.
   - `HOOKRELAY_PUBLIC_BASE_URL`: your Coolify-assigned HTTPS URL (e.g. `https://hookrelay.yourdomain.com`).
4. Deploy.

Coolify handles TLS termination. The server listens on port 8080 inside the container.

## Persistent storage

Coolify preserves the named volume `hookrelay-data` across deploys. SQLite data survives container restarts and updates.

## Health check

The Compose file defines a healthcheck via `GET /healthz`. Coolify uses this to determine container health.
