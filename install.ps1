<#
.SYNOPSIS
    Builds and installs amaranctl into ~/.local/bin/amaranctl.exe and sets up autostart.
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$StartDaemon
)

$ErrorActionPreference = "Stop"

$ProjectRoot = $PSScriptRoot
$TargetDir = Join-Path $HOME ".local\bin"
$TargetExe = Join-Path $TargetDir "amaranctl.exe"
$BuiltExe = Join-Path $ProjectRoot "target\x86_64-pc-windows-gnu\release\amaranctl.exe"

# 1. Build if not skipped
if (-not $SkipBuild) {
    Write-Host "==> Building Windows release binary via Podman..." -ForegroundColor Cyan
    $BuildScript = Join-Path $ProjectRoot "build\build-windows.sh"
    # Convert Windows path to WSL path
    $WslScriptPath = ($BuildScript -replace '\\', '/') -replace '^([A-Za-z]):', { '/mnt/' + $_.Groups[1].Value.ToLower() }
    wsl -d Debian bash $WslScriptPath
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-Path $BuiltExe)) {
    Write-Error "Built binary not found at $BuiltExe"
}

# 2. Stop running amaranctl daemon if running so file can be overwritten
$Running = Get-Process amaranctl -ErrorAction SilentlyContinue
if ($Running) {
    Write-Host "==> Stopping running amaranctl daemon (PID: $($Running.Id))..." -ForegroundColor Yellow
    Stop-Process -Id $Running.Id -Force
    Start-Sleep -Milliseconds 500
}

# 3. Ensure ~/.local/bin exists and copy binary
if (-not (Test-Path $TargetDir)) {
    New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
}

Write-Host "==> Installing amaranctl to $TargetExe..." -ForegroundColor Cyan
Copy-Item $BuiltExe $TargetExe -Force

# 4. Remove redundant old binaries if present
$OldRootExe = Join-Path $ProjectRoot "amaranctl.exe"
if (Test-Path $OldRootExe) {
    Write-Host "==> Cleaning up redundant root binary $OldRootExe..." -ForegroundColor DarkGray
    Remove-Item $OldRootExe -Force -ErrorAction SilentlyContinue
}
$OldLocalAppExe = Join-Path $env:LOCALAPPDATA "Programs\amaranctl\amaranctl.exe"
if (Test-Path $OldLocalAppExe) {
    Write-Host "==> Cleaning up redundant binary from LocalAppData..." -ForegroundColor DarkGray
    Remove-Item (Split-Path $OldLocalAppExe -Parent) -Recurse -Force -ErrorAction SilentlyContinue
}

# 5. Configure Windows Autostart from ~/.local/bin\amaranctl.exe
Write-Host "==> Registering Windows autostart from $TargetExe run..." -ForegroundColor Cyan
$RegKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$RegVal = "`"$TargetExe`" run"
Set-ItemProperty -Path $RegKey -Name "amaranctl" -Value $RegVal

# 6. Verify installation
$VersionOutput = & $TargetExe --version
Write-Host "==> Successfully installed: $VersionOutput" -ForegroundColor Green
Write-Host "    Binary: $TargetExe"
Write-Host "    Autostart: $RegVal"

# 7. Optionally start daemon
if ($StartDaemon) {
    Write-Host "==> Launching amaranctl daemon..." -ForegroundColor Cyan
    & $TargetExe run
}
