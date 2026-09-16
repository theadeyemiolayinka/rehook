//! CLI command parsing and dispatch.

use std::sync::Arc;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use rehook_protocol::PROTOCOL_VERSION;
use uuid::Uuid;

use crate::config::AgentConfig;
use crate::connection;
use crate::credentials;
use crate::db::LocalDb;
use crate::state::ConnectionState;

#[derive(Parser)]
#[command(name = "rehook", version, about = "Rehook local agent")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version and protocol information.
    Version,
    /// Authenticate the agent with a server and store the token.
    Login {
        /// Server base URL, e.g. http://localhost:8080
        #[arg(long)]
        server: String,
        /// Agent ID (from the admin dashboard Agents page).
        #[arg(long)]
        agent_id: String,
        /// Agent token (format re_...). Shown once when the agent is created.
        #[arg(long)]
        token: String,
        /// Friendly name for this agent.
        #[arg(long, default_value = "default")]
        name: String,
    },
    /// Configure local delivery targets.
    Target {
        #[command(subcommand)]
        action: TargetAction,
    },
    /// Configure a project to route to a target.
    Route {
        #[command(subcommand)]
        action: RouteAction,
    },
    /// Show current configuration.
    Config,
    /// Start the agent connection loop only (no web UI).
    Start,
    /// Show recent local delivery history.
    History {
        #[arg(long, default_value = "20")]
        limit: i64,
    },
    /// Start the local agent web UI and connection loop. Binds to localhost only.
    Web {
        /// Port to bind the web UI to.
        #[arg(long, default_value = "8787")]
        port: u16,
        /// Directory containing the built agent dashboard assets.
        #[arg(long)]
        dashboard_dir: Option<String>,
    },
    /// Sign out and delete stored credentials.
    Logout,
}

#[derive(Subcommand)]
enum TargetAction {
    /// Add or update a target.
    Add {
        /// Target identifier (referenced by routes).
        id: String,
        /// Local URL to deliver to.
        url: String,
    },
    /// Remove a target.
    Remove { id: String },
    /// List targets.
    List,
}

#[derive(Subcommand)]
enum RouteAction {
    /// Route an endpoint to a target. Use the endpoint ID from the admin dashboard.
    Add {
        endpoint_id: String,
        target_id: String,
    },
    /// Remove a route.
    Remove { endpoint_id: String },
    /// List routes.
    List,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!("rehook agent {}", env!("CARGO_PKG_VERSION"));
            println!("protocol version {PROTOCOL_VERSION}");
            Ok(())
        }
        Commands::Login {
            server,
            agent_id,
            token,
            name,
        } => login(&server, &agent_id, &token, &name).await,
        Commands::Target { action } => target(action),
        Commands::Route { action } => route(action),
        Commands::Config => show_config(),
        Commands::Start => start().await,
        Commands::History { limit } => history(limit).await,
        Commands::Web {
            port,
            dashboard_dir,
        } => web(port, dashboard_dir).await,
        Commands::Logout => {
            credentials::delete_token()?;
            let mut config = AgentConfig::load()?;
            config.agent_id = None;
            config.server_url = None;
            config.save()?;
            println!("signed out");
            Ok(())
        }
    }
}

async fn login(server: &str, agent_id: &str, token: &str, name: &str) -> Result<()> {
    // Validate the server is reachable.
    let health = format!("{}/healthz", server.trim_end_matches('/'));
    let resp = reqwest::Client::new().get(&health).send().await;
    if let Err(e) = resp {
        return Err(anyhow!("could not reach server at {server}: {e}"));
    }

    let agent_uuid = Uuid::parse_str(agent_id).map_err(|e| anyhow!("invalid agent_id: {e}"))?;

    // Store the token securely.
    credentials::store_token(agent_uuid, token)?;

    let mut config = AgentConfig::load()?;
    config.server_url = Some(server.to_string());
    config.agent_id = Some(agent_uuid);
    config.agent_name = Some(name.to_string());
    config.save()?;

    println!("logged in to {server} as agent {name}");
    println!("agent id: {agent_uuid}");
    println!();
    println!("next steps:");
    println!("  1. Add a local target:  rehook target add myapp http://localhost:8000/webhook");
    println!("  2. Route an endpoint:  rehook route add <endpoint-id> myapp");
    println!("  3. Start the agent:     rehook web");
    println!();
    println!("or open the web UI with 'rehook web' to configure targets and routes visually.");
    Ok(())
}

