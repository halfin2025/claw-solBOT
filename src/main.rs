mod config;
mod domain;
mod scanner;
mod security;
mod execution;
mod state;
mod monitoring;

use anyhow::Result;
use tracing::{info};

#[tokio::main]
async fn main() -> Result<()> {
    // Load local .env if present (no-op in prod/systemd envs)
    let _ = dotenvy::dotenv();

    monitoring::init_tracing();

    let cfg = config::Config::from_env()?;
    info!(?cfg, "boot");

    // Phase 1: start scanner (new pools) -> validate -> execution.
    // Stub for now.
    scanner::run(cfg).await?;

    Ok(())
}
