use anyhow::{anyhow, Result};
use base64::Engine as _;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcSendTransactionConfig, RpcSimulateTransactionConfig};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{read_keypair_file, Keypair, Signer};
use solana_sdk::transaction::VersionedTransaction;
use tracing::{info, warn};

use crate::config::Config;
use crate::jupiter::{ensure_slippage_bounds, JupiterClient, QuoteRequest, SwapRequest};

#[derive(Clone)]
pub struct Engine {
    pub cfg: Config,
    rpc: RpcClient,
    jup: JupiterClient,
}

#[derive(Debug, Clone)]
pub struct SwapPlan {
    pub input_mint: String,
    pub output_mint: String,
    /// base units
    pub in_amount: u64,
    pub slippage_bps: u64,
}

#[derive(Debug, Clone)]
pub struct SwapResult {
    pub signature: String,
}

impl Engine {
    pub fn new(cfg: Config) -> Self {
        let rpc = RpcClient::new_with_commitment(cfg.helius_http_url.clone(), CommitmentConfig::confirmed());
        let jup = JupiterClient::new(cfg.jupiter_base_url.clone());
        Self { cfg, rpc, jup }
    }

    fn load_keypair(&self) -> Result<Keypair> {
        let path = self
            .cfg
            .sol_keypair_path
            .as_deref()
            .ok_or_else(|| anyhow!("SOL_KEYPAIR_PATH is required"))?;
        Ok(read_keypair_file(path)?)
    }

    /// Jupiter quote -> build swap -> simulateTransaction (mandatory) -> send.
    pub async fn execute_swap(&self, plan: SwapPlan) -> Result<SwapResult> {
        ensure_slippage_bounds(plan.slippage_bps, self.cfg.max_slippage_bps)?;

        info!(?plan, dry_run = self.cfg.dry_run, "engine.execute_swap");

        // DRY_RUN still performs quote building but does not sign/send.
        let kp = self.load_keypair()?;
        let user_pubkey = kp.pubkey();

        // 1) Quote
        let quote = self
            .jup
            .quote(QuoteRequest {
                input_mint: plan.input_mint.clone(),
                output_mint: plan.output_mint.clone(),
                amount: plan.in_amount.to_string(),
                slippage_bps: plan.slippage_bps,
                only_direct_routes: None,
            })
            .await?;

        // 2) Priority fee (best-effort)
        let compute_unit_price_micro_lamports = self.dynamic_priority_fee_micro_lamports().await.ok();

        // 3) Swap tx from Jupiter
        let swap = self
            .jup
            .swap(SwapRequest {
                quote_response: quote.rest,
                user_public_key: user_pubkey.to_string(),
                wrap_and_unwrap_sol: Some(true),
                compute_unit_price_micro_lamports,
            })
            .await?;

        let tx_bytes = base64::engine::general_purpose::STANDARD.decode(swap.swap_transaction)?;
        let mut vtx: VersionedTransaction = bincode::deserialize(&tx_bytes)?;

        // Sign (Jupiter provides message; we add our sig)
        vtx.sign(&[&kp], self.rpc.get_latest_blockhash().await?)?;

        // 4) simulateTransaction (mandatory)
        let sim = self
            .rpc
            .simulate_transaction_with_config(
                &vtx,
                RpcSimulateTransactionConfig {
                    sig_verify: false,
                    replace_recent_blockhash: true,
                    commitment: Some(CommitmentConfig::processed()),
                    ..RpcSimulateTransactionConfig::default()
                },
            )
            .await?;

        if let Some(err) = sim.value.err {
            return Err(anyhow!("simulateTransaction failed: {err:?}"));
        }

        if self.cfg.dry_run {
            info!("dry_run: simulation ok, skipping send");
            return Ok(SwapResult {
                signature: "DRY_RUN".into(),
            });
        }

        // 5) Send
        let sig = self
            .rpc
            .send_transaction_with_config(
                &vtx,
                RpcSendTransactionConfig {
                    skip_preflight: true, // we already simulated
                    preflight_commitment: Some(CommitmentConfig::processed().commitment),
                    ..RpcSendTransactionConfig::default()
                },
            )
            .await?;

        Ok(SwapResult {
            signature: sig.to_string(),
        })
    }

    /// Best-effort dynamic priority fee.
    ///
    /// Returns micro-lamports per CU.
    async fn dynamic_priority_fee_micro_lamports(&self) -> Result<u64> {
        // Not all RPCs support getRecentPrioritizationFees. We keep it best-effort.
        let fees = self.rpc.get_recent_prioritization_fees(&[]).await?;
        let Some(p) = fees.iter().map(|f| f.prioritization_fee).max() else {
            return Ok(0);
        };
        // Clamp to a sane range; tune later.
        let micro = p.max(1).min(50_000);
        Ok(micro)
    }

    /// Emergency: close all positions immediately (market exit via Jupiter).
    pub async fn close_all_positions(&self) -> Result<()> {
        warn!(dry_run = self.cfg.dry_run, "engine.close_all_positions.stub");
        // TODO: iterate positions from state, build market exits, simulate+send
        Ok(())
    }
}
