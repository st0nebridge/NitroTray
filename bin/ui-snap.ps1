<#
@module ui-snap
@description Renders a UI preview with fixture data in headless Edge at 2x DPR (the same Chromium engine as
             WebView2): the popup, or the CPU/GPU hover flyout. The popup render can be compared with the 2x
             reference image; both feed docs/images.

Usage:  powershell -NoProfile -File bin/ui-snap.ps1 [-View popup|flyout-cpu|flyout-gpu] [-State reference|helper-missing] [-Port 5178]
Needs:  the ui-preview dev server running (node bin/devserver.mjs).
Output: reports/ui/popup@2x.png (popup-<state>@2x.png), or reports/ui/flyout-<cpu|gpu>@2x.png
#>
param([ValidateSet('popup', 'flyout-cpu', 'flyout-gpu')][string]$View = 'popup', [string]$State = 'reference', [int]$Port = 5178)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $root 'reports\ui'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$name = if ($View -ne 'popup') { "$View@2x.png" } elseif ($State -eq 'reference') { 'popup@2x.png' } else { "popup-$State@2x.png" }
$out = Join-Path $outDir $name
$edge = @(
    "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw 'Microsoft Edge not found' }
$profileDir = Join-Path $env:TEMP 'nitrotray-edge-profile'
if ($View -eq 'popup') {
    $url = "http://localhost:$Port/tests/ui/preview/popup.html?mode=ours&bare&state=$State"; $size = '572,440'
} else {
    $url = "http://localhost:$Port/tests/ui/preview/flyout.html?bare&only=$($View.Substring(7))"; $size = '340,214'
}
$edgeArgs = @('--headless=new', '--disable-gpu', '--hide-scrollbars', '--no-first-run', "--user-data-dir=$profileDir",
    '--force-device-scale-factor=2', "--window-size=$size", '--virtual-time-budget=3000', "--screenshot=$out", $url)
$p = Start-Process -FilePath $edge -ArgumentList $edgeArgs -Wait -PassThru -WindowStyle Hidden
if ($p.ExitCode -ne 0 -or -not (Test-Path $out)) { throw "screenshot failed (exit $($p.ExitCode))" }
Write-Output $out
