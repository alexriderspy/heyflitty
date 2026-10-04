# Builds the Windows installer (NSIS .exe) for x64 and ARM64 inside the VM.
param([string[]]$Targets = @('x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc'))
$env:Path = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ';' + $env:Path
Get-Process flitty -ErrorAction SilentlyContinue | Stop-Process -Force
Set-Location (Join-Path $env:USERPROFILE 'heyflitty')
foreach ($target in $Targets) {
  rustup target add $target 2>&1 | Out-Null
  $started = Get-Date
  npx tauri build --target $target --bundles nsis 2>&1 | Out-String -Width 300 | Select-String -Pattern 'error|Finished|warning: unused' | Select-Object -Last 5
  $installer = Get-ChildItem "src-tauri\target\$target\release\bundle\nsis\*.exe" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime | Select-Object -Last 1
  "INSTALLER $target $($installer.FullName) $([int]($installer.Length/1KB))KB in $([int]((Get-Date)-$started).TotalSeconds)s"
}
