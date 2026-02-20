mod config;
mod engine;
mod journal;
mod jupiter;
mod logger;
mod monitoring;
mod notifier;
mod risk;
mod state;
mod time;
mod strategy;

use anyhow::Result;
use rand::{thread_rng, Rng};
use tracing::{error, info, warn};

use crate::config::Config;
use crate::engine::Engine;
use crate::notifier::Notifier;
use crate::risk::{BotMode, RiskEvent, RiskParams, RiskState};
use crate::state::{PersistedState, StateStore};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    monitoring::init_tracing();

    let cfg = Config::from_env()?;
    info!(?cfg, "boot");

    let notifier = Notifier::new(cfg.slack_webhook_url.clone());
    let engine = Engine::new(cfg.clone());

    // Load or initialize state.json
    let store = StateStore::new(&cfg.state_path);
    let day_key = crate::time::day_key(&cfg.tz)?;
    let mut st = match store.load()? {
        Some(s) => {
            info!(path = %cfg.state_path, "state.load.ok");
            s
        }
        None => {
            info!(path = %cfg.state_path, "state.init");
            let risk = RiskState::new(day_key.clone(), cfg.capital_usdc);
            PersistedState::new(risk)
        }
    };

    // Risk params from config
    let risk_params = RiskParams {
        capital_usdc: cfg.capital_usdc,
        position_size_usdc: cfg.position_size_usdc,
        max_open_positions: cfg.max_open_positions,
        max_daily_loss_pct: cfg.max_daily_loss_pct,
        stop_loss_pct: cfg.stop_loss_pct,
        take_profit_pct: cfg.take_profit_pct,
        trailing_arm_pct: cfg.trailing_arm_pct,
        portfolio_hard_stop_pct: cfg.portfolio_hard_stop_pct,
    };

    // Heartbeat log every 5m
    {
        let hb_path = cfg.heartbeat_log_path.clone();
        let notifier_hb = notifier.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
            loop {
                interval.tick().await;
                if let Err(e) = crate::logger::append_line(&hb_path, &crate::logger::heartbeat_line()) {
                    error!(error = %e, "heartbeat.log.write_failed");
                    let _ = notifier_hb.alert(&format!("[SIE] heartbeat log write failed: {e}")).await;
                }
            }
        });
    }

    // Positions loop every 5s
    {
        let notifier_pos = notifier.clone();
        let engine_pos = engine.clone();
        let store_path = cfg.state_path.clone();
        let tz = cfg.tz.clone();
        let trading_journal_path = cfg.trading_journal_path.clone();
        let slippage_bps = cfg.slippage_bps;
        let risk_params = risk_params.clone();

        tokio::spawn(async move {
            let store = StateStore::new(store_path);
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                interval.tick().await;

                // Reload latest state each tick (simple & safe for now)
                let mut st = match store.load() {
                    Ok(Some(s)) => s,
                    Ok(None) => PersistedState::new(RiskState::new(
                        crate::time::day_key(&tz).unwrap_or_else(|_| "1970-01-01".into()),
                        risk_params.capital_usdc,
                    )),
                    Err(e) => {
                        let _ = notifier_pos
                            .alert(&format!("[SIE] state load failed (positions loop): {e}"))
                            .await;
                        continue;
                    }
                };

                // Daily rollover
                if let Ok(dk) = crate::time::day_key(&tz) {
                    st.risk.rollover_day_if_needed(dk);
                }

                // If we ever reach EmergencyStop, liquidate immediately.
                if st.risk.mode == BotMode::EmergencyStop {
                    warn!("risk.emergency_stop.active");
                    let _ = notifier_pos
                        .alert("[SIE] EMERGENCY STOP active: closing all positions immediately")
                        .await;

                    // Liquidate sequentially; keep trying even if some closes fail.
                    let mut idx = 0usize;
                    while idx < st.positions.len() {
                        let p = &st.positions[idx];
                        match engine_pos
                            .close_position_market(
                                p.base_mint.clone(),
                                p.quote_mint.clone(),
                                p.base_amount,
                            )
                            .await
                        {
                            Ok(r) => {
                                let _ = notifier_pos
                                    .alert(&format!(
                                        "[SIE] HARD STOP SELL {} tx={} ",
                                        p.base_mint, r.signature
                                    ))
                                    .await;

                                // Best-effort journal entry
                                let _ = crate::journal::append_trade_close(
                                    &trading_journal_path,
                                    "hard-stop",
                                    &format!("{}/{}", p.base_mint, p.quote_mint),
                                    p.buy_tx.as_deref().unwrap_or(""),
                                    &r.signature,
                                    p.size_usdc,
                                    0.0,
                                    0.0,
                                    crate::risk::ExitReason::HardStop,
                                    "portfolio hard stop: emergency liquidation",
                                    "N/A",
                                );

                                st.positions.remove(idx);
                                continue;
                            }
                            Err(e) => {
                                let _ = notifier_pos
                                    .alert(&format!(
                                        "[SIE] HARD STOP SELL FAILED {}: {e}",
                                        p.base_mint
                                    ))
                                    .await;
                                idx += 1;
                            }
                        }
                    }

                    if let Err(e) = store.save(&st) {
                        let _ = notifier_pos
                            .alert(&format!("[SIE] state save failed after hard stop liquidation: {e}"))
                            .await;
                    }
                }

                // Monitor open positions: compute price and enforce SL/TP/trailing.
                // Exits are allowed even in READ_ONLY.
                let mut closed_any = false;
                let mut i = 0usize;
                while i < st.positions.len() {
                    let mut close_reason = None;

                    let p = &mut st.positions[i];
                    let price = match engine_pos
                        .price_quote_per_base(&p.base_mint, &p.quote_mint)
                        .await
                    {
                        Ok(px) => px,
                        Err(e) => {
                            let _ = notifier_pos
                                .alert(&format!("[SIE] price fetch failed for {}: {e}", p.base_mint))
                                .await;
                            i += 1;
                            continue;
                        }
                    };

                    // update peak
                    if price > p.peak_price {
                        p.peak_price = price;
                    }

                    let pnl_pct = (price - p.entry_price) / p.entry_price;
                    if !p.trailing_armed && pnl_pct >= p.trailing_arm_pct {
                        p.trailing_armed = true;
                    }

                    let stop_price = if p.trailing_armed {
                        p.peak_price * (1.0 - p.stop_loss_pct)
                    } else {
                        p.entry_price * (1.0 - p.stop_loss_pct)
                    };
                    let tp_price = p.entry_price * (1.0 + p.take_profit_pct);

                    if price <= stop_price {
                        close_reason = Some(if p.trailing_armed {
                            crate::risk::ExitReason::TrailingStop
                        } else {
                            crate::risk::ExitReason::StopLoss
                        });
                    } else if price >= tp_price {
                        close_reason = Some(crate::risk::ExitReason::TakeProfit);
                    }

                    if let Some(reason) = close_reason {
                        // Market exit: sell base -> quote.
                        let res = engine_pos
                            .execute_swap(crate::engine::SwapPlan {
                                input_mint: p.base_mint.clone(),
                                output_mint: p.quote_mint.clone(),
                                in_amount: p.base_amount,
                                slippage_bps,
                            })
                            .await;

                        match res {
                            Ok(r) => {
                                p.sell_tx = Some(r.signature.clone());

                                // Realized pnl estimate based on current price.
                                // base tokens (approx) = base_amount / 10^decimals, but we don't persist decimals yet.
                                // We approximate with entry size in USDC for accounting scaffold.
                                let est_exit_usdc = p.size_usdc * (1.0 + pnl_pct);
                                let pnl_usdc = est_exit_usdc - p.size_usdc;

                                let ev = st.risk.register_realized_pnl(&risk_params, pnl_usdc);
                                st.sync_mode_from_risk();

                                let _ = notifier_pos
                                    .alert(&format!(
                                        "[SIE] SELL {} reason={:?} pnl=${:.2} ({:.2}%) tx={} mode={:?}",
                                        p.base_mint,
                                        reason,
                                        pnl_usdc,
                                        pnl_pct * 100.0,
                                        r.signature,
                                        st.risk.mode
                                    ))
                                    .await;

                                // Journal append (best-effort)
                                let _ = crate::journal::append_trade_close(
                                    &trading_journal_path,
                                    "momentum-scalping", // placeholder until strategy tags positions
                                    &format!("{}/{}", p.base_mint, p.quote_mint),
                                    p.buy_tx.as_deref().unwrap_or(""),
                                    p.sell_tx.as_deref().unwrap_or(""),
                                    p.size_usdc,
                                    pnl_usdc,
                                    pnl_pct,
                                    reason,
                                    "auto-exit via risk rules",
                                    "N/A (pricefeed scaffold)",
                                );

                                // Remove position
                                st.positions.remove(i);
                                closed_any = true;

                                // React to mode transitions.
                                if matches!(ev, RiskEvent::EnterReadOnly) {
                                    let _ = notifier_pos
                                        .alert("[SIE] READ_ONLY entered: daily loss limit reached")
                                        .await;
                                }
                                if matches!(ev, RiskEvent::EnterEmergencyStop) {
                                    let _ = notifier_pos
                                        .alert("[SIE] EMERGENCY STOP entered: portfolio hard stop reached")
                                        .await;
                                    // Liquidate remaining positions ASAP (loop continues)
                                }

                                continue; // do not increment i (we removed current)
                            }
                            Err(e) => {
                                let _ = notifier_pos
                                    .alert(&format!("[SIE] SELL failed for {}: {e}", p.base_mint))
                                    .await;
                                i += 1;
                                continue;
                            }
                        }
                    }

                    i += 1;
                }

                if closed_any {
                    if let Err(e) = store.save(&st) {
                        let _ = notifier_pos
                            .alert(&format!("[SIE] state save failed after closes: {e}"))
                            .await;
                    }
                } else {
                    st.sync_mode_from_risk();
                    if let Err(e) = store.save(&st) {
                        let _ = notifier_pos
                            .alert(&format!("[SIE] state save failed (positions loop): {e}"))
                            .await;
                    }
                }
            }
        });
    }

    // Market loop every 10-20s (jitter)
    {
        let notifier_mkt = notifier.clone();
        let store_path = cfg.state_path.clone();
        let tz = cfg.tz.clone();
        let risk_params = risk_params.clone();

        tokio::spawn(async move {
            let store = StateStore::new(store_path);
            loop {
                let sleep_s: u64 = thread_rng().gen_range(10..=20);
                tokio::time::sleep(std::time::Duration::from_secs(sleep_s)).await;

                let mut st = match store.load() {
                    Ok(Some(s)) => s,
                    Ok(None) => PersistedState::new(RiskState::new(
                        crate::time::day_key(&tz).unwrap_or_else(|_| "1970-01-01".into()),
                        risk_params.capital_usdc,
                    )),
                    Err(e) => {
                        let _ = notifier_mkt
                            .alert(&format!("[SIE] state load failed (market loop): {e}"))
                            .await;
                        continue;
                    }
                };

                if let Ok(dk) = crate::time::day_key(&tz) {
                    st.risk.rollover_day_if_needed(dk);
                }

                // Enforce read-only: no new positions.
                if st.risk.mode != BotMode::Trading {
                    continue;
                }

                // Strategy (momentum scalping) - DRY-RUN scaffold.
                // NOTE: We still don't have candle/volume feed wired, so this emits no intents.
                let scalper = crate::strategy::momentum::MomentumScalper::new(
                    // USDC mint (mainnet)
                    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                    risk_params.position_size_usdc,
                    vec![],
                );
                if let Ok(intents) = scalper.evaluate() {
                    if !intents.is_empty() {
                        let _ = notifier_mkt
                            .alert(&format!("[SIE] momentum intents: {}", intents.len()))
                            .await;
                    }
                }

                // TODO next:
                // - wire candle source (1m/5m) and volume breakout
                // - risk gate + max positions
                // - call engine.execute_swap (simulateTransaction mandatory)

                st.sync_mode_from_risk();
                if let Err(e) = store.save(&st) {
                    let _ = notifier_mkt
                        .alert(&format!("[SIE] state save failed (market loop): {e}"))
                        .await;
                }
            }
        });
    }

    // Boot notice
    notifier
        .alert(&format!(
            "[SIE] daemon started (mode={:?}, dry_run={})",
            st.risk.mode, cfg.dry_run
        ))
        .await
        .ok();

    // Main: persist state periodically + watch for mode transitions
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        interval.tick().await;

        // Daily rollover
        let new_day_key = crate::time::day_key(&cfg.tz).unwrap_or_else(|_| day_key.clone());
        st.risk.rollover_day_if_needed(new_day_key);

        // If risk state says enter read-only/emergency, alert.
        match st.risk.mode {
            BotMode::ReadOnly => {
                notifier
                    .alert(&format!(
                        "[SIE] READ_ONLY: daily loss limit breached (limit=${:.2})",
                        risk_params.daily_loss_limit_usdc()
                    ))
                    .await
                    .ok();
            }
            BotMode::EmergencyStop => {
                notifier
                    .alert("[SIE] EMERGENCY STOP: portfolio hard stop breached")
                    .await
                    .ok();
            }
            BotMode::Trading => {}
        }

        st.sync_mode_from_risk();
        if let Err(e) = store.save(&st) {
            error!(error = %e, "state.save_failed");
        }

        info!(mode = ?st.risk.mode, open_positions = st.positions.len(), "tick");

        // Keep risk events placeholder used (for future wiring on trade close)
        let _ = RiskEvent::None;
    }
}
