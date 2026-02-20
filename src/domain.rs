use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Strategy {
    MomentumScalping,
    AntiRugSniping,
    LstArb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeIntent {
    pub strategy: Strategy,
    pub base_mint: String,
    pub quote_mint: String,
    pub size_usdc: f64,

    /// Optional metadata for logging/journaling.
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVerdict {
    pub pass: bool,
    pub score: f64,
    pub reasons: Vec<String>,
}
