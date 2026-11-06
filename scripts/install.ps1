<#
.SYNOPSIS
    Installs amaranctl on Windows into ~/.local/bin/amaranctl.exe and sets up autostart.
.DESCRIPTION
    Builds the binary via container (Podman in WSL) or uses an existing build / release download,
    copies the executable to ~/.local/bin/amaranctl.exe, adds ~/.local/bin to User PATH if missing,
    and configures Windows autostart.
#>
[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$StartDaemon
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "       amaranctl Windows Installer        " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

$ScriptsDir = $PSScriptRoot
$ProjectRoot = Split-Path $ScriptsDir -Parent
$TargetDir = Join-Path $HOME ".local\bin"
$TargetExe = Join-Path $TargetDir "amaranctl.exe"
$BuiltExe = Join-Path $ProjectRoot "target\x86_64-pc-windows-gnu\release\amaranctl.exe"

# 1. Build if requested and not skipped
if (-not $SkipBuild) {
    $BuildScript = Join-Path $ScriptsDir "build-windows.sh"
    if (Test-Path $BuildScript) {
        Write-Host "[1/4] Building Windows release binary via container..." -ForegroundColor Yellow
        $WslScriptPath = ($BuildScript -replace '\\', '/') -replace '^([A-Za-z]):', { '/mnt/' + $_.Groups[1].Value.ToLower() }
        try {
            wsl -d Debian bash $WslScriptPath
            if ($LASTEXITCODE -ne 0) {
                Write-Warning "Container build returned non-zero code ($LASTEXITCODE)."
            }
        } catch {
            Write-Warning "Could not run container build via WSL: $_"
        }
    }
}

# 2. Check for binary or fallback download
Write-Host "[2/4] Verifying binary..." -ForegroundColor Yellow
if (-not (Test-Path $BuiltExe)) {
    if (Test-Path $TargetExe) {
        Write-Host "    Found existing installed binary at $TargetExe." -ForegroundColor DarkGray
    } else {
        Write-Host "    Local build not found. Attempting download from GitHub releases..." -ForegroundColor Cyan
        $downloadUrl = "https://github.com/amaranctl/amaranctl/releases/latest/download/amaranctl-x86_64-pc-windows-gnu.exe"
        if (-not (Test-Path $TargetDir)) {
            New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
        }
        try {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $TargetExe -UseBasicParsing
            Write-Host "    Downloaded binary to $TargetExe" -ForegroundColor Green
        } catch {
            Write-Error "Binary not found at $BuiltExe and could not download automatically. Please build first using 'make build-windows' or place amaranctl.exe in $TargetDir"
        }
    }
}

# 3. Stop running daemon before updating file
$Running = Get-Process amaranctl -ErrorAction SilentlyContinue
if ($Running) {
    Write-Host "==> Stopping running amaranctl daemon (PID: $($Running.Id))..." -ForegroundColor Yellow
    Stop-Process -Id $Running.Id -Force
    Start-Sleep -Milliseconds 500
}

# 4. Copy binary to ~/.local/bin
if (Test-Path $BuiltExe) {
    if (-not (Test-Path $TargetDir)) {
        New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
    }
    Write-Host "[3/4] Installing amaranctl to $TargetExe..." -ForegroundColor Yellow
    Copy-Item $BuiltExe $TargetExe -Force
}

# 5. Clean up redundant old locations if present
$OldRootExe = Join-Path $ProjectRoot "amaranctl.exe"
if (Test-Path $OldRootExe) {
    Write-Host "    Cleaning up redundant root binary $OldRootExe..." -ForegroundColor DarkGray
    Remove-Item $OldRootExe -Force -ErrorAction SilentlyContinue
}
$OldLocalAppExe = Join-Path $env:LOCALAPPDATA "Programs\amaranctl\amaranctl.exe"
if (Test-Path $OldLocalAppExe) {
    Write-Host "    Cleaning up redundant binary in LocalAppData..." -ForegroundColor DarkGray
    Remove-Item (Split-Path $OldLocalAppExe -Parent) -Recurse -Force -ErrorAction SilentlyContinue
}

# 6. Ensure ~/.local/bin is in User PATH
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$TargetDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$TargetDir", "User")
    $env:Path = "$env:Path;$TargetDir"
    Write-Host "    Added $TargetDir to User PATH." -ForegroundColor Green
}

# 7. Configure Windows Autostart from ~/.local/bin\amaranctl.exe
Write-Host "[4/4] Configuring Windows Autostart..." -ForegroundColor Yellow
$RegKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$RegVal = "`"$TargetExe`" run"
Set-ItemProperty -Path $RegKey -Name "amaranctl" -Value $RegVal
Write-Host "    Autostart registered in registry: $RegVal" -ForegroundColor Green

# 8. Verify installation
try {
    $VersionOutput = & $TargetExe --version
    Write-Host ""
    Write-Host "==========================================" -ForegroundColor Green
    Write-Host "        Installation Successful!          " -ForegroundColor Green
    Write-Host "==========================================" -ForegroundColor Green
    Write-Host "Version:  $VersionOutput"
    Write-Host "Binary:   $TargetExe"
    Write-Host "Autostart: $RegVal"
} catch {
    Write-Warning "Could not execute $($TargetExe): $_"
}

Write-Host ""
Write-Host "Usage:"
Write-Host "  Tray app / daemon:  amaranctl run"
Write-Host "  CLI commands:       amaranctl status"
Write-Host "                      amaranctl on | off | toggle"
Write-Host "                      amaranctl brightness 50"
Write-Host "                      amaranctl cct 4500"
Write-Host "  Hotkeys:            Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
Write-Host ""

if ($StartDaemon) {
    Write-Host "==> Launching amaranctl daemon..." -ForegroundColor Cyan
    Start-Process -FilePath $TargetExe -ArgumentList "run"
}
