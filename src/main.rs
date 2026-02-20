mod config;
mod logger;
mod notifier;
mod risk;
mod state;
mod monitoring;

use anyhow::Result;
use tracing::{error, info};

use crate::config::Config;
use crate::notifier::Notifier;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    monitoring::init_tracing();

    let cfg = Config::from_env()?;
    info!(?cfg, "boot");

    let notifier = Notifier::new(cfg.slack_webhook_url.clone());

    // Daemon loops (scaffold):
    // - market scan every 10-20s (randomized)
    // - position checks every 5s
    // - heartbeat log every 5m

    let hb_path = cfg.heartbeat_log_path.clone();
    let notifier_hb = notifier.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            if let Err(e) = crate::logger::append_line(&hb_path, &crate::logger::heartbeat_line()) {
                error!(error = %e, "heartbeat.log.write_failed");
                let _ = notifier_hb.alert(&format!("[SIE] heartbeat log write failed: {e}")).await;
            }
        }
    });

    // Placeholder forever loop to prove daemon structure.
    // Next milestone will wire scanner/strategies/engine + state.json + risk gates.
    info!("daemon.start");
    notifier.alert("[SIE] daemon started (scaffold)").await.ok();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        info!(dry_run = cfg.dry_run, "daemon.idle");
    }
}
