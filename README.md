# Claw SOL Bot (Solana Infinity Engine)

Bot de trading autónomo en Solana con **gestión de riesgo estricta**.

## Objetivo
- Ejecutar estrategias (momentum scalping, anti-rug sniping, LST arb)
- **Cumplir reglas de riesgo sin excepción** (daily loss -3%, SL/TP, hard stop)
- Ejecución vía **Jupiter v6+** (y anti-MEV con Jito cuando aplique)

## Estado
- Scaffold inicial (daemon + config + logging + notifier Slack)
- Falta implementar: Jupiter engine, estrategias, persistencia completa `state.json`, simulación, priority fees, Jito bundles.

## Configuración
Copia `.env.example` a `.env` y completa lo necesario.

### Slack Alerts
Configurar `SLACK_WEBHOOK_URL`.

## Run
```bash
cargo run
```

## Ops (native)
Ver `ops/README.md` para instalación nativa + systemd.

## Documentación evolutiva
- `docs/trading.md` se actualiza al cerrar cada posición.

## Seguridad
- No se implementa nada relacionado a creación de rugs/fraude.
- El bot debe entrar en **READ_ONLY** al alcanzar el límite diario de pérdida.
