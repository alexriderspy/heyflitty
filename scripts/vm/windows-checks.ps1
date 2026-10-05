# Automated Windows checks for HeyFlitty. Run in the interactive session while HeyFlitty is running.
# Prints one RESULT line per check and SHOT lines (name + base64 PNG) for visual review.
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
Add-Type @'
using System; using System.Text; using System.Runtime.InteropServices;
public static class Win {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc p, IntPtr l);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern long GetWindowLongPtrW(IntPtr h, int i);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern IntPtr WindowFromPoint(POINT p);
  [DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, UIntPtr extra);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  public struct RECT { public int L, T, R, B; }
  public struct POINT { public int X, Y; }
}
'@
[Win]::SetProcessDPIAware() | Out-Null
$results = New-Object System.Collections.ArrayList
function Result($name, $pass, $detail) { [void]$results.Add(("RESULT {0} {1} {2}" -f ($(if ($pass) { 'PASS' } else { 'FAIL' })), $name, $detail)) }
function ClassOf($h) { $sb = New-Object Text.StringBuilder 256; [Win]::GetClassName($h, $sb, 256) | Out-Null; $sb.ToString() }
function TitleOf($h) { $sb = New-Object Text.StringBuilder 256; [Win]::GetWindowText($h, $sb, 256) | Out-Null; $sb.ToString() }
function TopLevelAt($x, $y) { $p = New-Object Win+POINT; $p.X = $x; $p.Y = $y; [Win]::GetAncestor([Win]::WindowFromPoint($p), 2) }
function Key($vk, $up) { [Win]::keybd_event([byte]$vk, 0, $(if ($up) { 2 } else { 0 }), [UIntPtr]::Zero) }
function PushToTalk($holdMs) { Key 0x11 $false; Key 0x12 $false; Key 0x20 $false; Start-Sleep -Milliseconds $holdMs; Key 0x20 $true; Key 0x12 $true; Key 0x11 $true }
$shots = New-Object System.Collections.ArrayList
function Shot($name) {
  $b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
  $bmp = New-Object Drawing.Bitmap $b.Width, $b.Height
  [Drawing.Graphics]::FromImage($bmp).CopyFromScreen($b.Location, [Drawing.Point]::Empty, $b.Size)
  $small = New-Object Drawing.Bitmap $bmp, ([int]($b.Width / 2.5)), ([int]($b.Height / 2.5))
  $ms = New-Object IO.MemoryStream; $small.Save($ms, [Drawing.Imaging.ImageFormat]::Png)
  [void]$shots.Add("SHOT $name " + [Convert]::ToBase64String($ms.ToArray()))
}
function HeyFlittyWindows {
  $ids = @(Get-Process flitty -ErrorAction SilentlyContinue | ForEach-Object Id)
  $found = New-Object System.Collections.ArrayList
  [Win]::EnumWindows({ param($h, $l) $p = 0; [Win]::GetWindowThreadProcessId($h, [ref]$p) | Out-Null
    if ($ids -contains $p -and [Win]::IsWindowVisible($h)) { [void]$found.Add($h) }; $true }, [IntPtr]::Zero) | Out-Null
  $found
}
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds

# 1. Process and overlay window flags.
$proc = Get-Process flitty -ErrorAction SilentlyContinue
Result 'process-running' ($null -ne $proc) "pid=$($proc.Id)"
$overlay = HeyFlittyWindows | Where-Object { (TitleOf $_) -eq 'HeyFlitty overlay' } | Select-Object -First 1
if ($overlay) {
  $ex = [Win]::GetWindowLongPtrW($overlay, -20); $r = New-Object Win+RECT; [Win]::GetWindowRect($overlay, [ref]$r) | Out-Null
  $flags = @{ TOPMOST = 0x8; TRANSPARENT = 0x20; TOOLWINDOW = 0x80; LAYERED = 0x80000; NOACTIVATE = 0x08000000 }
  $missing = $flags.Keys | Where-Object { ($ex -band $flags[$_]) -eq 0 }
  Result 'overlay-exstyle' ($missing.Count -eq 0) ("exstyle=0x{0:X} missing={1}" -f $ex, ($missing -join ','))
  $covers = $r.L -le $screen.Left -and $r.T -le $screen.Top -and $r.R -ge $screen.Right -and $r.B -ge $screen.Bottom
  Result 'overlay-covers-screen' $covers "rect=($($r.L),$($r.T),$($r.R),$($r.B)) screen=$($screen.Width)x$($screen.Height)"
} else { Result 'overlay-exists' $false 'no visible window titled HeyFlitty overlay' }

