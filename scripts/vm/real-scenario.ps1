# One real turn against OpenAI in a realistic setup: open something, park the mouse, ask.
# Uses the key saved in Credential Manager (never read here) and the debug build's FLITTY_TEST_TRANSCRIPT.
param([string]$Exe, [string]$ArgList = '', [double]$FracX = 0.5, [double]$FracY = 0.5, [string]$Question, [string]$Name = 'scenario', [int]$Settle = 5)
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
function Shot($label) {
  $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds; $bmp = New-Object Drawing.Bitmap $b.Width, $b.Height
  [Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.Location, [Drawing.Point]::Empty, $b.Size)
  $small = New-Object Drawing.Bitmap $bmp, ([int]($b.Width / 2)), ([int]($b.Height / 2))
  $ms = New-Object IO.MemoryStream; $small.Save($ms, [Drawing.Imaging.ImageFormat]::Png)
  "SHOT $Name-$label " + [Convert]::ToBase64String($ms.ToArray())
}
# The VM is not activated and Windows keeps popping an "Activation settings" window over everything.
function CloseNag { Get-Process | Where-Object { $_.MainWindowTitle -match 'Activat' } | ForEach-Object { $_.CloseMainWindow() | Out-Null } }
function CloseAll {
  Get-Process flitty, msedge, notepad, mspaint, SystemSettings, ApplicationFrameHost -ErrorAction SilentlyContinue | Stop-Process -Force
  (New-Object -ComObject Shell.Application).Windows() | ForEach-Object { $_.Quit() }
}

CloseAll; Start-Sleep -Milliseconds 800
$flitty = Join-Path $env:USERPROFILE 'heyflitty\src-tauri\target\debug\flitty.exe'
$log = Join-Path $env:TEMP "flitty-scn-$(Get-Date -Format yyyyMMddHHmmss).log"
Start-Process cmd.exe -ArgumentList "/c set FLITTY_TEST_TRANSCRIPT=$Question&& `"$flitty`" > `"$log`" 2>&1" -WorkingDirectory (Split-Path $flitty) -WindowStyle Hidden
Start-Sleep -Seconds 4
# Open the scenario last so it is the focused window.
if ($ArgList) { Start-Process $Exe -ArgumentList $ArgList } else { Start-Process $Exe }
Start-Sleep -Seconds $Settle
CloseNag; Start-Sleep -Milliseconds 800
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
[K]::SetCursorPos([int]($screen.Width * $FracX), [int]($screen.Height * $FracY)) | Out-Null; Start-Sleep -Milliseconds 600
Shot 'before'
PushToTalk 700
$deadline = (Get-Date).AddSeconds(40)
while ((Get-Date) -lt $deadline -and -not (Select-String -Path $log -Pattern 'reply complete|failed|error' -Quiet)) { Start-Sleep -Milliseconds 200 }
Start-Sleep -Milliseconds 1600
Shot 'pointing'
"LOG " + ((Get-Content $log | Where-Object { $_ -match 'cursor screen|elements listed|reply complete|failed|error|pointed' }) -join ' || ')
CloseAll
