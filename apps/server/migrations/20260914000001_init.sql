-- Users. Initially a single administrator. Schema supports multiple users
-- later without changes.
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_admin INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_login_at TEXT
);

CREATE INDEX idx_users_username ON users(username);

-- Dashboard sessions. HTTP-only secure cookies reference these.
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- A random token; the cookie carries a hash of it.
    token_hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_expires ON sessions(expires_at);

-- Webhook projects. A logical workspace for one or more endpoints.
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_projects_slug ON projects(slug);

-- Inbound endpoints. A project may have many. The public_identifier is
-- unguessable and is the only thing exposed in the inbound URL.
CREATE TABLE endpoints (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    public_identifier TEXT NOT NULL UNIQUE,
    enabled INTEGER NOT NULL DEFAULT 1,
    -- Optional provider label for future signature verification.
    provider TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_endpoints_public_identifier ON endpoints(public_identifier);
CREATE INDEX idx_endpoints_project ON endpoints(project_id);

-- Captured webhook events. The original event is immutable.
CREATE TABLE events (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    endpoint_id TEXT NOT NULL REFERENCES endpoints(id) ON DELETE CASCADE,
    request_method TEXT NOT NULL,
    content_type TEXT,
    remote_address TEXT,
    received_at TEXT NOT NULL DEFAULT (datetime('now')),
    payload_size INTEGER NOT NULL DEFAULT 0,
    -- JSON object of request headers. Sensitive headers may be masked on
    -- read by the API layer.
    headers_json TEXT NOT NULL,
    -- Raw body bytes (bounded by MAX_WEBHOOK_BODY_SIZE).
    body BLOB,
    delivery_state TEXT NOT NULL DEFAULT 'pending'
);

CREATE INDEX idx_events_project ON events(project_id, received_at DESC);
CREATE INDEX idx_events_endpoint ON events(endpoint_id, received_at DESC);
CREATE INDEX idx_events_received ON events(received_at DESC);

-- Agents. Authenticated by a revocable token, separate from dashboard
-- sessions.
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    -- Hash of the agent token. The raw token is shown once at creation.
    token_hash TEXT NOT NULL UNIQUE,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT
);

CREATE INDEX idx_agents_token_hash ON agents(token_hash);

-- Agent subscriptions to endpoints. Delivery is only authorized for
-- subscribed agents. Subscriptions are at the endpoint level so different
-- endpoints in the same project can route to different local targets.
CREATE TABLE agent_subscriptions (
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    endpoint_id TEXT NOT NULL REFERENCES endpoints(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, endpoint_id)
);

CREATE INDEX idx_agent_subscriptions_endpoint ON agent_subscriptions(endpoint_id);

-- Delivery attempts. Each replay creates a new row; the original event is
-- never mutated.
CREATE TABLE deliveries (
    id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    -- The target identifier the agent resolves locally.
    target_id TEXT NOT NULL,
    attempt_number INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    -- Minimal metadata reported by the agent.
    http_status INTEGER,
    duration_ms INTEGER,
    error_category TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX idx_deliveries_event ON deliveries(event_id, attempt_number);
CREATE INDEX idx_deliveries_agent ON deliveries(agent_id, started_at DESC);

-- Lightweight audit log for important administrative actions.
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    target_type TEXT,
    target_id TEXT,
    detail TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_audit_log_created ON audit_log(created_at DESC);
