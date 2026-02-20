mod config;
mod engine;
mod journal;
mod logger;
mod monitoring;
mod notifier;
mod risk;
mod state;
mod time;

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
                    if let Err(e) = engine_pos.close_all_positions().await {
                        let _ = notifier_pos
                            .alert(&format!("[SIE] emergency liquidation failed: {e}"))
                            .await;
                    }
                }

                st.sync_mode_from_risk();
                if let Err(e) = store.save(&st) {
                    let _ = notifier_pos
                        .alert(&format!("[SIE] state save failed (positions loop): {e}"))
                        .await;
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

                // TODO:
                // - evaluate strategies
                // - build TradeIntent
                // - risk gate + max positions
                // - call engine.execute_swap

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
