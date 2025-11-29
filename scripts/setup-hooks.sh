#!/usr/bin/env bash
set -euo pipefail

# Determine the directory of this script (physical path to avoid symlink ancestors)
THIS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
HOOKS_DIR="${THIS_DIR}/git-hooks"

# 1. Verify existence and directory type
if [ ! -d "${HOOKS_DIR}" ]; then
    echo "Error: Hooks directory '${HOOKS_DIR}' does not exist or is not a directory." >&2
    exit 1
fi

# 2. Resolve canonical path (POSIX compliant) to prevent symlink redirection
CANONICAL_HOOKS_DIR="$(cd "${HOOKS_DIR}" && pwd -P)"

# Explicitly reject the hooks directory itself being a symlink.
if [ -L "${HOOKS_DIR}" ]; then
    echo "Error: Hooks directory '${HOOKS_DIR}' is a symlink; symlinked hooks are not supported." >&2
    exit 1
fi

# 3. Run git config and check status
if git config core.hooksPath "${CANONICAL_HOOKS_DIR}"; then
    echo "Success: Git hooks path set to ${CANONICAL_HOOKS_DIR}"
else
    echo "Error: Failed to set core.hooksPath." >&2
    exit 1
fi
