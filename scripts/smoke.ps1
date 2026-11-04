# Launch the packaged game, verify that a real window appears, then close it
# through WM_CLOSE (MainWindow.CloseMainWindow) and verify a clean exit.
[CmdletBinding()]
param(
    [string]$ExePath,
    [ValidateRange(1, 120)]
    [int]$WindowTimeoutSeconds = 20,
    [ValidateRange(1, 60)]
    [int]$CloseTimeoutSeconds = 5,
    [string]$VcVarsPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
$repoRoot = Split-Path -Parent $PSScriptRoot
$process = $null

try {
    . (Join-Path $PSScriptRoot 'common.ps1')
    $environmentInfo = Initialize-BuildEnvironment -VcVarsPath $VcVarsPath
    Write-BuildEnvironment -EnvironmentInfo $environmentInfo

    if ([string]::IsNullOrWhiteSpace($ExePath)) {
        $ExePath = Join-Path $repoRoot 'dist\minesweeper.exe'
    }
    elseif (-not [System.IO.Path]::IsPathRooted($ExePath)) {
        $ExePath = Join-Path $repoRoot $ExePath
    }
    $fullExePath = [System.IO.Path]::GetFullPath($ExePath)
    if (-not (Test-Path -LiteralPath $fullExePath -PathType Leaf)) {
        throw "Executable not found: $fullExePath. Run .\scripts\package.ps1 first."
    }

    $exe = Get-Item -LiteralPath $fullExePath
    if ($exe.Length -le 0) {
        throw "Executable is empty: $fullExePath"
    }

    Write-Host ("[smoke] executable: " + $exe.FullName)
    Write-Host ("[smoke] size      : " + $exe.Length + " bytes")

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $exe.FullName
    $startInfo.WorkingDirectory = Split-Path -Parent $exe.FullName
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $false

    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) {
        throw "Process.Start returned no process object."
    }
    Write-Host ("[smoke] started pid=" + $process.Id)

    $deadline = [DateTime]::UtcNow.AddSeconds($WindowTimeoutSeconds)
    $windowHandle = [IntPtr]::Zero
    while ([DateTime]::UtcNow -lt $deadline) {
        Start-Sleep -Milliseconds 200
        if ($process.HasExited) {
            throw ("Process exited during startup with exit code " + $process.ExitCode + ".")
        }
        $process.Refresh()
        $windowHandle = $process.MainWindowHandle
        if ($windowHandle -ne [IntPtr]::Zero -and -not [string]::IsNullOrWhiteSpace($process.MainWindowTitle)) {
            break
        }
    }

    if ($windowHandle -eq [IntPtr]::Zero) {
        throw ("No top-level window appeared within " + $WindowTimeoutSeconds + " seconds.")
    }
    if ([string]::IsNullOrWhiteSpace($process.MainWindowTitle)) {
        throw "A window handle appeared, but the window title is empty."
    }

    Start-Sleep -Milliseconds 750
    $process.Refresh()
    if ($process.HasExited) {
        throw ("Process exited after showing a window with exit code " + $process.ExitCode + ".")
    }
    if ($process.MainWindowHandle -eq [IntPtr]::Zero) {
        throw "The process is still running, but its main window disappeared."
    }
    if (-not $process.Responding) {
        throw "The game process is not responding after showing its window."
    }

    Write-Host ("[smoke] window    : hwnd=0x" + $process.MainWindowHandle.ToInt64().ToString('X') + " title=" + $process.MainWindowTitle)
    Write-Host ("[smoke] stability : process alive and responding")
    Write-Host "[smoke] requesting clean close (WM_CLOSE)"

    $closeRequested = $process.CloseMainWindow()
    if (-not $closeRequested) {
        throw "CloseMainWindow returned false; WM_CLOSE could not be posted."
    }
    if (-not $process.WaitForExit($CloseTimeoutSeconds * 1000)) {
        throw ("Process did not exit within " + $CloseTimeoutSeconds + " seconds after WM_CLOSE.")
    }
    if ($process.ExitCode -ne 0) {
        throw ("Process closed with non-zero exit code " + $process.ExitCode + ".")
    }

    Write-Host ("[smoke] exit code : " + $process.ExitCode)
    Write-Host "SMOKE PASS"
    exit 0
}
catch {
    Write-Host ("SMOKE FAIL: " + $_.Exception.Message) -ForegroundColor Red
    if ($null -ne $process -and -not $process.HasExited) {
        try {
            $process.Kill()
            $process.WaitForExit(3000) | Out-Null
        }
        catch {
            Write-Host ("[smoke] cleanup warning: " + $_.Exception.Message) -ForegroundColor Yellow
        }
    }
    exit 1
}