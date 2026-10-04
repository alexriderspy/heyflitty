#!/usr/bin/env bash
# Copies a file out of the VM in chunks (prlctl exec truncates large outputs).
# Usage: pull-file.sh 'C:\path\in\vm' local-destination
set -euo pipefail
VM="${SPIKE_VM:-Windows 11}"
SOURCE="$1"; DEST="$2"; CHUNK=120000
size=$(prlctl exec "$VM" --current-user powershell -NoProfile -WindowStyle Hidden -Command "(Get-Item '$SOURCE').Length" | tr -d '\r\n ')
: > "$DEST"
for ((offset = 0; offset < size; offset += CHUNK)); do
  prlctl exec "$VM" --current-user powershell -NoProfile -WindowStyle Hidden -Command \
    "\$f=[IO.File]::OpenRead('$SOURCE'); \$f.Seek($offset,0) | Out-Null; \$b=New-Object byte[] $CHUNK; \$n=\$f.Read(\$b,0,$CHUNK); \$f.Close(); [Convert]::ToBase64String(\$b,0,\$n)" \
    | tr -d '\r\n' | base64 -D >> "$DEST"
done
echo "pulled $size bytes -> $DEST ($(wc -c < "$DEST" | tr -d ' ') written)"
