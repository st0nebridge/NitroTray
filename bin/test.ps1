<#
@module test
@description Runs the Rust test suites (unit + regression) and the web UI test suite.

Usage:  powershell -NoProfile -File bin/test.ps1
Exit:   non-zero if any suite fails.
#>
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)

cargo test --all-targets
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (Test-Path tests/ui) {
    $uiTests = Get-ChildItem tests/ui -Filter *.test.mjs -Recurse | ForEach-Object FullName
    if ($uiTests) {
        node --test @uiTests
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }
}
Write-Output 'test: ok'
