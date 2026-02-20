use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::risk::{BotMode, RiskState};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub id: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub size_usdc: f64,

    // pricing (placeholder until we wire real price feed)
    pub entry_price: f64,
    pub peak_price: f64,

    // exit rules snapshot
    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub trailing_arm_pct: f64,
    pub trailing_armed: bool,

    // tx ids
    pub buy_tx: Option<String>,
    pub sell_tx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub version: u32,
    pub mode: BotMode,

    // Risk accounting (daily pnl counters + modes)
    pub risk: RiskState,

    // Open positions
    pub positions: Vec<Position>,
}

pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> Result<Option<PersistedState>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.path)?;
        let st = serde_json::from_str(&raw)?;
        Ok(Some(st))
    }

    pub fn save(&self, st: &PersistedState) -> Result<()> {
        let raw = serde_json::to_string_pretty(st)?;
        fs::write(&self.path, raw)?;
        Ok(())
    }
}

impl PersistedState {
    pub fn new(risk: RiskState) -> Self {
        Self {
            version: 1,
            mode: risk.mode,
            risk,
            positions: vec![],
        }
    }

    pub fn sync_mode_from_risk(&mut self) {
        self.mode = self.risk.mode;
    }
}