# 2. Click-through: hit-testing never lands on the overlay.
$probePoints = @(@(200, 200), @([int]($screen.Width / 2), [int]($screen.Height / 2)), @([int]($screen.Width - 60), [int]($screen.Height - 20)))
$hits = foreach ($pt in $probePoints) { [Win]::SetCursorPos($pt[0], $pt[1]) | Out-Null; Start-Sleep -Milliseconds 100; ClassOf (TopLevelAt $pt[0] $pt[1]) }
Result 'click-through-hit-test' ($hits.Count -eq 3 -and -not ($hits -match 'Tauri')) ("top-level classes under points: " + ($hits -join ' | '))

# 3. Real clicks and typing reach the app underneath.
Get-Process notepad -ErrorAction SilentlyContinue | Stop-Process -Force
$testFile = Join-Path $env:TEMP 'flitty-click-test.txt'; Set-Content $testFile ''
Start-Process notepad.exe $testFile; Start-Sleep -Seconds 3
$np = Get-Process notepad | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
[Win]::SetWindowPos($np.MainWindowHandle, [IntPtr]::Zero, 300, 200, 900, 600, 0x40) | Out-Null; Start-Sleep -Milliseconds 500
[Win]::SetCursorPos(700, 500) | Out-Null; Start-Sleep -Milliseconds 200
[Win]::mouse_event(0x2, 0, 0, 0, [UIntPtr]::Zero); [Win]::mouse_event(0x4, 0, 0, 0, [UIntPtr]::Zero); Start-Sleep -Milliseconds 400
$fg = [Win]::GetForegroundWindow()
Result 'click-reaches-app' ($fg -eq $np.MainWindowHandle) ("foreground after click: " + (ClassOf $fg))
[System.Windows.Forms.SendKeys]::SendWait('flitty-typing-ok'); Start-Sleep -Milliseconds 300
Shot 'notepad-typing'

# 4. Push-to-talk while Notepad is focused: keeps focus, typing continues.
$before = [Win]::GetForegroundWindow()
PushToTalk 1200; Start-Sleep -Milliseconds 300
Shot 'after-release-processing'
Start-Sleep -Milliseconds 1500
Shot 'pointing'
$after = [Win]::GetForegroundWindow()
Result 'ptt-keeps-focus' ($before -eq $after) ("before=" + (ClassOf $before) + " after=" + (ClassOf $after))
[System.Windows.Forms.SendKeys]::SendWait(' still-typing'); Start-Sleep -Milliseconds 300

# 5. Capture timing from HeyFlitty's log.
Start-Sleep -Seconds 3
$captureLines = Get-Content (Join-Path $env:TEMP 'flitty.log') -ErrorAction SilentlyContinue | Select-String 'captured'
Result 'screen-capture' ($captureLines.Count -gt 0) ("log: " + (($captureLines | Select-Object -Last 2) -join ' || '))

# 6. Stays above a topmost window opened after it (Task Manager-style always-on-top).
$np2 = $np.MainWindowHandle
[Win]::SetWindowPos($np2, [IntPtr](-1), 300, 200, 900, 600, 0x40) | Out-Null  # make Notepad topmost too
[Win]::SetForegroundWindow($np2) | Out-Null; Start-Sleep -Seconds 4   # let the buddy finish pointing and fly back
[Win]::SetCursorPos(650, 450) | Out-Null; Start-Sleep -Milliseconds 400
Shot 'over-topmost-notepad'
$overlayNow = HeyFlittyWindows | Where-Object { (TitleOf $_) -eq 'HeyFlitty overlay' } | Select-Object -First 1
Add-Type -Name Z -Namespace Win2 -MemberDefinition '[DllImport("user32.dll")] public static extern System.IntPtr GetWindow(System.IntPtr h, uint cmd);'
$above = $false; $walker = $np2
while ($walker -ne [IntPtr]::Zero) { $walker = [Win2.Z]::GetWindow($walker, 3); if ($walker -eq $overlayNow) { $above = $true; break } }
Result 'above-later-topmost-window' $above 'overlay found above a topmost Notepad activated after it'
[Win]::SetWindowPos($np2, [IntPtr](-2), 0, 0, 0, 0, 0x3) | Out-Null  # back to non-topmost

