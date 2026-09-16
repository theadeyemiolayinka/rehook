//! Local SQLite database for delivery history and event storage.
//!
//! Stores delivery attempts and captured events locally. Never transmits
//! response bodies or event payloads to the server. Retention is bounded.

use anyhow::{Context, Result};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeliveryRecord {
    pub id: String,
    pub event_id: String,
    pub target_id: String,
    pub attempt_number: i64,
    pub status: String,
    pub http_status: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error_category: Option<String>,
    pub error_message: Option<String>,
    pub response_headers_json: Option<String>,
    pub response_body: Option<Vec<u8>>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StoredEvent {
    pub id: String,
    pub project_id: String,
    pub endpoint_id: String,
    pub request_method: String,
    pub content_type: Option<String>,
    pub received_at: String,
    pub payload_size: i64,
    pub headers_json: String,
    pub body: Option<Vec<u8>>,
}

pub struct LocalDb {
    pool: SqlitePool,
}

impl LocalDb {
    pub async fn open() -> Result<Self> {
        let dir = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("could not determine data directory"))?
            .join("rehook");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("agent.db");
        let url = format!("sqlite:{}", path.display());

        let opts: SqliteConnectOptions = url
            .parse::<SqliteConnectOptions>()
            .context("parsing local db url")?
            .create_if_missing(true)
            .pragma("journal_mode", "WAL")
            .pragma("synchronous", "NORMAL")
            .pragma("busy_timeout", "5000");

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .context("connecting local db")?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS deliveries (
                id TEXT PRIMARY KEY,
                event_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                attempt_number INTEGER NOT NULL,
                status TEXT NOT NULL,
                http_status INTEGER,
                duration_ms INTEGER,
                error_category TEXT,
                error_message TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT
            )",
        )
        .execute(&pool)
        .await
        .context("creating deliveries table")?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_deliveries_event ON deliveries(event_id, started_at DESC)",
        )
        .execute(&pool)
        .await
        .ok();

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                endpoint_id TEXT NOT NULL,
                request_method TEXT NOT NULL,
                content_type TEXT,
                received_at TEXT NOT NULL,
                payload_size INTEGER NOT NULL DEFAULT 0,
                headers_json TEXT NOT NULL,
                body BLOB
            )",
        )
        .execute(&pool)
        .await
        .context("creating events table")?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_events_received ON events(received_at DESC)")
            .execute(&pool)
            .await
            .ok();

        // Add response capture columns to databases created before they
        // existed. SQLite has no IF NOT EXISTS for ALTER TABLE, so check
        // the table info first.
        let cols: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('deliveries')")
                .fetch_all(&pool)
                .await
                .unwrap_or_default();
        if !cols.iter().any(|c| c == "response_headers_json") {
            sqlx::query("ALTER TABLE deliveries ADD COLUMN response_headers_json TEXT")
                .execute(&pool)
                .await
                .ok();
        }
        if !cols.iter().any(|c| c == "response_body") {
            sqlx::query("ALTER TABLE deliveries ADD COLUMN response_body BLOB")
                .execute(&pool)
                .await
                .ok();
        }

        Ok(Self { pool })
    }

    pub async fn record(&self, rec: &DeliveryRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO deliveries
                (id, event_id, target_id, attempt_number, status, http_status,
                 duration_ms, error_category, error_message, response_headers_json,
                 response_body, started_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&rec.id)
        .bind(&rec.event_id)
        .bind(&rec.target_id)
        .bind(rec.attempt_number)
        .bind(&rec.status)
        .bind(rec.http_status)
        .bind(rec.duration_ms)
        .bind(&rec.error_category)
        .bind(&rec.error_message)
        .bind(&rec.response_headers_json)
        .bind(&rec.response_body)
        .bind(&rec.started_at)
        .bind(&rec.completed_at)
        .execute(&self.pool)
        .await
        .context("recording delivery")?;
        Ok(())
    }

    pub async fn recent_deliveries(&self, limit: i64) -> Result<Vec<DeliveryRecord>> {
        let rows: Vec<DeliveryRecord> = sqlx::query_as(
            "SELECT id, event_id, target_id, attempt_number, status, http_status,
                    duration_ms, error_category, error_message, response_headers_json,
                    response_body, started_at, completed_at
             FROM deliveries ORDER BY started_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("fetching deliveries")?;
        Ok(rows)
    }

    pub async fn deliveries_for_event(&self, event_id: &str) -> Result<Vec<DeliveryRecord>> {
        let rows: Vec<DeliveryRecord> = sqlx::query_as(
            "SELECT id, event_id, target_id, attempt_number, status, http_status,
                    duration_ms, error_category, error_message, response_headers_json,
                    response_body, started_at, completed_at
             FROM deliveries WHERE event_id = ? ORDER BY started_at DESC",
        )
        .bind(event_id)
        .fetch_all(&self.pool)
        .await
        .context("fetching deliveries for event")?;
        Ok(rows)
    }

    pub async fn get_delivery(&self, id: &str) -> Result<Option<DeliveryRecord>> {
        let row: Option<DeliveryRecord> = sqlx::query_as(
            "SELECT id, event_id, target_id, attempt_number, status, http_status,
                    duration_ms, error_category, error_message, response_headers_json,
                    response_body, started_at, completed_at
             FROM deliveries WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("fetching delivery")?;
        Ok(row)
    }

    pub async fn delete_event(&self, id: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM events WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("deleting event")?;
        Ok(res.rows_affected() > 0)
    }

    pub async fn store_event(&self, event: &StoredEvent) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO events
                (id, project_id, endpoint_id, request_method, content_type,
                 received_at, payload_size, headers_json, body)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(&event.project_id)
        .bind(&event.endpoint_id)
        .bind(&event.request_method)
        .bind(&event.content_type)
        .bind(&event.received_at)
        .bind(event.payload_size)
        .bind(&event.headers_json)
        .bind(&event.body)
        .execute(&self.pool)
        .await
        .context("storing event")?;
        Ok(())
    }

    pub async fn recent_events(&self, limit: i64) -> Result<Vec<StoredEvent>> {
        let rows: Vec<StoredEvent> = sqlx::query_as(
            "SELECT id, project_id, endpoint_id, request_method, content_type,
                    received_at, payload_size, headers_json, body
             FROM events ORDER BY received_at DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .context("fetching events")?;
        Ok(rows)
    }

    pub async fn get_event(&self, id: &str) -> Result<Option<StoredEvent>> {
        let row: Option<StoredEvent> = sqlx::query_as(
            "SELECT id, project_id, endpoint_id, request_method, content_type,
                    received_at, payload_size, headers_json, body
             FROM events WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("fetching event")?;
        Ok(row)
    }

    pub async fn delivery_count(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deliveries")
            .fetch_one(&self.pool)
            .await
            .context("counting deliveries")?;
        Ok(count)
    }

    pub async fn event_count(&self) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
            .fetch_one(&self.pool)
            .await
            .context("counting events")?;
        Ok(count)
    }

    pub async fn db_size(&self) -> Result<i64> {
        let size: i64 = sqlx::query_scalar(
            "SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size()",
        )
        .fetch_one(&self.pool)
        .await
        .context("checking db size")?;
        Ok(size)
    }

    pub async fn clear_all(&self) -> Result<()> {
        sqlx::query("DELETE FROM deliveries")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM events")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
