use anyhow::Result;
use tracing::info;

use crate::config::Config;

#[derive(Clone)]
pub struct Engine {
    pub cfg: Config,
}

#[derive(Debug, Clone)]
pub struct SwapPlan {
    pub base_mint: String,
    pub quote_mint: String,
    pub in_amount: u64,
    pub slippage_bps: u64,
}

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub signature: String,
}

impl Engine {
    pub fn new(cfg: Config) -> Self {
        Self { cfg }
    }

    /// Skeleton: Jupiter quote -> build swap -> simulateTransaction -> send.
    ///
    /// NOTE: This is intentionally a stub until we wire Solana RPC client + keypair.
    pub async fn execute_swap(&self, plan: SwapPlan) -> Result<SwapResult> {
        info!(?plan, dry_run = self.cfg.dry_run, "engine.execute_swap");

        // TODO:
        // 1) GET /quote (Jupiter v6)
        // 2) POST /swap (get tx)
        // 3) simulateTransaction (Solana RPC) - mandatory
        // 4) dynamic priority fee (computeBudget)
        // 5) sendTransaction
        // 6) confirm + return signature

        if self.cfg.dry_run {
            return Ok(SwapResult { signature: "DRY_RUN".into() });
        }

        anyhow::bail!("engine not yet wired (missing rpc/keypair/jupiter swap build)");
    }

    /// Emergency: close all positions immediately (market exit via Jupiter).
    pub async fn close_all_positions(&self) -> Result<()> {
        info!(dry_run = self.cfg.dry_run, "engine.close_all_positions");
        // TODO: iterate positions, build market exits, simulate+send
        Ok(())
    }
}
