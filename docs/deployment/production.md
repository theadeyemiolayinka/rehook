# Production Deployment

## Reverse proxy

Use a reverse proxy for TLS termination. The server listens on plain HTTP on port 8080.

### Caddy

```
hooks.example.com {
    reverse_proxy localhost:8080
}
```

### Traefik

Configure an entrypoint for HTTPS and route to the container on port 8080.

### Nginx

```
server {
    listen 443 ssl http2;
    server_name hooks.example.com;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Trusted proxy hops

If you use a reverse proxy, set `REHOOK_TRUSTED_PROXY_HOPS` to the number of proxy hops between the internet and the server. This controls whether `X-Forwarded-For` is trusted for client IP resolution.

## Session key

Always set `REHOOK_SESSION_KEY` to a stable 32-byte hex value in production. If unset, a random key is generated on each restart and all sessions are invalidated.

## Public base URL

Set `REHOOK_PUBLIC_BASE_URL` to your HTTPS URL. This ensures:
- Webhook URLs displayed in the dashboard are correct.
- Session cookies are marked `Secure`.
- The dashboard is accessible at the expected URL.

## Backups

The SQLite database is the only persistent state. Back up the file at `{REHOOK_DATA_DIR}/rehook.db` regularly. SQLite WAL mode means you should also back up the `-wal` file, or use `sqlite3 rehook.db ".backup '/path/to/backup.db'"` for a consistent snapshot.
