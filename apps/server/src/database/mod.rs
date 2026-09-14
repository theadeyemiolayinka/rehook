//! Database initialization, migrations, and bootstrap.

pub mod models;

use anyhow::{Context, Result};
use sqlx::SqlitePool;

use crate::auth::password::hash_password;
use crate::config::Config;

/// Run embedded migrations.
pub async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| anyhow::anyhow!("migration failed: {e}"))?;
    Ok(())
}

/// Create the initial administrator from `ADMIN_USERNAME` / `ADMIN_PASSWORD`
/// only if no administrator exists yet. Never overwrites an existing admin.
/// Never logs credentials.
pub async fn bootstrap_admin(pool: &SqlitePool, config: &Config) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE is_admin = 1")
        .fetch_one(pool)
        .await
        .context("checking for existing admin")?;

    if count > 0 {
        tracing::debug!("administrator already exists; skipping bootstrap");
        return Ok(());
    }

    let (Some(username), Some(password)) = (&config.admin_username, &config.admin_password) else {
        tracing::warn!(
            "no administrator exists and ADMIN_USERNAME/ADMIN_PASSWORD are not set; \
             the dashboard will be inaccessible until bootstrap is performed"
        );
        return Ok(());
    };

    let password_hash = hash_password(password).context("hashing bootstrap password")?;
    sqlx::query("INSERT INTO users (id, username, password_hash, is_admin) VALUES (?, ?, ?, 1)")
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(username)
        .bind(password_hash)
        .execute(pool)
        .await
        .context("inserting bootstrap admin")?;

    tracing::info!(username = %username, "bootstrap administrator created");
    Ok(())
}
