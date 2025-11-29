#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd -P)"
cd "$ROOT_DIR"

echo "[bootstrap] installing rustfmt and clippy components"
rustup component add rustfmt clippy >/dev/null

echo "[bootstrap] installing JS dev dependencies via npm ci"
npm ci

echo "[bootstrap] installing git hooks"
./scripts/setup-hooks.sh

echo "[bootstrap] done"
