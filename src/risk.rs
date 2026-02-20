use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParams {
    pub capital_usdc: f64,
    pub position_size_usdc: f64,
    pub max_open_positions: usize,

    // Daily loss guard
    pub max_daily_loss_pct: f64, // 0.03 => 3%

    // Per-trade exits
    pub stop_loss_pct: f64,    // 0.10 => 10%
    pub take_profit_pct: f64,  // 0.40 => 40%
    pub trailing_arm_pct: f64, // 0.15 => 15% profit arms trailing

    // Portfolio emergency
    pub portfolio_hard_stop_pct: f64, // 0.20 => 20%
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BotMode {
    Trading,
    ReadOnly,
    EmergencyStop,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExitReason {
    StopLoss,
    TrailingStop,
    TakeProfit,
    DailyLossLimit,
    HardStop,
    Manual,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DailyPnl {
    pub day_key: String, // YYYY-MM-DD in configured TZ
    pub realized_pnl_usdc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskState {
    pub mode: BotMode,
    pub daily: DailyPnl,
    pub starting_balance_usdc: f64,
    pub current_balance_usdc: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RiskEvent {
    None,
    EnterReadOnly,      // daily -3% hit
    EnterEmergencyStop, // portfolio -20% hit
}

impl RiskState {
    pub fn new(day_key: String, starting_balance_usdc: f64) -> Self {
        Self {
            mode: BotMode::Trading,
            daily: DailyPnl {
                day_key,
                realized_pnl_usdc: 0.0,
            },
            starting_balance_usdc,
            current_balance_usdc: starting_balance_usdc,
        }
    }

    pub fn rollover_day_if_needed(&mut self, new_day_key: String) {
        if self.daily.day_key != new_day_key {
            self.daily.day_key = new_day_key;
            self.daily.realized_pnl_usdc = 0.0;
            // keep mode as-is (ReadOnly remains until operator decides otherwise)
        }
    }

    /// Registers realized PnL, updates mode if limits are breached, and returns an event to act on.
    pub fn register_realized_pnl(&mut self, params: &RiskParams, pnl_usdc: f64) -> RiskEvent {
        let prev_mode = self.mode;

        self.daily.realized_pnl_usdc += pnl_usdc;
        self.current_balance_usdc += pnl_usdc;

        // Rule: PROHIBITED to lose more than X% of total capital per day.
        let max_loss = -params.max_daily_loss_pct * params.capital_usdc;
        if self.daily.realized_pnl_usdc <= max_loss {
            self.mode = BotMode::ReadOnly;
        }

        // Hard stop based on total balance drawdown vs starting balance.
        let dd = (self.current_balance_usdc - self.starting_balance_usdc) / self.starting_balance_usdc;
        if dd <= -params.portfolio_hard_stop_pct {
            self.mode = BotMode::EmergencyStop;
        }

        match (prev_mode, self.mode) {
            (BotMode::Trading, BotMode::ReadOnly) => RiskEvent::EnterReadOnly,
            (_, BotMode::EmergencyStop) if prev_mode != BotMode::EmergencyStop => RiskEvent::EnterEmergencyStop,
            _ => RiskEvent::None,
        }
    }

    pub fn can_open_new_position(&self, params: &RiskParams, open_positions: usize) -> bool {
        matches!(self.mode, BotMode::Trading) && open_positions < params.max_open_positions
    }
}

impl RiskParams {
    pub fn daily_loss_limit_usdc(&self) -> f64 {
        self.max_daily_loss_pct * self.capital_usdc
    }
}
