# Solana Infinity Engine (SIE)

High-frequency Solana trading bot focused on:
- New pool detection (Raydium / Meteora / Pump.fun)
- Fast execution (Jupiter v6 + Jito bundles)
- Defensive risk controls (anti-honeypot, liquidity shock exits)

This repo is intentionally **separate** from the Polymarket bot.

## Status
Scaffold only (Rust toolchain not yet installed on this host).

## Next steps
1) Install Rust toolchain (`rustup` + `cargo`) or system packages.
2) `cargo build` + start implementing scanner modules.

## Non-goals
No support for fraud / rug-pull creation.