# 7. Start menu open.
[Win]::SetCursorPos(500, 400) | Out-Null
Key 0x5B $false; Key 0x5B $true; Start-Sleep -Milliseconds 1200
[Win]::SetCursorPos([int]($screen.Width / 2), [int]($screen.Height / 2)) | Out-Null; Start-Sleep -Milliseconds 400
Shot 'start-menu'
Key 0x1B $false; Key 0x1B $true; Start-Sleep -Milliseconds 500

# 8. Borderless full screen (Edge kiosk-style F11).
$edgeProfile = Join-Path $env:TEMP 'flitty-edge-test'
# A throwaway profile runs as its own process tree, so cleanup never touches the user's browser.
Start-Process msedge.exe "--user-data-dir=$edgeProfile --no-first-run --new-window about:blank"; Start-Sleep -Seconds 4
Key 0x7A $false; Key 0x7A $true; Start-Sleep -Seconds 2
[Win]::SetCursorPos(900, 500) | Out-Null; Start-Sleep -Milliseconds 400
Shot 'edge-fullscreen'
Key 0x7A $false; Key 0x7A $true; Start-Sleep -Milliseconds 800
$edgeFg = [Win]::GetForegroundWindow()
PushToTalk 800; Start-Sleep -Milliseconds 400
Result 'ptt-from-edge' ([Win]::GetForegroundWindow() -eq $edgeFg) ("foreground stays " + (ClassOf $edgeFg))

# 9. Not in Alt+Tab / taskbar: overlay must not be an app window.
$appWindows = (HeyFlittyWindows | Where-Object { ([Win]::GetWindowLongPtrW($_, -20) -band 0x40000) -ne 0 }).Count
Result 'not-in-alt-tab' ($appWindows -eq 0) "windows with WS_EX_APPWINDOW: $appWindows"

# 10. Resource use over 10 s idle (HeyFlitty plus its WebView2 children).
$webviewIds = @(Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -match 'com.heyflitty.desktop' } | ForEach-Object ProcessId)
$all = @($proc.Id) + $webviewIds
$cpu0 = (Get-Process -Id $all -ErrorAction SilentlyContinue | Measure-Object CPU -Sum).Sum; Start-Sleep -Seconds 10
$cpu1 = (Get-Process -Id $all -ErrorAction SilentlyContinue | Measure-Object CPU -Sum).Sum
$ram = (Get-Process -Id $all -ErrorAction SilentlyContinue | Measure-Object WorkingSet64 -Sum).Sum / 1MB
$cpuPct = [math]::Round((($cpu1 - $cpu0) / 10) * 100 / [Environment]::ProcessorCount, 1)
Result 'idle-resources' ($cpuPct -lt 5 -and $ram -lt 400) ("cpu=$cpuPct% of machine, ram=$([int]$ram)MB, processes=$($all.Count)")

# 11. Second launch keeps one instance and opens settings.
$exe = (Get-Process -Id $proc.Id).Path
Start-Process $exe; Start-Sleep -Seconds 3
$panelFg = TitleOf ([Win]::GetForegroundWindow())
$count = @(Get-Process flitty -ErrorAction SilentlyContinue).Count
$panel = HeyFlittyWindows | Where-Object { (TitleOf $_) -eq 'HeyFlitty' }
Result 'single-instance' ($count -eq 1) "flitty processes=$count"
Result 'settings-opens' ($null -ne $panel) "panel windows=$(@($panel).Count)"
Result 'settings-in-front' ($panelFg -eq 'HeyFlitty') "foreground title='$panelFg'"
Shot 'settings-panel'

# Cleanup test apps.
Get-Process notepad -ErrorAction SilentlyContinue | Stop-Process -Force
Get-CimInstance Win32_Process -Filter "Name='msedge.exe'" | Where-Object { $_.CommandLine -match 'flitty-edge-test' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
$results
$shots
