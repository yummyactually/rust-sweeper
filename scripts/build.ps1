# Build the debug binary by default; use -Release for an optimized build.
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

    $cargoArguments = @('build')
    if ($Release) {
        $cargoArguments += '--release'
    }

    Push-Location -LiteralPath $repoRoot
    $pushedLocation = $true
    Write-Host ("[build] cargo " + ($cargoArguments -join ' '))
    $exitCode = Invoke-CargoCommand -CargoPath $environmentInfo.Cargo -CargoArguments $cargoArguments
    if ($exitCode -ne 0) {
        throw "cargo build failed with exit code $exitCode."
    }

    $profileName = if ($Release) { 'release' } else { 'debug' }
    $binaryPath = Join-Path $repoRoot ("target\{0}\minesweeper.exe" -f $profileName)
    if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
        throw "Cargo reported success, but the expected binary was not found: $binaryPath"
    }
    $binary = Get-Item -LiteralPath $binaryPath
    Write-Host ("[build] executable: " + $binary.FullName + " (" + $binary.Length + " bytes)")
    Write-Host "BUILD PASS"
    exit 0
}
catch {
    Write-Host ("BUILD FAIL: " + $_.Exception.Message) -ForegroundColor Red
    exit 1
}
finally {
    if ($pushedLocation) {
        Pop-Location
    }
}