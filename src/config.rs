use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    // RPC
    pub helius_http_url: String,
    pub helius_wss_url: String,
    pub quicknode_http_url: Option<String>,
    pub quicknode_wss_url: Option<String>,

    // Alerts
    pub slack_webhook_url: Option<String>,

    // Runtime
    pub dry_run: bool,
    pub tz: String,

    // Risk (defaults match spec)
    pub capital_usdc: f64,
    pub position_size_usdc: f64,
    pub max_open_positions: usize,
    pub max_daily_loss_pct: f64,
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub trailing_arm_pct: f64,
    pub portfolio_hard_stop_pct: f64,

    // Execution
    pub jupiter_base_url: String,
    pub slippage_bps: u64,
    pub max_slippage_bps: u64,

    // Keys
    pub sol_keypair_path: Option<String>,

    // Persistence
    pub state_path: String,
    pub heartbeat_log_path: String,
    pub trading_journal_path: String,
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
        // RPC
        let helius_http_url = std::env::var("HELIUS_HTTP_URL")
            .or_else(|_| std::env::var("SIE_RPC_HTTP"))
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        let helius_wss_url = std::env::var("HELIUS_WSS_URL")
            .or_else(|_| std::env::var("SIE_RPC_WS"))
            .unwrap_or_else(|_| "wss://api.mainnet-beta.solana.com".to_string());

        let quicknode_http_url = std::env::var("QUICKNODE_HTTP_URL").ok();
        let quicknode_wss_url = std::env::var("QUICKNODE_WSS_URL").ok();

        // Alerts
        let slack_webhook_url = std::env::var("SLACK_WEBHOOK_URL").ok();

        // Runtime
        let dry_run = env_bool("DRY_RUN", true);
        let tz = std::env::var("SIE_TZ").unwrap_or_else(|_| "America/Buenos_Aires".to_string());

        // Risk
        let capital_usdc = env_parse::<f64>("SIE_CAPITAL_USDC").unwrap_or(200.0);
        let position_size_usdc = env_parse::<f64>("SIE_POSITION_SIZE_USDC").unwrap_or(20.0);
        let max_open_positions = env_parse::<usize>("MAX_OPEN_POSITIONS").unwrap_or(5);
        let max_daily_loss_pct = env_parse::<f64>("SIE_MAX_DAILY_LOSS_PCT").unwrap_or(0.03);
        let stop_loss_pct = env_parse::<f64>("SIE_STOP_LOSS_PCT").unwrap_or(0.10);
        let take_profit_pct = env_parse::<f64>("SIE_TAKE_PROFIT_PCT").unwrap_or(0.40);
        let trailing_arm_pct = env_parse::<f64>("SIE_TRAILING_ARM_PCT").unwrap_or(0.15);
        let portfolio_hard_stop_pct = env_parse::<f64>("SIE_PORTFOLIO_HARD_STOP_PCT").unwrap_or(0.20);

        if position_size_usdc <= 0.0 || capital_usdc <= 0.0 {
            return Err(anyhow!("invalid capital/position size"));
        }

        // Execution
        let jupiter_base_url = std::env::var("JUPITER_BASE_URL")
            .unwrap_or_else(|_| "https://quote-api.jup.ag".to_string());
        let slippage_bps = env_parse::<u64>("SIE_SLIPPAGE_BPS").unwrap_or(50);
        let max_slippage_bps = env_parse::<u64>("SIE_MAX_SLIPPAGE_BPS").unwrap_or(100);
        if slippage_bps > max_slippage_bps {
            return Err(anyhow!("SIE_SLIPPAGE_BPS cannot exceed SIE_MAX_SLIPPAGE_BPS"));
        }

        let sol_keypair_path = std::env::var("SOL_KEYPAIR_PATH").ok();

        let state_path = std::env::var("SIE_STATE_PATH").unwrap_or_else(|_| "./state.json".to_string());
        let heartbeat_log_path = std::env::var("SIE_HEARTBEAT_LOG").unwrap_or_else(|_| "./heartbeat.log".to_string());
        let trading_journal_path = std::env::var("SIE_TRADING_MD").unwrap_or_else(|_| "./docs/trading.md".to_string());

        Ok(Self {
            helius_http_url,
            helius_wss_url,
            quicknode_http_url,
            quicknode_wss_url,
            slack_webhook_url,
            dry_run,
            tz,
            capital_usdc,
            position_size_usdc,
            max_open_positions,
            max_daily_loss_pct,
            stop_loss_pct,
            take_profit_pct,
            trailing_arm_pct,
            portfolio_hard_stop_pct,
            jupiter_base_url,
            slippage_bps,
            max_slippage_bps,
            sol_keypair_path,
            state_path,
            heartbeat_log_path,
            trading_journal_path,
        })
    }
}
