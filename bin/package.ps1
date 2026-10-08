<#
@module package
@description Builds the release binaries and the NSIS installer dist\NitroTray-<version>-setup.exe, then
             writes dist\SHA256SUMS.txt for it.

Usage:  powershell -NoProfile -ExecutionPolicy Bypass -File bin/package.ps1 [-SkipBuild]
Needs:  NSIS 3 (makensis on PATH or in %ProgramFiles(x86)%\NSIS): winget install NSIS.NSIS
Exit:   non-zero if the build or makensis fails (NSIS warnings are errors).
#>
param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$match = Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $match) { throw 'version not found in Cargo.toml' }
$version = $match.Matches[0].Groups[1].Value
$versionNum = ($version -split '[-+]')[0]

if (-not $SkipBuild) {
    & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'build.ps1')
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

$makensis = Get-Command makensis -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Source
if (-not $makensis) {
    $candidate = Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe'
    if (-not (Test-Path $candidate)) { throw 'NSIS not found: winget install NSIS.NSIS' }
    $makensis = $candidate
}

$src = Join-Path $root 'target\release'
foreach ($exe in 'NitroTray.exe', 'NitroTrayService.exe') {
    if (-not (Test-Path (Join-Path $src $exe))) { throw "$exe not found in $src - run bin/build.ps1" }
}

# The license page is a rich-edit control: hand it CRLF line ends.
$staging = Join-Path $root 'target\package'
New-Item -ItemType Directory -Force -Path $staging | Out-Null
$license = Join-Path $staging 'LICENSE.txt'
$text = [IO.File]::ReadAllText((Join-Path $root 'LICENSE')) -replace "`r?`n", "`r`n"
[IO.File]::WriteAllText($license, $text, [Text.Encoding]::ASCII)

$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$name = "NitroTray-$version-setup.exe"
$out = Join-Path $dist $name
& $makensis -V2 -WX "-DVERSION=$version" "-DVERSION_NUM=$versionNum" "-DSRCDIR=$src" "-DLICENSE_FILE=$license" "-DOUTFILE=$out" (Join-Path $root 'src\installer\nitrotray.nsi')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$hash = (Get-FileHash -Algorithm SHA256 $out).Hash.ToLowerInvariant()
[IO.File]::WriteAllText((Join-Path $dist 'SHA256SUMS.txt'), "$hash  $name`n", [Text.Encoding]::ASCII)
Write-Output ("package: {0} ({1:N1} MB)" -f $out, ((Get-Item $out).Length / 1MB))
Write-Output "sha256:  $hash"
