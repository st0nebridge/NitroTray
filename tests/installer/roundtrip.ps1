<#
@module installer_roundtrip
@description Elevated end-to-end check of the NSIS installer on a machine without NitroTray: fresh silent
             install, upgrade over the running helper, service/pipe/registry/ACL checks, silent uninstall,
             then an install with /NOHELPER /NOAUTOSTART and its uninstall. Leaves nothing installed.

Usage:  gsudo powershell -NoProfile -ExecutionPolicy Bypass -File tests/installer/roundtrip.ps1 [-Setup <exe>] [-Log <file>]
Exit:   0 when every check passes, 1 otherwise. Refuses to run over an existing installation.
#>
param([string]$Setup = '', [string]$Log = '')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $Setup) {
    $Setup = Get-ChildItem (Join-Path $root 'dist') -Filter 'NitroTray-*-setup.exe' |
        Sort-Object LastWriteTime | Select-Object -Last 1 -ExpandProperty FullName
}
if (-not $Setup -or -not (Test-Path $Setup)) { throw 'no installer: run bin/package.ps1 first' }
if (-not $Log) { $Log = Join-Path $root 'reports\installer\roundtrip.txt' }

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
if (-not ([Security.Principal.WindowsPrincipal]$identity).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'run elevated (gsudo)'
}

$version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$pf = if ($env:ProgramW6432) { $env:ProgramW6432 } else { $env:ProgramFiles }
$dir = Join-Path $pf 'NitroTray'
$uninstKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\NitroTray'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$runValue = '"' + (Join-Path $dir 'NitroTray.exe') + '" --autostart'
$shortcut = Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\NitroTray.lnk'
$settingsDir = Join-Path $env:LOCALAPPDATA 'NitroTray'

if ((Test-Path $dir) -or (Test-Path $uninstKey) -or (Get-Service NitroTrayHelper -ErrorAction SilentlyContinue)) {
    throw 'NitroTray is already installed; this test only runs where it is not'
}
$settingsBefore = Test-Path $settingsDir
$script:firstModes = $null
$runBefore = (Get-ItemProperty $runKey -Name NitroTray -ErrorAction SilentlyContinue).NitroTray

$results = New-Object System.Collections.Generic.List[object]
function Check([string]$Name, [bool]$Ok, [string]$Detail = '') {
    $results.Add([pscustomobject]@{ Result = $(if ($Ok) { 'PASS' } else { 'FAIL' }); Check = $Name; Detail = $Detail })
}

function Invoke-Setup([string]$Label, [string[]]$Arguments) {
    $p = Start-Process -FilePath $Setup -ArgumentList $Arguments -Wait -PassThru
    Check "$Label exits 0" ($p.ExitCode -eq 0) "exit $($p.ExitCode)"
}

