<#
@module live_webviews
@description Live check that NitroTray's webviews load and its event loop stays alive (CU-20261001-003):
             start-up with the popup shown (popup and details popup pages load, --exit is honoured), then
             a graph icon turned on from the settings window (no details popup while it is open, created
             and loaded once it closes), then the tray click gestures (CU-20261029-001): the notification
             area's own messages are posted to the icon's window, so a single click must open the details
             popup, a second click close it, and a double click end with only the main popup open. Each run
             is isolated: own instance tag, config/WebView2 folder and a local DevTools port; the user's
             running NitroTray and settings are not touched. The gesture checks turn "close on focus loss"
             off: a test process is not the foreground app, and whichever window the user is working in
             would otherwise close the popups mid-check. Focus-loss dismissal is checked on its own.

Usage:  powershell -NoProfile -ExecutionPolicy Bypass -File tests/live/webviews.ps1 [-Exe <NitroTray.exe>] [-Runs 3]
Exit:   0 when every check passes, 1 otherwise.
#>
param([string]$Exe = '', [int]$Runs = 3, [int]$Port = 9460)
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $Exe) { $Exe = Join-Path $root 'target\release\NitroTray.exe' }
if (-not (Test-Path $Exe)) { throw "no build at ${Exe}: run bin/build.ps1 first" }
$cdp = Join-Path $PSScriptRoot 'cdp.mjs'
$script:failures = 0

Add-Type -TypeDefinition @'
using System; using System.Collections.Generic; using System.Runtime.InteropServices; using System.Text;
public static class LiveWin {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
}
// The notification-area icon's hidden window (class tray_icon_app) receives the shell's mouse messages
// as WM_USER + 2002 with the mouse message in lParam; posting them drives the same path a real click does.
public static class LiveTray {
    delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    public static IntPtr[] Windows(uint pid) {
        var found = new List<IntPtr>();
        EnumWindows(delegate(IntPtr h, IntPtr l) {
            uint owner; GetWindowThreadProcessId(h, out owner);
            var name = new StringBuilder(64); GetClassName(h, name, name.Capacity);
            if (owner == pid && name.ToString() == "tray_icon_app") found.Add(h);
            return true;
        }, IntPtr.Zero);
        return found.ToArray();
    }
    public static void Mouse(IntPtr h, string what) {
        uint msg = what == "down" ? 0x0201u : what == "up" ? 0x0202u : 0x0203u;
        LiveWin.PostMessage(h, 6002, IntPtr.Zero, (IntPtr)msg);
    }
}
'@

function Check([string]$Name, [bool]$Ok) {
    Write-Output ('  [{0}] {1}' -f $(if ($Ok) { 'ok' } else { 'FAIL' }), $Name)
    if (-not $Ok) { $script:failures++ }
}
function Loaded([int]$P, [string]$Page) { (node $cdp eval $P $Page 'typeof window.nitro') -eq '"object"' }
function WaitFor([scriptblock]$Cond, [int]$Seconds) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt $Seconds) { if (& $Cond) { return $true }; Start-Sleep -Milliseconds 500 }
    return $false
}

function Start-Isolated([string]$Name, [int]$P, [string]$Config, [string[]]$Arguments = @()) {
    $dir = Join-Path $env:TEMP "nitrotray-live-$Name"
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    New-Item -ItemType Directory -Force $dir | Out-Null
    if ($Config) { Set-Content -Path (Join-Path $dir 'config.json') -Value $Config -Encoding ascii }
    $env:NITROTRAY_INSTANCE = "live$Name"
    $env:NITROTRAY_CONFIG_DIR = $dir
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$P"
    $start = @{ FilePath = $Exe; PassThru = $true }
    if ($Arguments.Count) { $start.ArgumentList = $Arguments }
    [pscustomobject]@{ Process = (Start-Process @start); Dir = $dir }
}

function Stop-Isolated($Run) {
    Start-Process -FilePath $Exe -ArgumentList '--exit'
    $exited = $Run.Process.WaitForExit(30000)
    if (-not $exited) { Stop-Process -Id $Run.Process.Id -Force }
    Start-Sleep -Seconds 1
    Get-CimInstance Win32_Process -Filter "Name='msedgewebview2.exe'" | Where-Object { $_.CommandLine -like "*$($Run.Dir)*" } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
    Remove-Item -Recurse -Force $Run.Dir -ErrorAction SilentlyContinue
    $exited
}

foreach ($i in 1..$Runs) {
    $p = $Port + $i
    Write-Output "start-up $i (popup shown at launch)"
    $run = Start-Isolated "start$i" $p ''
    Check 'popup page loads' (WaitFor { Loaded $p 'popup.html' } 60)
    Check 'flyout page loads' (WaitFor { Loaded $p 'flyout.html' } 30)
    Check '--exit is honoured' (Stop-Isolated $run)
}

