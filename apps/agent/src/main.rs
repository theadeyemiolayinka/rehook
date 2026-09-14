//! HookRelay local agent.
//!
//! Connects outbound to a HookRelay server, subscribes to projects, and
//! delivers captured webhooks to explicitly configured local targets. The
//! server never sends a URL; the agent resolves a target identifier to a
//! locally configured, allowlisted destination and validates it before any
//! request. See SECURITY.md.

mod cli;
mod config;
mod connection;
mod credentials;
mod db;
mod delivery;
mod targets;
mod web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    cli::run().await
}
