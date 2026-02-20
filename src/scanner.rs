use anyhow::Result;
use tracing::info;

use crate::config::Config;

pub async fn run(cfg: Config) -> Result<()> {
    // Phase 1: copy-trade defensive (observe-only)
    info!(
        dry_run = cfg.dry_run,
        helius_http = %cfg.helius_http_url,
        helius_wss = %cfg.helius_wss_url,
        quicknode_http = cfg.quicknode_http_url.as_deref().unwrap_or(""),
        quicknode_wss = cfg.quicknode_wss_url.as_deref().unwrap_or(""),
        rpc_failover_p95_ms = cfg.rpc_failover_p95_ms,
        alpha_wallets_path = %cfg.alpha_wallets_path,
        "scanner.start"
    );

    // TODO (next):
    // - load alpha_wallets.txt
    // - connect ws (logsSubscribe/accountSubscribe)
    // - emit AlphaWalletTx events
    // - risk/security gating (rugcheck/bitquery)

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        info!(dry_run = cfg.dry_run, "scanner.heartbeat");
    }
}