fn target(action: TargetAction) -> Result<()> {
    let mut config = AgentConfig::load()?;
    match action {
        TargetAction::Add { id, url } => {
            crate::targets::validate_url(&url)?;
            config.targets.insert(id.clone(), url.clone());
            config.save()?;
            println!("target {id} -> {url}");
        }
        TargetAction::Remove { id } => {
            config.targets.remove(&id);
            config.save()?;
            println!("removed target {id}");
        }
        TargetAction::List => {
            if config.targets.is_empty() {
                println!("no targets configured");
            } else {
                for (id, url) in &config.targets {
                    println!("{id}\t{url}");
                }
            }
        }
    }
    Ok(())
}

fn route(action: RouteAction) -> Result<()> {
    let mut config = AgentConfig::load()?;
    match action {
        RouteAction::Add {
            endpoint_id,
            target_id,
        } => {
            if !config.targets.contains_key(&target_id) {
                return Err(anyhow!("unknown target {target_id}; add it first"));
            }
            config
                .endpoint_targets
                .insert(endpoint_id.clone(), target_id.clone());
            config.save()?;
            println!("route {endpoint_id} -> {target_id}");
        }
        RouteAction::Remove { endpoint_id } => {
            config.endpoint_targets.remove(&endpoint_id);
            config.save()?;
            println!("removed route for {endpoint_id}");
        }
        RouteAction::List => {
            if config.endpoint_targets.is_empty() {
                println!("no routes configured");
                println!();
                println!("add a route with: rehook route add <endpoint-id> <target-id>");
            } else {
                println!("endpoint_id\ttarget_id");
                for (eid, tid) in &config.endpoint_targets {
                    println!("{eid}\t{tid}");
                }
            }
        }
    }
    Ok(())
}

fn show_config() -> Result<()> {
    let config = AgentConfig::load()?;
    println!("server: {}", config.server_url.unwrap_or_default());
    println!("agent id: {}", config.agent_id.unwrap_or_default());
    println!("agent name: {}", config.agent_name.unwrap_or_default());
    println!();
    println!("targets:");
    for (id, url) in &config.targets {
        println!("  {id} -> {url}");
    }
    println!();
    println!("routes (endpoint -> target):");
    for (eid, tid) in &config.endpoint_targets {
        println!("  {eid} -> {tid}");
    }
    Ok(())
}

async fn start() -> Result<()> {
    let config = AgentConfig::load()?;
    if config.server_url.is_none() {
        return Err(anyhow!("not logged in; run `rehook login` first"));
    }
    if config.agent_id.is_none() {
        return Err(anyhow!("not logged in; run `rehook login` first"));
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rehook=info".into()),
        )
        .init();

    let db = Arc::new(LocalDb::open().await?);
    tracing::info!("local db opened");

    let conn_state = ConnectionState::new();
    connection::run(db, conn_state).await
}

async fn history(limit: i64) -> Result<()> {
    let db = LocalDb::open().await?;
    let rows = db.recent_deliveries(limit).await?;
    if rows.is_empty() {
        println!("no deliveries recorded");
        return Ok(());
    }
    println!(
        "{:<36} {:<8} {:<6} {:<8} {:<10}",
        "id", "status", "http", "ms", "target"
    );
    for r in rows {
        println!(
            "{:<36} {:<8} {:<6} {:<8} {}",
            r.id,
            r.status,
            r.http_status.map(|s| s.to_string()).unwrap_or_default(),
            r.duration_ms.unwrap_or(0),
            r.target_id,
        );
    }
    Ok(())
}

async fn web(port: u16, dashboard_dir: Option<String>) -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rehook=info".into()),
        )
        .init();

    let dir = dashboard_dir.map(std::path::PathBuf::from);
    crate::web::run(port, dir).await
}
