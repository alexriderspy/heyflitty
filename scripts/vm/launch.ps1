# Starts HeyFlitty in the interactive desktop session via a one-shot scheduled task,
# so it outlives the prlctl session that launched it.
$exe = Join-Path $env:USERPROFILE 'heyflitty\src-tauri\target\debug\flitty.exe'
Get-Process flitty -ErrorAction SilentlyContinue | Stop-Process -Force
$log = Join-Path $env:TEMP 'flitty.log'
$action = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument "/c `"`"$exe`" > `"$log`" 2>&1`"" -WorkingDirectory (Split-Path $exe)
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero)
Register-ScheduledTask -TaskName 'flitty' -Action $action -Principal $principal -Settings $settings -Force | Out-Null
Start-ScheduledTask -TaskName 'flitty'
Start-Sleep -Seconds 4
"pid: " + (Get-Process flitty -ErrorAction SilentlyContinue).Id
Get-Content $log -ErrorAction SilentlyContinue | Select-Object -Last 5
