# Build the optimized Windows executable and copy it to dist\minesweeper.exe.
[CmdletBinding()]
param(
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

    Push-Location -LiteralPath $repoRoot
    $pushedLocation = $true
    Write-Host '[package] cargo build --release --bin minesweeper'
    $cargoArguments = @('build', '--release', '--bin', 'minesweeper')
    $exitCode = Invoke-CargoCommand -CargoPath $environmentInfo.Cargo -CargoArguments $cargoArguments
    if ($exitCode -ne 0) {
        throw "cargo build --release failed with exit code $exitCode."
    }

    $sourcePath = Join-Path $repoRoot 'target\release\minesweeper.exe'
    if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
        throw "Release executable was not found: $sourcePath"
    }

    $distDirectory = Join-Path $repoRoot 'dist'
    $destinationPath = Join-Path $distDirectory 'minesweeper.exe'
    New-Item -ItemType Directory -Path $distDirectory -Force | Out-Null
    Copy-Item -LiteralPath $sourcePath -Destination $destinationPath -Force

    if (-not (Test-Path -LiteralPath $destinationPath -PathType Leaf)) {
        throw "Package copy failed; file is missing: $destinationPath"
    }

    $source = Get-Item -LiteralPath $sourcePath
    $destination = Get-Item -LiteralPath $destinationPath
    if ($destination.Length -le 0) {
        throw "Packaged executable is empty: $destinationPath"
    }
    if ($source.Length -ne $destination.Length) {
        throw "Packaged executable size differs from the release binary."
    }

    $hash = Get-FileHash -LiteralPath $destinationPath -Algorithm SHA256
    Write-Host ("[package] executable: " + $destination.FullName)
    Write-Host ("[package] size      : " + $destination.Length + " bytes")
    Write-Host ("[package] sha256    : " + $hash.Hash)
    Write-Host "PACKAGE PASS"
    exit 0
}
catch {
    Write-Host ("PACKAGE FAIL: " + $_.Exception.Message) -ForegroundColor Red
    exit 1
}
finally {
    if ($pushedLocation) {
        Pop-Location
    }
}