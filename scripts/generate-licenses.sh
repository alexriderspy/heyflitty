#!/usr/bin/env bash
# Regenerates THIRD_PARTY_LICENSES.txt from Cargo.lock and the npm dependencies.
# Needs `cargo install cargo-about --locked --features cli`.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
rust=$(cd src-tauri && cargo about generate about.hbs)
js=$(node scripts/js-licenses.mjs)
printf '%s\n\n%s\n' "$js" "$rust" > THIRD_PARTY_LICENSES.txt
echo "wrote THIRD_PARTY_LICENSES.txt ($(wc -l < THIRD_PARTY_LICENSES.txt) lines)"
