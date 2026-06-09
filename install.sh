#!/usr/bin/env bash
# install.sh — build and install ousia-atscale to ~/.local/bin/
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN="$HOME/.local/bin/ousia-atscale"

echo "[install] Building ousia-atscale (release)..."
cargo build --release --manifest-path "$CRATE_DIR/Cargo.toml"

mkdir -p "$HOME/.local/bin"
cp "$CRATE_DIR/target/release/ousia-atscale" "$BIN"
chmod +x "$BIN"

echo "[install] Installed to $BIN"
"$BIN" --version
