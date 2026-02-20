use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolCandidate {
    pub venue: Venue,
    pub pool_address: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub slot: u64,
    pub tx_sig: String,
    pub detected_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Venue {
    Raydium,
    Meteora,
    PumpFun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVerdict {
    pub pass: bool,
    pub score: f64,
    pub reasons: Vec<String>,
}
