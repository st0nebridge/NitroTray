<#
@module dev-run
@description Development launcher: runs NitroTray + a console helper from copies of the debug build
             (so `cargo build` is never blocked by locked executables), on a private dev pipe and a
             private config directory.

Usage:  powershell -NoProfile -ExecutionPolicy Bypass -File bin/dev-run.ps1 [-Real] [-Stop] [-Release]
  -Real     helper talks to the real Acer firmware; it is started elevated via gsudo (console mode,
            not installed as a service). Without -Real the helper uses the simulated backend.
  -Stop     ask a running dev tray to exit (--exit) and stop the dev helper.
  -Release  use target\release instead of target\debug.
#>
param([switch]$Real, [switch]$Stop, [switch]$Release)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$runDir = Join-Path $root 'target\devrun'
$pipe = '\\.\pipe\NitroTrayDev'
$configDir = Join-Path $root 'target\devrun\config'
# Scopes the mutex and show/exit events so this dev instance never touches an installed NitroTray.
$env:NITROTRAY_INSTANCE = 'dev'

function Stop-Dev {
    $tray = Join-Path $runDir 'NitroTray.exe'
    if (Test-Path $tray) {
        $p = Start-Process -FilePath $tray -ArgumentList '--exit' -PassThru -WindowStyle Hidden
        $p.WaitForExit(5000) | Out-Null
    }
    Start-Sleep -Milliseconds 800
    Get-Process NitroTray -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$runDir*" } | Stop-Process -Force
    # An elevated helper's Path reads as null from a normal process, so match by name — but never touch
    # an installed NitroTrayHelper service.
    $servicePid = (Get-CimInstance Win32_Service -Filter "Name='NitroTrayHelper'" -ErrorAction SilentlyContinue).ProcessId
    $helpers = Get-Process NitroTrayService -ErrorAction SilentlyContinue |
        Where-Object { $_.Id -ne $servicePid -and ((-not $_.Path) -or $_.Path -like "$runDir*") }
    foreach ($h in $helpers) {
        try { Stop-Process -Id $h.Id -Force -ErrorAction Stop } catch { gsudo taskkill /PID $h.Id /F | Out-Null }
    }
}

Stop-Dev
if ($Stop) { Write-Output 'dev instance stopped'; exit 0 }

$profileDir = if ($Release) { 'release' } else { 'debug' }
New-Item -ItemType Directory -Force -Path $runDir, $configDir | Out-Null
foreach ($name in 'NitroTray.exe', 'NitroTrayService.exe') {
    Copy-Item -Force (Join-Path $root "target\$profileDir\$name") (Join-Path $runDir $name)
}
$helperExe = Join-Path $runDir 'NitroTrayService.exe'
if ($Real) {
    Start-Process -FilePath 'gsudo' -ArgumentList @('--new', $helperExe, 'console', '--pipe', $pipe) -WindowStyle Minimized
} else {
    Start-Process -FilePath $helperExe -ArgumentList @('console', '--simulate', '--pipe', $pipe) -WindowStyle Hidden
}
for ($i = 0; $i -lt 40 -and -not ([System.IO.Directory]::GetFiles('\\.\pipe\') -contains $pipe); $i++) { Start-Sleep -Milliseconds 250 }

$env:NITROTRAY_PIPE = $pipe
$env:NITROTRAY_DEV_HELPER = '1'
$env:NITROTRAY_CONFIG_DIR = $configDir
Start-Process -FilePath (Join-Path $runDir 'NitroTray.exe') -WindowStyle Hidden
Write-Output "dev instance started (helper: $(if ($Real) { 'real firmware, elevated' } else { 'simulated' }))"
