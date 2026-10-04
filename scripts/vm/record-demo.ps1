# Records a real Flitty answer on the Windows desktop with ffmpeg (for the README and landing page).
# Uses the saved OpenAI key and FLITTY_TEST_TRANSCRIPT in place of a spoken question.
param([string]$Question = 'where can I see the open issues?', [string]$Url = 'https://github.com/tauri-apps/tauri', [int]$Seconds = 20)
Add-Type @'
using System; using System.Runtime.InteropServices;
public static class K {
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
}
'@
[K]::SetProcessDPIAware() | Out-Null
$ffmpeg = Join-Path $env:USERPROFILE 'spike-setup\ffmpeg\ffmpeg.exe'
$exe = Join-Path $env:USERPROFILE 'heyflitty\src-tauri\target\debug\flitty.exe'
$video = Join-Path $env:TEMP 'flitty-demo.mp4'
$log = Join-Path $env:TEMP "flitty-demo-$(Get-Date -Format yyyyMMddHHmmss).log"
Get-Process flitty, ffmpeg -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item $video -ErrorAction SilentlyContinue

# Open the page in a throwaway Edge profile (no first-run screens), maximized and in front.
$edgeProfile = Join-Path $env:TEMP 'flitty-demo-edge'
# A fresh profile each time avoids Edge's "restore pages" prompt from the last run.
Remove-Item $edgeProfile -Recurse -Force -ErrorAction SilentlyContinue
Start-Process msedge.exe "--user-data-dir=$edgeProfile --no-first-run --no-default-browser-check --start-maximized $Url"; Start-Sleep -Seconds 7
[K]::ShowWindow([K]::GetForegroundWindow(), 3) | Out-Null; Start-Sleep -Seconds 1

$command = "/c set FLITTY_TEST_TRANSCRIPT=$Question&& `"$exe`" > `"$log`" 2>&1"
Start-Process cmd.exe -ArgumentList $command -WorkingDirectory (Split-Path $exe) -WindowStyle Hidden
Start-Sleep -Seconds 5
$screen = Add-Type -AssemblyName System.Windows.Forms -PassThru | Out-Null; $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
[K]::SetCursorPos([int]($bounds.Width * 0.62), [int]($bounds.Height * 0.42)) | Out-Null

$recorder = Start-Process $ffmpeg -ArgumentList "-hide_banner -loglevel error -y -f gdigrab -framerate 24 -draw_mouse 1 -i desktop -t $Seconds -vf scale=1728:-2 -c:v libx264 -preset veryfast -crf 24 -pix_fmt yuv420p `"$video`"" -WindowStyle Hidden -PassThru
Start-Sleep -Milliseconds 1500
foreach ($k in 0x11,0x12,0x20) { [K]::keybd_event($k,0,0,[UIntPtr]::Zero) }
Start-Sleep -Milliseconds 1600
foreach ($k in 0x20,0x12,0x11) { [K]::keybd_event($k,0,2,[UIntPtr]::Zero) }
$recorder.WaitForExit()
Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -match 'flitty-demo-edge' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
"LOG " + ((Get-Content $log) -join ' || ')
"VIDEO $video $((Get-Item $video).Length)"
