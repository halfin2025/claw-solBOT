use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::{Path, PathBuf}};

use crate::risk::{BotMode, RiskState};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub id: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub size_usdc: f64,

    pub entry_price: f64,
    pub peak_price: f64,

    pub stop_loss_pct: f64,
    pub take_profit_pct: f64,
    pub trailing_armed: bool,

    pub buy_tx: Option<String>,
    pub sell_tx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub version: u32,
    pub mode: BotMode,
    pub risk: RiskState,
    pub positions: Vec<Position>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: 1,
            mode: BotMode::Trading,
            risk: RiskState::new("1970-01-01".into(), 0.0),
            positions: vec![],
        }
    }
}

pub struct StateStore {
    path: PathBuf,
}

impl StateStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self { path: path.as_ref().to_path_buf() }
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
