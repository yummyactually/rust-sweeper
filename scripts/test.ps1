# Run the Rust test suite, including library unit tests and integration tests.
[CmdletBinding()]
param(
    [switch]$Release,
    [string]$VcVarsPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
$repoRoot = Split-Path -Parent $PSScriptRoot
$pushedLocation = $false

try {
    . (Join-Path $PSScriptRoot 'common.ps1')
    $environmentInfo = Initialize-BuildEnvironment -VcVarsPath $VcVarsPath
    Write-BuildEnvironment -EnvironmentInfo $environmentInfo

    $cargoArguments = @('test')
    if ($Release) {
        $cargoArguments += '--release'
    }

    Push-Location -LiteralPath $repoRoot
    $pushedLocation = $true
    Write-Host ("[test] cargo " + ($cargoArguments -join ' '))
    $exitCode = Invoke-CargoCommand -CargoPath $environmentInfo.Cargo -CargoArguments $cargoArguments
    if ($exitCode -ne 0) {
        throw "cargo test failed with exit code $exitCode."
    }

    Write-Host "TEST PASS"
    exit 0
}
catch {
    Write-Host ("TEST FAIL: " + $_.Exception.Message) -ForegroundColor Red
    exit 1
}
finally {
    if ($pushedLocation) {
        Pop-Location
    }
}