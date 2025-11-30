#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd -P)"
cd "$ROOT_DIR"

# Pre-flight checks
echo "[bootstrap] verifying required tools..."

if ! command -v rustup >/dev/null 2>&1; then
  echo "[bootstrap] error: 'rustup' is not installed or not in PATH." >&2
  echo "[bootstrap] Please install Rust: https://rustup.rs/" >&2
  exit 1
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "[bootstrap] error: 'npm' is not installed or not in PATH." >&2
  echo "[bootstrap] Please install Node.js and npm: https://nodejs.org/" >&2
  exit 1
fi

if [ ! -x "./scripts/setup-hooks.sh" ]; then
  echo "[bootstrap] error: './scripts/setup-hooks.sh' not found or not executable." >&2
  exit 1
fi

echo "[bootstrap] installing rustfmt and clippy components"
rustup component add rustfmt clippy >/dev/null

if [ -f "package.json" ]; then
  echo "[bootstrap] installing JS dev dependencies via npm ci"
  npm ci
else
  echo "[bootstrap] no package.json found, skipping npm ci"
fi

echo "[bootstrap] installing git hooks"
./scripts/setup-hooks.sh

echo "[bootstrap] done"
