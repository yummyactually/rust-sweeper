# Shared build-environment helpers for the Minesweeper Rust project.
# Dot-source this file from the public scripts. This file is intentionally
# ASCII-only so Windows PowerShell 5.1 reads it correctly on any code page.

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

function Get-ApplicationPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $command = Get-Command -Name $Name -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($null -eq $command) {
        return $null
    }
    return $command.Source
}

function Find-VcVars64 {
    [CmdletBinding()]
    param(
        [string]$ExplicitPath
    )

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        if (-not (Test-Path -LiteralPath $ExplicitPath -PathType Leaf)) {
            throw "The vcvars64.bat path passed with -VcVarsPath does not exist: $ExplicitPath"
        }
        return (Resolve-Path -LiteralPath $ExplicitPath).Path
    }

    $overridePath = [Environment]::GetEnvironmentVariable('VCVARS64', 'Process')
    if (-not [string]::IsNullOrWhiteSpace($overridePath)) {
        if (-not (Test-Path -LiteralPath $overridePath -PathType Leaf)) {
            throw "The VCVARS64 environment variable points to a missing file: $overridePath"
        }
        return (Resolve-Path -LiteralPath $overridePath).Path
    }

    $candidates = New-Object System.Collections.Generic.List[string]

    # vswhere is the supported way to discover Visual Studio installations,
    # including Build Tools installed outside Program Files.
    $programFilesX86 = [Environment]::GetEnvironmentVariable('ProgramFiles(x86)', 'Process')
    $programFiles = [Environment]::GetEnvironmentVariable('ProgramFiles', 'Process')
    $vswhereCandidates = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($programFilesX86)) {
        [void]$vswhereCandidates.Add((Join-Path $programFilesX86 'Microsoft Visual Studio\Installer\vswhere.exe'))
    }
    if (-not [string]::IsNullOrWhiteSpace($programFiles)) {
        [void]$vswhereCandidates.Add((Join-Path $programFiles 'Microsoft Visual Studio\Installer\vswhere.exe'))
    }

    foreach ($vswhere in $vswhereCandidates) {
        if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
            continue
        }

        $installations = @(& $vswhere -all -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null)
        $vswhereExitCode = $LASTEXITCODE
        if ($vswhereExitCode -ne 0) {
            continue
        }

        foreach ($installation in $installations) {
            $installation = ([string]$installation).Trim()
            if ([string]::IsNullOrWhiteSpace($installation)) {
                continue
            }
            [void]$candidates.Add((Join-Path $installation 'VC\Auxiliary\Build\vcvars64.bat'))
        }
    }

    # Also honor an already-known VC installation and check conventional paths.
    $vcInstallDir = [Environment]::GetEnvironmentVariable('VCINSTALLDIR', 'Process')
    if (-not [string]::IsNullOrWhiteSpace($vcInstallDir)) {
        [void]$candidates.Add((Join-Path $vcInstallDir 'Auxiliary\Build\vcvars64.bat'))
    }

    $visualStudioRoots = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($programFilesX86)) {
        [void]$visualStudioRoots.Add($programFilesX86)
    }
    if (-not [string]::IsNullOrWhiteSpace($programFiles)) {
        [void]$visualStudioRoots.Add($programFiles)
    }
    $editions = @('BuildTools', 'Community', 'Professional', 'Enterprise', 'Preview')
    foreach ($root in $visualStudioRoots) {
        foreach ($year in @('2022', '2019', '2017')) {
            foreach ($edition in $editions) {
                [void]$candidates.Add((Join-Path $root ("Microsoft Visual Studio\{0}\{1}\VC\Auxiliary\Build\vcvars64.bat" -f $year, $edition)))
            }
        }
    }

    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }

    throw @"
Could not locate vcvars64.bat.

Install Visual Studio 2022 Build Tools with the "Desktop development with C++"
workload (MSVC x64 build tools and a Windows 10/11 SDK), or set VCVARS64 to
the full path of vcvars64.bat, for example:

  $env:VCVARS64 = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'

Then rerun this script.
"@
}

function Import-VcVars64 {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$VcVarsPath
    )

    # Run vcvars64.bat in cmd.exe and import the resulting process environment.
    # This is the PowerShell equivalent of starting an x64 Native Tools prompt.
    $commandLine = 'call "' + $VcVarsPath + '" >nul && set'
    $rawLines = @(& $env:ComSpec /d /c $commandLine 2>&1)
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "vcvars64.bat failed with exit code $exitCode. Path: $VcVarsPath"
    }
    if ($rawLines.Count -eq 0) {
        throw "vcvars64.bat produced no environment output. Path: $VcVarsPath"
    }

    $importedCount = 0
    $sawPath = $false
    foreach ($rawLine in $rawLines) {
        $line = ([string]$rawLine).TrimEnd([char[]]@([char]13, [char]10))
        if ([string]::IsNullOrEmpty($line)) {
            continue
        }

        $equalsAt = $line.IndexOf('=')
        if ($equalsAt -le 0) {
            # cmd.exe emits drive-current-directory pseudo variables such as
            # "=C:=C:\path"; they are not real process environment variables.
            continue
        }

        $name = $line.Substring(0, $equalsAt)
        $value = $line.Substring($equalsAt + 1)
        [Environment]::SetEnvironmentVariable($name, $value, 'Process')
        $importedCount++
        if ($name -ieq 'PATH') {
            $sawPath = $true
        }
    }

    if ($importedCount -eq 0 -or -not $sawPath) {
        throw "Could not import the Visual Studio environment from vcvars64.bat. Path: $VcVarsPath"
    }
}

