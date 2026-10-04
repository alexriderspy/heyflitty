#!/usr/bin/env bash
# Packs the working tree (including uncommitted changes), serves it to the VM, and builds there.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$(git rev-parse --show-toplevel)"
: "${SPIKE_SERVE_DIR:?set SPIKE_SERVE_DIR to the directory served at http://10.211.55.2:8765}"
# Tracked and untracked (but not ignored) files, so new files are included before they are committed.
git ls-files -z --cached --others --exclude-standard | COPYFILE_DISABLE=1 xargs -0 tar --no-mac-metadata -cf "$SPIKE_SERVE_DIR/heyflitty.tar"
"$SCRIPT_DIR/run-ps.sh" "$SCRIPT_DIR/build.ps1"
