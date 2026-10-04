# Fetches the latest tree from the host and builds Flitty inside the VM.
$ErrorActionPreference = 'Stop'
# A running copy locks flitty.exe and the link step fails.
Get-Process flitty -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 500
$env:Path = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ';' + $env:Path
$root = Join-Path $env:USERPROFILE 'heyflitty'
New-Item -ItemType Directory -Force -Path $root | Out-Null
$tar = Join-Path $env:TEMP 'heyflitty.tar'
Invoke-WebRequest -UseBasicParsing -Uri 'http://10.211.55.2:8765/heyflitty.tar' -OutFile $tar
# Replace sources but keep node_modules and the Rust target dir between builds.
Get-ChildItem $root -Force | Where-Object { $_.Name -notin @('node_modules') } | ForEach-Object {
  if ($_.Name -eq 'src-tauri') { Get-ChildItem $_.FullName -Force | Where-Object { $_.Name -ne 'target' } | Remove-Item -Recurse -Force }
  else { Remove-Item $_.FullName -Recurse -Force }
}
tar -xf $tar -C $root
Set-Location $root
$ErrorActionPreference = 'Continue'
$started = Get-Date
npm install --no-audit --no-fund --loglevel=error --update-notifier=false 2>&1 | Out-String | Select-Object -Last 1 | Out-Null
npx tauri build --no-bundle 2>&1 | Select-String -Pattern '^(error|warning: unused)|-->|Finished|Built application|failed' | Select-Object -Last 25
"build exit $LASTEXITCODE in $([int]((Get-Date)-$started).TotalSeconds)s"
