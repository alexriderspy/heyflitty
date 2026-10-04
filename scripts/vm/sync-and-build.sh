#!/usr/bin/env bash
# Packs the working tree (including uncommitted changes), serves it to the VM, and builds there.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$(git rev-parse --show-toplevel)"
: "${SPIKE_SERVE_DIR:?set SPIKE_SERVE_DIR to the directory served at http://10.211.55.2:8765}"
TREE=$(git stash create); git archive --format=tar -o "$SPIKE_SERVE_DIR/heyflitty.tar" "${TREE:-HEAD}"
"$SCRIPT_DIR/run-ps.sh" "$SCRIPT_DIR/build.ps1"
