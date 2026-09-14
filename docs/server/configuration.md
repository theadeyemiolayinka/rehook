# Server Configuration

All configuration is via environment variables. See `.env.example` for the full list.

## Required

| Variable | Description |
|---|---|
| `ADMIN_PASSWORD` | Password for the initial admin user. Set on first boot only. |
| `HOOKRELAY_SESSION_KEY` | 32-byte hex key for session signing. Generate with `openssl rand -hex 32`. |

## Network

| Variable | Default | Description |
|---|---|---|
| `HOOKRELAY_LISTEN_ADDR` | `0.0.0.0:8080` | Address to bind. |
| `HOOKRELAY_PUBLIC_BASE_URL` | `http://localhost:8080` | Public URL for constructing webhook URLs. Set to your HTTPS domain. |
| `HOOKRELAY_TRUSTED_PROXY_HOPS` | `0` | Number of trusted proxy hops for X-Forwarded-For. 0 = use direct peer. |

## Storage

| Variable | Default | Description |
|---|---|---|
| `HOOKRELAY_DATA_DIR` | `./data` | Directory for SQLite database. |
| `HOOKRELAY_DATABASE_URL` | `sqlite:{data_dir}/hookrelay.db` | SQLite connection string. |
| `HOOKRELAY_DASHBOARD_DIR` | `./dashboards/admin/dist` | Directory containing built dashboard assets. |

## Limits

| Variable | Default | Description |
|---|---|---|
| `HOOKRELAY_MAX_WEBHOOK_BODY_SIZE` | `1048576` (1 MB) | Maximum inbound webhook body size in bytes. |
| `HOOKRELAY_MAX_STORED_EVENTS` | `10000` | Maximum stored events (for future retention enforcement). |
| `HOOKRELAY_EVENT_RETENTION_DAYS` | `14` | Event retention period (for future retention enforcement). |

## Sessions

| Variable | Default | Description |
|---|---|---|
| `HOOKRELAY_SESSION_TTL_HOURS` | `720` (30 days) | Session lifetime in hours. |

## Logging

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `hookrelay=info,tower_http=info` | Log filter. |

## Bootstrap administrator

| Variable | Description |
|---|---|
| `ADMIN_USERNAME` | Username for the initial admin (default: `admin`). |
| `ADMIN_PASSWORD` | Password for the initial admin. |

Both must be set or both unset. The admin is created only on first startup. Existing administrators are never overwritten.
