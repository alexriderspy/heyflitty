Get-Process flitty -ErrorAction SilentlyContinue | Stop-Process -Force
Unregister-ScheduledTask -TaskName 'flitty' -Confirm:$false -ErrorAction SilentlyContinue
"stopped"
