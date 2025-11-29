#!/usr/bin/env bash
set -euo pipefail

# Determine the directory of this script
THIS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HOOKS_DIR="${THIS_DIR}/git-hooks"

# 1. Verify existence and directory type
if [ ! -d "${HOOKS_DIR}" ]; then
    echo "Error: Hooks directory '${HOOKS_DIR}' does not exist or is not a directory." >&2
    exit 1
fi

# 2. Resolve canonical path (POSIX compliant) to prevent symlink redirection
CANONICAL_HOOKS_DIR="$(cd "${HOOKS_DIR}" && pwd -P)"

if [ "${HOOKS_DIR}" != "${CANONICAL_HOOKS_DIR}" ]; then
    echo "Error: Hooks directory resolves to '${CANONICAL_HOOKS_DIR}', expected '${HOOKS_DIR}'." >&2
    echo "Symlinked hooks directories are not supported for security reasons." >&2
    exit 1
fi

# 3. Run git config and check status
if git config core.hooksPath "${CANONICAL_HOOKS_DIR}"; then
    echo "Success: Git hooks path set to ${CANONICAL_HOOKS_DIR}"
else
    echo "Error: Failed to set core.hooksPath." >&2
    exit 1
fi
