use anyhow::Result;

use crate::domain::{Strategy, TradeIntent};

#[derive(Debug, Clone)]
pub struct MomentumParams {
    pub rsi_period: usize,
    pub rsi_breakout: f64,
    pub min_volume_usdc_1m: f64,
}

#[derive(Debug, Clone)]
pub struct MomentumScalper {
    pub quote_mint: String,
    pub size_usdc: f64,
    pub params: MomentumParams,

    // TODO: wire candle source (1m/5m) for RSI + volume breakout.
    pub watchlist_base_mints: Vec<String>,
}

impl MomentumScalper {
    pub fn new(quote_mint: String, size_usdc: f64, watchlist_base_mints: Vec<String>) -> Self {
        Self {
            quote_mint,
            size_usdc,
            params: MomentumParams {
                rsi_period: 14,
                rsi_breakout: 60.0,
                min_volume_usdc_1m: 50_000.0,
            },
            watchlist_base_mints,
        }
    }

    /// Returns intents in DRY-RUN mode only once we have market data.
    pub fn evaluate(&self) -> Result<Vec<TradeIntent>> {
        // Hard requirement from spec:
        // - Momentum scalping based on volume breakout + RSI on 1m/5m.
        // We don't have candle data provider yet, so we return none.
        Ok(vec![])
    }
}

// --- indicator utilities (pure, unit-testable) ---

/// Simple RSI (Wilder) over close prices.
pub fn rsi_wilder(closes: &[f64], period: usize) -> Option<f64> {
    if period == 0 || closes.len() < period + 1 {
        return None;
    }

    let mut gain = 0.0;
    let mut loss = 0.0;

    for i in 1..=period {
        let diff = closes[i] - closes[i - 1];
        if diff >= 0.0 {
            gain += diff;
        } else {
            loss += -diff;
        }
    }

    let mut avg_gain = gain / period as f64;
    let mut avg_loss = loss / period as f64;

    for i in (period + 1)..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let (g, l) = if diff >= 0.0 { (diff, 0.0) } else { (0.0, -diff) };
        avg_gain = (avg_gain * (period as f64 - 1.0) + g) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + l) / period as f64;
    }

    if avg_loss == 0.0 {
        return Some(100.0);
    }

    let rs = avg_gain / avg_loss;
    Some(100.0 - (100.0 / (1.0 + rs)))
}

/// Placeholder: build a trade intent once signals are satisfied.
pub fn intent_buy(base_mint: String, quote_mint: String, size_usdc: f64) -> TradeIntent {
    TradeIntent {
        strategy: Strategy::MomentumScalping,
        base_mint,
        quote_mint,
        size_usdc,
        notes: vec![],
    }
}
