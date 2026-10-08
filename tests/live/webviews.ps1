<#
@module live_webviews
@description Live check that NitroTray's webviews load and its event loop stays alive (CU-20261001-003):
             start-up with the popup shown (popup and hover flyout pages load, --exit is honoured), then
             hover details turned on from the settings window (no flyout while it is open, created and
             loaded once it closes). Each run is isolated: own instance tag, config/WebView2 folder and a
             local DevTools port; the user's running NitroTray and settings are not touched.

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
using System; using System.Runtime.InteropServices;
public static class LiveWin {
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
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

function Start-Isolated([string]$Name, [int]$P, [string]$Config) {
    $dir = Join-Path $env:TEMP "nitrotray-live-$Name"
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    New-Item -ItemType Directory -Force $dir | Out-Null
    if ($Config) { Set-Content -Path (Join-Path $dir 'config.json') -Value $Config -Encoding ascii }
    $env:NITROTRAY_INSTANCE = "live$Name"
    $env:NITROTRAY_CONFIG_DIR = $dir
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=$P"
    [pscustomobject]@{ Process = (Start-Process -FilePath $Exe -PassThru); Dir = $dir }
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
Write-Output 'hover details turned on from settings'
$run = Start-Isolated 'toggle' $p '{"tray_icons":{"hover_details":false}}'
Check 'popup page loads' (WaitFor { Loaded $p 'popup.html' } 60)
Check 'no flyout while hover details are off' (-not ((node $cdp pages $p) -match 'flyout'))
node $cdp eval $p 'popup.html' "window.ipc.postMessage(JSON.stringify({cmd:'open_settings'}))" | Out-Null
Check 'settings window loads' (WaitFor { Loaded $p 'window.html' } 60)
node $cdp eval $p 'window.html' "window.ipc.postMessage(JSON.stringify({cmd:'save_settings',settings:{tray_icons:{hover_details:true}}}))" | Out-Null
Check 'setting saved' (WaitFor { (Get-Content (Join-Path $run.Dir 'config.json') -Raw | ConvertFrom-Json).tray_icons.hover_details } 10)
Start-Sleep -Seconds 2
Check 'no flyout while the settings window is open' (-not ((node $cdp pages $p) -match 'flyout'))
$title = 'NitroTray ' + [char]0x2014 + ' NitroSense companion'
[void][LiveWin]::PostMessage([LiveWin]::FindWindow([NullString]::Value, $title), 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
Check 'flyout created and loaded after it closes' (WaitFor { Loaded $p 'flyout.html' } 60)
Check '--exit is honoured' (Stop-Isolated $run)

Write-Output $(if ($script:failures) { "$($script:failures) check(s) failed" } else { 'all checks passed' })
exit [int]($script:failures -gt 0)
