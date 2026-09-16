# Server Configuration

All configuration is via environment variables or command-line flags. Flags take precedence over the matching environment variable. See `.env.example` for the full list and `rehook-server --help` for the flags.

| Flag | Environment variable |
|---|---|
| `--listen-addr` | `REHOOK_LISTEN_ADDR` |
| `--public-base-url` | `REHOOK_PUBLIC_BASE_URL` |
| `--data-dir` | `REHOOK_DATA_DIR` |
| `--dashboard-dir` | `REHOOK_DASHBOARD_DIR` |

## Required

| Variable | Description |
|---|---|
| `ADMIN_PASSWORD` | Password for the initial admin user. Set on first boot only. |
| `REHOOK_SESSION_KEY` | 32-byte hex key for session signing. Generate with `openssl rand -hex 32`. |

## Network

| Variable | Default | Description |
|---|---|---|
| `REHOOK_LISTEN_ADDR` | `0.0.0.0:8080` | Address to bind. |
| `REHOOK_PUBLIC_BASE_URL` | `http://localhost:8080` | Public URL for constructing webhook URLs. Set to your HTTPS domain. |
| `REHOOK_TRUSTED_PROXY_HOPS` | `0` | Number of trusted proxy hops for X-Forwarded-For. 0 = use direct peer. |

## Storage

| Variable | Default | Description |
|---|---|---|
| `REHOOK_DATA_DIR` | `./data` | Directory for SQLite database. |
| `REHOOK_DATABASE_URL` | `sqlite:{data_dir}/rehook.db` | SQLite connection string. |
| `REHOOK_DASHBOARD_DIR` | unset (embedded) | Optional. Serve dashboard assets from a directory instead of the assets embedded in the binary. |

## Limits

| Variable | Default | Description |
|---|---|---|
| `REHOOK_MAX_WEBHOOK_BODY_SIZE` | `1048576` (1 MB) | Maximum inbound webhook body size in bytes. |
| `REHOOK_MAX_STORED_EVENTS` | `10000` | Maximum stored events (for future retention enforcement). |
| `REHOOK_EVENT_RETENTION_DAYS` | `14` | Event retention period (for future retention enforcement). |

## Sessions

| Variable | Default | Description |
|---|---|---|
| `REHOOK_SESSION_TTL_HOURS` | `720` (30 days) | Session lifetime in hours. |

## Logging

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `rehook=info,tower_http=info` | Log filter. |

## Bootstrap administrator

| Variable | Description |
|---|---|
| `ADMIN_USERNAME` | Username for the initial admin (default: `admin`). |
| `ADMIN_PASSWORD` | Password for the initial admin. |

Both must be set or both unset. The admin is created only on first startup. Existing administrators are never overwritten.
