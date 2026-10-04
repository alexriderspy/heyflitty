# One real turn against OpenAI with the key saved in Credential Manager (never read here).
# Shows a page full screen, parks the mouse at a fraction of the screen, asks the transcript, captures the result.
param([string]$Page, [double]$FracX, [double]$FracY, [string]$Question, [string]$Name = 'real')
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

Get-Process flitty, msedge -ErrorAction SilentlyContinue | Stop-Process -Force; Start-Sleep -Milliseconds 800
$profile = Join-Path $env:TEMP "flitty-edge-$(Get-Random)"
Start-Process msedge -ArgumentList "--kiosk $Page --edge-kiosk-type=fullscreen --no-first-run --user-data-dir=`"$profile`""
Start-Sleep -Seconds 5

$exe = Join-Path $env:USERPROFILE 'heyflitty\src-tauri\target\debug\flitty.exe'
$log = Join-Path $env:TEMP "flitty-real-$(Get-Date -Format yyyyMMddHHmmss).log"
$command = "/c set FLITTY_TEST_TRANSCRIPT=$Question&& `"$exe`" > `"$log`" 2>&1"
Start-Process cmd.exe -ArgumentList $command -WorkingDirectory (Split-Path $exe) -WindowStyle Hidden
Start-Sleep -Seconds 6

$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
[K]::SetCursorPos([int]($screen.Width * $FracX), [int]($screen.Height * $FracY)) | Out-Null; Start-Sleep -Milliseconds 600
Shot 'before'
PushToTalk 700
$deadline = (Get-Date).AddSeconds(30)
while ((Get-Date) -lt $deadline -and -not (Select-String -Path $log -Pattern 'reply complete|failed|error' -Quiet)) { Start-Sleep -Milliseconds 200 }
Start-Sleep -Milliseconds 1500
Shot 'pointing'
Start-Sleep -Milliseconds 2500
Shot 'later'
"LOG " + ((Get-Content $log) -join " || ")
Get-Process flitty, msedge -ErrorAction SilentlyContinue | Stop-Process -Force
