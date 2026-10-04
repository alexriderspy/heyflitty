# End-to-end checks of the AI loop against the mock OpenAI server (scripts/mock-ai/server.py on the host).
# Uses FLITTY_TEST_TRANSCRIPT so no microphone is needed, and a throwaway test key that is removed afterwards.
param([string]$Scenario = 'reply')
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class K {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
}
'@
[K]::SetProcessDPIAware() | Out-Null
function PushToTalk($holdMs) { foreach ($k in 0x11,0x12,0x20) { [K]::keybd_event($k,0,0,[UIntPtr]::Zero) }; Start-Sleep -Milliseconds $holdMs; foreach ($k in 0x20,0x12,0x11) { [K]::keybd_event($k,0,2,[UIntPtr]::Zero) } }
function Shot($name) {
  $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $bmp = New-Object Drawing.Bitmap $b.Width, $b.Height
  [Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.Location, [Drawing.Point]::Empty, $b.Size)
  $small = New-Object Drawing.Bitmap $bmp, ([int]($b.Width / 2.5)), ([int]($b.Height / 2.5))
  $ms = New-Object IO.MemoryStream; $small.Save($ms, [Drawing.Imaging.ImageFormat]::Png)
  "SHOT $Scenario-$name " + [Convert]::ToBase64String($ms.ToArray())
}

$mock = 'http://10.211.55.2:8766'
$configDir = Join-Path $env:APPDATA 'com.heyflitty.desktop'
Remove-Item (Join-Path $configDir 'settings.json') -ErrorAction SilentlyContinue
$credential = 'openai-api-key.com.heyflitty.desktop'
cmdkey /delete:$credential 2>&1 | Out-Null
if ($Scenario -eq 'reply') { cmdkey /generic:$credential /user:openai-api-key /pass:mock-test-key | Out-Null }

# Launch in the interactive session with the test transcript set.
$exe = Join-Path $env:USERPROFILE 'heyflitty\src-tauri\target\debug\flitty.exe'
$log = Join-Path $env:TEMP 'flitty.log'
Get-Process flitty -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 500
$command = "/c set FLITTY_TEST_TRANSCRIPT=where do I search on this page&& set FLITTY_TEST_OPENAI_BASE_URL=$mock&& `"$exe`" > `"$log`" 2>&1"
$action = New-ScheduledTaskAction -Execute 'cmd.exe' -Argument $command -WorkingDirectory (Split-Path $exe)
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -LogonType Interactive
Register-ScheduledTask -TaskName 'flitty' -Action $action -Principal $principal -Settings (New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero)) -Force | Out-Null
Start-ScheduledTask -TaskName 'flitty'; Start-Sleep -Seconds 5

$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
[K]::SetCursorPos([int]($screen.Width * 0.45), [int]($screen.Height * 0.6)) | Out-Null; Start-Sleep -Milliseconds 500
PushToTalk 700
Start-Sleep -Milliseconds 600
Shot 'thinking'
$deadline = (Get-Date).AddSeconds(15)
while ((Get-Date) -lt $deadline -and -not (Select-String -Path $log -Pattern 'reply complete|API key' -Quiet)) { Start-Sleep -Milliseconds 200 }
Start-Sleep -Milliseconds 1300
Shot 'pointing'
Start-Sleep -Seconds 2
if ($Scenario -eq 'no-key') { Shot 'settings-prompt' }
cmdkey /delete:$credential 2>&1 | Out-Null
"LOG " + ((Get-Content $log | Where-Object { $_ -match 'first sentence|reply complete|captured|API key|failed|error' }) -join ' || ')
