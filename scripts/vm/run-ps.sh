#!/usr/bin/env bash
# Runs a local PowerShell script inside the Parallels VM.
# The script is served from the host over the Parallels shared network, because
# long -EncodedCommand payloads exceed what prlctl exec accepts.
# Usage: run-ps.sh [--system] script.ps1 [script args...]   (default: the logged-in user's desktop session)
set -euo pipefail
VM="${SPIKE_VM:-Windows 11}"
SERVE_DIR="${SPIKE_SERVE_DIR:?set SPIKE_SERVE_DIR to the directory served at http://10.211.55.2:8765}"
USER_FLAG="--current-user"
if [[ "${1:-}" == "--system" ]]; then USER_FLAG=""; shift; fi
SCRIPT="$1"; shift
SCRIPT_ARGS="$*"
NAME="run-$(date +%s%N)-$(basename "$SCRIPT")"
cp "$SCRIPT" "$SERVE_DIR/$NAME"
LOADER="\$ProgressPreference='SilentlyContinue'; \$f=Join-Path \$env:TEMP '$NAME'; Invoke-WebRequest -UseBasicParsing -Uri 'http://10.211.55.2:8765/$NAME' -OutFile \$f; & \$f $SCRIPT_ARGS; Remove-Item \$f"
prlctl exec "$VM" $USER_FLAG powershell -NoProfile -ExecutionPolicy Bypass -Command "$LOADER"
rm -f "$SERVE_DIR/$NAME"
