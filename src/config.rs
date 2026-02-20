use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // Primary + failover RPC
    pub helius_http_url: String,
    pub helius_wss_url: String,
    pub quicknode_http_url: Option<String>,
    pub quicknode_wss_url: Option<String>,
    pub rpc_failover_p95_ms: u64,

    // Execution
    pub dry_run: bool,
    pub sol_keypair_path: Option<String>,

    // Alpha / copy-trade inputs
    pub alpha_wallets_path: String,

    // Jito
    pub jito_bundle_url: Option<String>,
    pub jito_auth_token: Option<String>,
    pub jito_tip_lamports: u64,

    // Strategy
    pub quote_asset: QuoteAsset,
    pub max_new_token_size_sol: f64,
    pub max_established_token_size_sol: f64,

    // Risk
    pub max_daily_dd_pct: f64,
    pub panic_liquidity_drop_pct: f64,
    pub max_usdc_per_trade: f64,
    pub max_daily_loss_usdc: f64,
    pub max_open_positions: usize,

    // Back-compat (older env names)
    pub rpc_http_legacy: Option<String>,
    pub rpc_ws_legacy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuoteAsset {
    SOL,
    USDC,
}

fn env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key).ok().map(|s| s.trim().to_lowercase()) {
        None => default,
        Some(v) if v.is_empty() => default,
        Some(v) if v == "1" || v == "true" || v == "yes" || v == "y" || v == "on" => true,
        Some(v) if v == "0" || v == "false" || v == "no" || v == "n" || v == "off" => false,
        Some(_) => default,
    }
}

fn env_parse<T: std::str::FromStr>(key: &str) -> Option<T> {
    std::env::var(key).ok().and_then(|x| x.parse().ok())
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // New names (preferred)
        let helius_http_url = std::env::var("HELIUS_HTTP_URL")
            .or_else(|_| std::env::var("SIE_RPC_HTTP"))
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        let helius_wss_url = std::env::var("HELIUS_WSS_URL")
            .or_else(|_| std::env::var("SIE_RPC_WS"))
            .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string());

        let quicknode_http_url = std::env::var("QUICKNODE_HTTP_URL").ok();
        let quicknode_wss_url = std::env::var("QUICKNODE_WSS_URL").ok();

        let rpc_failover_p95_ms = env_parse::<u64>("RPC_FAILOVER_P95_MS").unwrap_or(150);

        let dry_run = env_bool("DRY_RUN", true);
        let sol_keypair_path = std::env::var("SOL_KEYPAIR_PATH").ok();

        let alpha_wallets_path = std::env::var("ALPHA_WALLETS_PATH")
            .unwrap_or_else(|_| "./alpha_wallets.txt".to_string());

        let jito_bundle_url = std::env::var("JITO_BUNDLE_URL").ok();
        let jito_auth_token = std::env::var("JITO_AUTH_TOKEN").ok();
        let jito_tip_lamports = env_parse::<u64>("JITO_TIP_LAMPORTS").unwrap_or(5_000);

        // Strategy / risk (keep legacy SIE_* envs too)
        let quote_asset = match std::env::var("SIE_QUOTE_ASSET")
            .or_else(|_| std::env::var("QUOTE_ASSET"))
            .unwrap_or_else(|_| "USDC".to_string())
            .to_uppercase()
            .as_str()
        {
            "SOL" => QuoteAsset::SOL,
            "USDC" => QuoteAsset::USDC,
            other => return Err(anyhow!("invalid QUOTE_ASSET/SIE_QUOTE_ASSET: {other}")),
        };

        let max_new_token_size_sol = env_parse::<f64>("SIE_MAX_NEW_TOKEN_SIZE_SOL").unwrap_or(0.5);
        let max_established_token_size_sol = env_parse::<f64>("SIE_MAX_ESTABLISHED_TOKEN_SIZE_SOL").unwrap_or(5.0);
        let max_daily_dd_pct = env_parse::<f64>("SIE_MAX_DAILY_DD_PCT").unwrap_or(0.05);
        let panic_liquidity_drop_pct = env_parse::<f64>("SIE_PANIC_LIQUIDITY_DROP_PCT").unwrap_or(0.20);

        let max_usdc_per_trade = env_parse::<f64>("MAX_USDC_PER_TRADE").unwrap_or(5.0);
        let max_daily_loss_usdc = env_parse::<f64>("MAX_DAILY_LOSS_USDC").unwrap_or(10.0);
        let max_open_positions = env_parse::<usize>("MAX_OPEN_POSITIONS").unwrap_or(3);

        // Back-compat capture
        let rpc_http_legacy = std::env::var("SIE_RPC_HTTP").ok();
        let rpc_ws_legacy = std::env::var("SIE_RPC_WS").ok();

        Ok(Self {
            helius_http_url,
            helius_wss_url,
            quicknode_http_url,
            quicknode_wss_url,
            rpc_failover_p95_ms,
            dry_run,
            sol_keypair_path,
            alpha_wallets_path,
            jito_bundle_url,
            jito_auth_token,
            jito_tip_lamports,
            quote_asset,
            max_new_token_size_sol,
            max_established_token_size_sol,
            max_daily_dd_pct,
            panic_liquidity_drop_pct,
            max_usdc_per_trade,
            max_daily_loss_usdc,
            max_open_positions,
            rpc_http_legacy,
            rpc_ws_legacy,
        })
    }
}