function Add-CargoToPath {
    [CmdletBinding()]
    param()

    $userProfile = [Environment]::GetEnvironmentVariable('USERPROFILE', 'Process')
    if ([string]::IsNullOrWhiteSpace($userProfile)) {
        $userProfile = $HOME
    }
    if ([string]::IsNullOrWhiteSpace($userProfile)) {
        throw "USERPROFILE is not set, so the Rust Cargo directory cannot be located."
    }

    $userCargoBin = Join-Path $userProfile '.cargo\bin'
    $pathEntries = @($env:PATH -split ';')
    $alreadyPresent = $false
    foreach ($entry in $pathEntries) {
        if ($entry.TrimEnd('\') -ieq $userCargoBin.TrimEnd('\')) {
            $alreadyPresent = $true
            break
        }
    }
    if (-not $alreadyPresent) {
        $env:PATH = $userCargoBin + ';' + $env:PATH
    }

    $cargoPath = Get-ApplicationPath 'cargo.exe'
    if ($null -eq $cargoPath) {
        throw @"
cargo.exe was not found.

Install the Rust toolchain with rustup (the MSVC toolchain is required) and
make sure this file exists:

  $userCargoBin\cargo.exe

The scripts automatically add %USERPROFILE%\.cargo\bin to PATH.
"@
    }

    $cargoDirectory = Split-Path -Parent $cargoPath
    if ($cargoDirectory.TrimEnd('\') -ine $userCargoBin.TrimEnd('\')) {
        $env:PATH = $cargoDirectory + ';' + $env:PATH
    }

    return $cargoPath
}

function Assert-MsvcToolset {
    [CmdletBinding()]
    param()

    $clPath = Get-ApplicationPath 'cl.exe'
    $linkPath = Get-ApplicationPath 'link.exe'
    if ($null -eq $clPath -or $null -eq $linkPath) {
        throw @"
The MSVC compiler toolset is not available after importing vcvars64.bat.

Install the "Desktop development with C++" workload in Visual Studio Build
Tools, including the MSVC v143 x64/x86 build tools and a Windows SDK. Then
start a new PowerShell window and rerun this script.
"@
    }

    return [pscustomobject]@{
        Cl   = $clPath
        Link = $linkPath
    }
}

function Initialize-BuildEnvironment {
    [CmdletBinding()]
    param(
        [string]$VcVarsPath
    )

    $resolvedVcVars = Find-VcVars64 -ExplicitPath $VcVarsPath
    Import-VcVars64 -VcVarsPath $resolvedVcVars
    $cargoPath = Add-CargoToPath
    $tools = Assert-MsvcToolset

    return [pscustomobject]@{
        VcVars64 = $resolvedVcVars
        Cargo    = $cargoPath
        Cl       = $tools.Cl
        Link     = $tools.Link
    }
}

function Invoke-CargoCommand {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$CargoPath,
        [Parameter(Mandatory = $true)]
        [string[]]$CargoArguments
    )

    # Cargo writes most progress and diagnostics to stderr. Temporarily allow
    # non-terminating errors and replay each line through Write-Host so Windows
    # PowerShell 5.1 does not hide the diagnostics when ErrorActionPreference is
    # Stop. The native exit code is preserved and returned to the caller.
    $previousPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $CargoPath @CargoArguments 2>&1 | ForEach-Object {
            if ($_ -is [System.Management.Automation.ErrorRecord]) {
                $message = $_.Exception.Message
                if (-not [string]::IsNullOrWhiteSpace($message)) {
                    Write-Host $message
                }
            }
            else {
                Write-Host ([string]$_)
            }
        }
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }

    return [int]$exitCode
}

function Write-BuildEnvironment {
    param(
        [Parameter(Mandatory = $true)]
        $EnvironmentInfo
    )

    Write-Host ("[env] vcvars64 : " + $EnvironmentInfo.VcVars64)
    Write-Host ("[env] cargo     : " + $EnvironmentInfo.Cargo)
    Write-Host ("[env] cl        : " + $EnvironmentInfo.Cl)
    Write-Host ("[env] link      : " + $EnvironmentInfo.Link)
}