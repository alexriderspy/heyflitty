#!/usr/bin/env bash
# Copies a file out of the VM in chunks (prlctl exec truncates large outputs).
# Usage: pull-file.sh 'C:\path\in\vm' local-destination
set -euo pipefail
VM="${SPIKE_VM:-Windows 11}"
SOURCE="$1"; DEST="$2"; CHUNK=120000
size=$(prlctl exec "$VM" --current-user powershell -NoProfile -WindowStyle Hidden -Command "(Get-Item '$SOURCE').Length" | tr -d '\r\n ')
: > "$DEST"
for ((offset = 0; offset < size; offset += CHUNK)); do
  # prlctl exec fails now and then; retry a chunk a few times before giving up.
  for attempt in 1 2 3 4; do
    chunk=$(prlctl exec "$VM" --current-user powershell -NoProfile -WindowStyle Hidden -Command \
      "\$f=[IO.File]::OpenRead('$SOURCE'); \$f.Seek($offset,0) | Out-Null; \$b=New-Object byte[] $CHUNK; \$n=\$f.Read(\$b,0,$CHUNK); \$f.Close(); [Convert]::ToBase64String(\$b,0,\$n)" 2>/dev/null | tr -d '\r\n') && [[ -n "$chunk" ]] && break
    sleep 2
  done
  [[ -n "$chunk" ]] || { echo "failed at offset $offset" >&2; exit 1; }
  printf '%s' "$chunk" | base64 -D >> "$DEST"
done
echo "pulled $size bytes -> $DEST ($(wc -c < "$DEST" | tr -d ' ') written)"