$p = $Port + $Runs + 1
Write-Output 'graph icons turned on from settings'
$run = Start-Isolated 'toggle' $p '{"tray_icons":{"cpu":{"enabled":false},"gpu":{"enabled":false}}}'
Check 'popup page loads' (WaitFor { Loaded $p 'popup.html' } 60)
Check 'no details popup while no graph icon is on' (-not ((node $cdp pages $p) -match 'flyout'))
node $cdp eval $p 'popup.html' "window.ipc.postMessage(JSON.stringify({cmd:'open_settings'}))" | Out-Null
Check 'settings window loads' (WaitFor { Loaded $p 'window.html' } 60)
node $cdp eval $p 'window.html' "window.ipc.postMessage(JSON.stringify({cmd:'save_settings',settings:{tray_icons:{cpu:{enabled:true}}}}))" | Out-Null
Check 'setting saved' (WaitFor { (Get-Content (Join-Path $run.Dir 'config.json') -Raw | ConvertFrom-Json).tray_icons.cpu.enabled } 10)
Start-Sleep -Seconds 2
Check 'no details popup while the settings window is open' (-not ((node $cdp pages $p) -match 'flyout'))
$title = 'NitroTray ' + [char]0x2014 + ' NitroSense companion'
[void][LiveWin]::PostMessage([LiveWin]::FindWindow([NullString]::Value, $title), 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
Check 'details popup created and loaded after it closes' (WaitFor { Loaded $p 'flyout.html' } 60)
Check '--exit is honoured' (Stop-Isolated $run)

# One graph icon (CPU) and nothing else, started minimised, so its tray window is the only one and
# both popups begin hidden. A hidden WebView2 reports document.visibilityState 'hidden'.
$p = $Port + $Runs + 2
Write-Output 'tray click gestures on the CPU icon'
function Shown([string]$Page) { (node $cdp eval $p $Page 'document.visibilityState') -eq '"visible"' }
function Click([string[]]$Steps) {
    foreach ($s in $Steps) { [LiveTray]::Mouse($script:tray, $s); Start-Sleep -Milliseconds 60 }
}
$run = Start-Isolated 'clicks' $p '{"general":{"close_on_focus_loss":false},"tray_icons":{"show_app_icon":false,"gpu":{"enabled":false}}}' @('--autostart')
Check 'popup page loads' (WaitFor { Loaded $p 'popup.html' } 60)
Check 'details popup page loads' (WaitFor { Loaded $p 'flyout.html' } 30)
Check 'one tray icon window' (WaitFor { $w = [LiveTray]::Windows([uint32]$run.Process.Id); $script:tray = $w[0]; $w.Count -eq 1 } 30)
Start-Sleep -Seconds 2
Check 'both popups start hidden' (-not (Shown 'popup.html') -and -not (Shown 'flyout.html'))
Click @('down', 'up')
Check 'single click opens the details popup' (WaitFor { Shown 'flyout.html' } 5)
Check 'the main popup stays closed' (-not (Shown 'popup.html'))
Click @('down', 'up')
Check 'a second click closes the details popup' (WaitFor { -not (Shown 'flyout.html') } 5)
# Wait out TOGGLE_GRACE: a click right after a close counts as part of that close, as for the main popup.
Start-Sleep -Milliseconds 600
Click @('down', 'up')
Check 'it opens again' (WaitFor { Shown 'flyout.html' } 5)
node $cdp eval $p 'flyout.html' "window.ipc.postMessage(JSON.stringify({cmd:'close_popup'}))" | Out-Null
Check 'Escape in the details popup closes it' (WaitFor { -not (Shown 'flyout.html') } 5)
Start-Sleep -Milliseconds 600
Click @('down', 'up')
Check 'a click after that opens it' (WaitFor { Shown 'flyout.html' } 5)
Click @('down', 'up', 'double', 'up')
Check 'a double click opens the main popup' (WaitFor { Shown 'popup.html' } 5)
Check 'and closes the details popup' (-not (Shown 'flyout.html'))
Start-Sleep -Seconds 1
Check 'the release that ends the double click leaves it open' (Shown 'popup.html')
Start-Sleep -Milliseconds 600
Click @('down', 'up')
Check 'a single click replaces the main popup with the details popup' (WaitFor { (Shown 'flyout.html') -and -not (Shown 'popup.html') } 5)
Check '--exit is honoured' (Stop-Isolated $run)

# Default settings: another window taking the focus closes the details popup. The settings window is that
# other window; it is opened from the hidden main popup's page, so no second click is involved.
$p = $Port + $Runs + 3
Write-Output 'focus loss closes the details popup'
$run = Start-Isolated 'focus' $p '{"tray_icons":{"show_app_icon":false,"gpu":{"enabled":false}}}' @('--autostart')
Check 'popup page loads' (WaitFor { Loaded $p 'popup.html' } 60)
Check 'details popup page loads' (WaitFor { Loaded $p 'flyout.html' } 30)
Check 'one tray icon window' (WaitFor { $w = [LiveTray]::Windows([uint32]$run.Process.Id); $script:tray = $w[0]; $w.Count -eq 1 } 30)
Start-Sleep -Seconds 2
Click @('down', 'up')
Check 'single click opens the details popup' (WaitFor { Shown 'flyout.html' } 5)
node $cdp eval $p 'popup.html' "window.ipc.postMessage(JSON.stringify({cmd:'open_settings'}))" | Out-Null
Check 'settings window loads' (WaitFor { Loaded $p 'window.html' } 60)
Check 'the details popup closes when the settings window takes focus' (WaitFor { -not (Shown 'flyout.html') } 5)
Check '--exit is honoured' (Stop-Isolated $run)

Write-Output $(if ($script:failures) { "$($script:failures) check(s) failed" } else { 'all checks passed' })
exit [int]($script:failures -gt 0)