function Invoke-Uninstall([string]$Label) {
    $uninstaller = Join-Path $dir 'Uninstall.exe'
    Check "$Label uninstaller present" (Test-Path $uninstaller)
    if (-not (Test-Path $uninstaller)) { return }
    # The NSIS uninstaller re-launches itself from %TEMP%; wait for the folder to go.
    Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait | Out-Null
    $deadline = (Get-Date).AddSeconds(60)
    while ((Test-Path $dir) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
}

function Get-Helper { Get-CimInstance Win32_Service -Filter "Name='NitroTrayHelper'" }

function Wait-Running {
    $deadline = (Get-Date).AddSeconds(15)
    do {
        $s = Get-Helper
        if ($s -and $s.State -eq 'Running') { return $s }
        Start-Sleep -Milliseconds 300
    } while ((Get-Date) -lt $deadline)
    return Get-Helper
}

function Invoke-Pipe([string]$Request) {
    $client = New-Object System.IO.Pipes.NamedPipeClientStream('.', 'NitroTray', [System.IO.Pipes.PipeDirection]::InOut)
    try {
        $client.Connect(5000)
        $bytes = [Text.Encoding]::UTF8.GetBytes("$Request`n")
        $client.Write($bytes, 0, $bytes.Length)
        $client.Flush()
        return (New-Object System.IO.StreamReader($client, [Text.Encoding]::UTF8)).ReadLine()
    } finally {
        $client.Dispose()
    }
}

# Allow ACEs for broad principals must not carry any write, delete or ownership right.
$broad = 'S-1-1-0', 'S-1-5-11', 'S-1-5-4', 'S-1-5-32-545', 'S-1-5-32-546'
$writeMask = [int64](0x2 -bor 0x4 -bor 0x10 -bor 0x40 -bor 0x100 -bor 0x10000 -bor 0x40000 -bor 0x80000 -bor 0x10000000 -bor 0x40000000)
function Get-BroadWriters([string]$Path) {
    foreach ($ace in (Get-Acl $Path).Access) {
        if ($ace.AccessControlType -ne 'Allow') { continue }
        try { $sid = $ace.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value } catch { continue }
        $rights = [int64][int32]$ace.FileSystemRights
        if ($rights -lt 0) { $rights += 4294967296 }
        if (($broad -contains $sid) -and ($rights -band $writeMask)) { "$($ace.IdentityReference): $($ace.FileSystemRights)" }
    }
}

function Test-Installed([string]$Label, [bool]$Helper, [bool]$Autostart) {
    foreach ($f in 'NitroTray.exe', 'NitroTrayService.exe', 'Uninstall.exe', 'nitrotray.ico', 'LICENSE.txt') {
        Check "$Label file $f" (Test-Path (Join-Path $dir $f))
    }
    Check "$Label Start menu shortcut" (Test-Path $shortcut)
    $arp = Get-ItemProperty $uninstKey -ErrorAction SilentlyContinue
    Check "$Label Apps entry version" ($arp -and $arp.DisplayVersion -eq $version) "$($arp.DisplayVersion)"
    Check "$Label Apps entry uninstaller" ($arp -and $arp.QuietUninstallString -eq ('"' + (Join-Path $dir 'Uninstall.exe') + '" /S')) "$($arp.QuietUninstallString)"
    $writers = @(Get-BroadWriters $dir) + @(Get-BroadWriters (Join-Path $dir 'NitroTrayService.exe'))
    Check "$Label folder is admin-only" ($writers.Count -eq 0) ($writers -join '; ')
    $run = (Get-ItemProperty $runKey -Name NitroTray -ErrorAction SilentlyContinue).NitroTray
    if ($Autostart) { Check "$Label autostart entry" ($run -eq $runValue) "$run" }
    else { Check "$Label no autostart entry" (-not $run) "$run" }
    if ($Helper) {
        $s = Wait-Running
        Check "$Label helper running" ($s -and $s.State -eq 'Running') "$($s.State)"
        Check "$Label helper runs as LocalSystem" ($s -and $s.StartName -eq 'LocalSystem') "$($s.StartName)"
        Check "$Label helper starts automatically" ($s -and $s.StartMode -eq 'Auto') "$($s.StartMode)"
        $image = (Join-Path $dir 'NitroTrayService.exe')
        Check "$Label helper image path" ($s -and $s.PathName -like "*$image*run*") "$($s.PathName)"
        try {
            $reply = Invoke-Pipe '{"op":"get_capabilities"}' | ConvertFrom-Json
            Check "$Label pipe answers get_capabilities" ($reply.ok -eq $true) "status=$($reply.capabilities.status) model=$($reply.capabilities.model)"
            $sensors = Invoke-Pipe '{"op":"get_sensors"}' | ConvertFrom-Json
            Check "$Label pipe answers get_sensors" ($sensors.ok -eq $true) "cpu=$($sensors.sensors.cpu_temp_c) C fan=$($sensors.sensors.cpu_fan_rpm) rpm"
            # Installing or restarting the helper must never change the user's modes.
            $fan = Invoke-Pipe '{"op":"get_fan_state"}' | ConvertFrom-Json
            $perf = Invoke-Pipe '{"op":"get_performance_mode"}' | ConvertFrom-Json
            $modes = "fan=$($fan.fan_state.mode) performance=$($perf.performance.mode)"
            if ($script:firstModes) {
                Check "$Label fan/performance modes unchanged" ($modes -eq $script:firstModes) "$script:firstModes -> $modes"
            } else {
                $script:firstModes = $modes
                Check "$Label reads fan/performance modes" ($fan.ok -and $perf.ok) $modes
            }
        } catch {
            Check "$Label pipe answers" $false $_.Exception.Message
        }
    } else {
        Check "$Label no helper service" (-not (Get-Helper))
    }
}

function Test-Removed([string]$Label) {
    Check "$Label folder removed" (-not (Test-Path $dir))
    Check "$Label service removed" (-not (Get-Helper))
    Check "$Label Apps entry removed" (-not (Test-Path $uninstKey))
    Check "$Label shortcut removed" (-not (Test-Path $shortcut))
    $run = (Get-ItemProperty $runKey -Name NitroTray -ErrorAction SilentlyContinue).NitroTray
    Check "$Label autostart entry removed" (-not $run) "$run"
    Check "$Label settings kept by a silent uninstall" ((Test-Path $settingsDir) -eq $settingsBefore)
}

try {
    Invoke-Setup 'fresh install' @('/S')
    Test-Installed 'fresh' $true $true
    Invoke-Setup 'upgrade over the running helper' @('/S')
    Test-Installed 'upgrade' $true $true
    Invoke-Uninstall 'uninstall'
    Test-Removed 'uninstall'
    Invoke-Setup 'minimal install' @('/S', '/NOHELPER', '/NOAUTOSTART')
    Test-Installed 'minimal' $false $false
    Invoke-Uninstall 'minimal uninstall'
    Test-Removed 'minimal uninstall'
} catch {
    Check 'test script' $false $_.Exception.Message
} finally {
    if (Test-Path (Join-Path $dir 'Uninstall.exe')) { Invoke-Uninstall 'cleanup' }
    if ($runBefore) { Set-ItemProperty $runKey -Name NitroTray -Value $runBefore }
}

$failed = @($results | Where-Object Result -eq 'FAIL').Count
$report = @(
    "NitroTray installer round trip - $(Get-Date -Format s)"
    "setup:  $Setup"
    "sha256: $((Get-FileHash -Algorithm SHA256 $Setup).Hash.ToLowerInvariant())"
    ''
    ($results | Format-Table -AutoSize -Wrap | Out-String -Width 200).TrimEnd()
    ''
    "$($results.Count - $failed) passed, $failed failed"
)
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Log) | Out-Null
$report | Set-Content -Encoding utf8 $Log
$report
if ($failed) { exit 1 }
