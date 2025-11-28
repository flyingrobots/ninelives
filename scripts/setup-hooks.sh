#!/usr/bin/env bash
set -euo pipefail

# Point git to the version-controlled hooks in scripts/git-hooks
THIS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
git config core.hooksPath "${THIS_DIR}/git-hooks"
echo "Git hooks path set to ${THIS_DIR}/git-hooks"
