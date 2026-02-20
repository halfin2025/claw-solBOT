# Ops (native)

Este bot se instala **nativo** (sin Docker).

## 1) Instalar Rust toolchain (una vez)

Recomendado (rustup):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
```

Verificar:
```bash
cargo --version
rustc --version
```

## 2) Ubicación recomendada

```bash
sudo mkdir -p /opt/claw-solbot
sudo chown -R $USER:$USER /opt/claw-solbot
cd /opt/claw-solbot

git clone git@github.com:halfin2025/claw-solBOT.git .
cp .env.example .env
```

Completar `.env`.

## 3) systemd

Copiar unit file:
```bash
sudo cp ops/systemd/solbot.service /etc/systemd/system/solbot.service
sudo systemctl daemon-reload
sudo systemctl enable --now solbot
```

Logs:
```bash
journalctl -u solbot -f
```

## 4) Actualizar

```bash
cd /opt/claw-solbot

git pull
cargo build --release
sudo systemctl restart solbot
```

## Notas
- `state.json` y `heartbeat.log` se escriben en el working dir (/opt/claw-solbot) por defecto.
- Si querés rutas diferentes, setear `SIE_STATE_PATH` / `SIE_HEARTBEAT_LOG` en `.env`.
