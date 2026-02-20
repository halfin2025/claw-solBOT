use anyhow::Result;
use chrono::{DateTime, Local};
use std::{fs::OpenOptions, io::Write, path::Path};

use crate::risk::ExitReason;

pub fn append_trade_close(
    trading_md_path: impl AsRef<Path>,
    strategy: &str,
    token: &str,
    buy_tx: &str,
    sell_tx: &str,
    size_usdc: f64,
    pnl_usdc: f64,
    pnl_pct: f64,
    exit_reason: ExitReason,
    lesson: &str,
    sentiment: &str,
) -> Result<()> {
    let now: DateTime<Local> = Local::now();

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(trading_md_path)?;

    writeln!(f, "- Fecha/Hora (TZ): {}", now.to_rfc3339())?;
    writeln!(f, "- Estrategia: {}", strategy)?;
    writeln!(f, "- Token (base/quote): {}", token)?;
    writeln!(f, "- Tx (buy): {}", buy_tx)?;
    writeln!(f, "- Tx (sell): {}", sell_tx)?;
    writeln!(f, "- Tamaño (USDC): {:.2}", size_usdc)?;
    writeln!(f, "- PnL (USDC / %): {:.2} / {:.2}%", pnl_usdc, pnl_pct * 100.0)?;
    writeln!(f, "- Motivo de salida: {:?}\n", exit_reason)?;
    writeln!(f, "### Lección aprendida\n\n{}\n", lesson)?;
    writeln!(f, "### Tendencia / sentimiento (Solana)\n\n{}\n", sentiment)?;
    writeln!(f, "---\n")?;

    Ok(())
}
