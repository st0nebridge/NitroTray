<#
@module build
@description Builds NitroTray.exe and NitroTrayService.exe (release by default) and syntax-checks the web UI.

Usage:  powershell -NoProfile -File bin/build.ps1 [-Debug]
Exit:   non-zero if cargo or the JS syntax check fails.
#>
param([switch]$Debug)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

$profileArgs = @()
if (-not $Debug) {
    $profileArgs = @('--release')
    # Release binaries carry no build-machine paths: dependencies' panic locations would otherwise embed
    # the Cargo home (inside the user's profile), and the crate's own would embed this checkout.
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $remap = @("--remap-path-prefix=$cargoHome=cargo", "--remap-path-prefix=$((Get-Location).Path)=nitrotray")
    $existing = if ($env:CARGO_ENCODED_RUSTFLAGS) { @($env:CARGO_ENCODED_RUSTFLAGS -split [char]0x1f) }
                elseif ($env:RUSTFLAGS) { @($env:RUSTFLAGS -split '\s+' | Where-Object { $_ }) }
                else { @() }
    $env:CARGO_ENCODED_RUSTFLAGS = (@($existing) + $remap) -join [char]0x1f
}
cargo build @profileArgs --bins
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Get-ChildItem src/ui/web -Filter *.js -ErrorAction SilentlyContinue | ForEach-Object {
    node --check $_.FullName
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
Write-Output 'build: ok'
