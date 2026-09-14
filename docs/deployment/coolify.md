# Coolify/Dokploy Deployment

HookRelay deploys on Coolify or Dokploy using standard Docker Compose. Both platforms support Docker Compose deployments with persistent volumes, TLS termination, and automatic health checks.

## Steps

1. Create a new resource in your platform and select "Docker Compose".
2. Point the platform at the repository root. It will use `compose.yml`.
3. Set the required environment variables in the platform UI:
   - `ADMIN_PASSWORD`: a strong password for the admin user.
   - `HOOKRELAY_SESSION_KEY`: generate with `openssl rand -hex 32`.
   - `HOOKRELAY_PUBLIC_BASE_URL`: your assigned HTTPS URL (e.g. `https://hookrelay.yourdomain.com`).
4. Deploy.

Both Coolify and Dokploy handle TLS termination. The server listens on port 8080 inside the container.

## Persistent storage

The named volume `hookrelay-data` is preserved across deploys. SQLite data survives container restarts and updates.

## Health check

The Compose file defines a healthcheck via `GET /healthz`. Both platforms use this to determine container health.

## Updating

Pull the latest changes and redeploy. Your data is preserved in the persistent volume.
