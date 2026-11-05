# scripts/install.ps1 - Automated installer for amaranctl on Windows
# Clean, fast installer with zero dependencies.

$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "       amaranctl Windows Installer        " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Setup installation directories
Write-Host "[1/3] Setting up amaranctl executable..." -ForegroundColor Yellow
$installDir = "$env:LOCALAPPDATA\Programs\amaranctl"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}
$targetExe = "$installDir\amaranctl.exe"

$localExe = Join-Path $PSScriptRoot "..\amaranctl.exe"
if (Test-Path $localExe) {
    Copy-Item -Path $localExe -Destination $targetExe -Force
    Write-Host "    Installed binary: $targetExe" -ForegroundColor Green
} else {
    Write-Host "    Downloading latest amaranctl.exe from GitHub releases..." -ForegroundColor Cyan
    $repo = "amaranctl/amaranctl"
    $downloadUrl = "https://github.com/$repo/releases/latest/download/amaranctl-windows.exe"
    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $targetExe -UseBasicParsing
        Write-Host "    Downloaded binary: $targetExe" -ForegroundColor Green
    } catch {
        Write-Host "    [!] Could not download automatically. Please place amaranctl.exe in $installDir" -ForegroundColor Yellow
    }
}

# Also update ~/.local/bin if directory exists
$localBin = "$HOME\.local\bin"
if (Test-Path $localBin) {
    Copy-Item -Path $targetExe -Destination "$localBin\amaranctl.exe" -Force -ErrorAction SilentlyContinue
}

# Add to User PATH if missing
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path = "$env:Path;$installDir"
    Write-Host "    Added $installDir to User PATH." -ForegroundColor Green
}

# 2. Autostart setup
Write-Host "[2/3] Autostart configuration..." -ForegroundColor Yellow
$choice = Read-Host "Start amaranctl automatically when you log into Windows? (Y/n)"
if ($choice -eq "" -or $choice.ToLower() -eq "y") {
    $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    Set-ItemProperty -Path $regPath -Name "amaranctl" -Value "`"$targetExe`""
    Write-Host "    Autostart enabled in Windows Registry." -ForegroundColor Green
}

# 3. First-run bootstrap check
Write-Host "[3/3] Configuration ready." -ForegroundColor Yellow
Write-Host "    amaranctl automatically detects and imports credentials from Amaran Desktop on first run." -ForegroundColor Gray

Write-Host ""
Write-Host "==========================================" -ForegroundColor Green
Write-Host "        Installation Successful!          " -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green
Write-Host "Binary:  $targetExe"
Write-Host ""
Write-Host "Usage:"
Write-Host "  Tray app:  Run 'amaranctl' or double-click amaranctl.exe"
Write-Host "  CLI:       amaranctl on | off | toggle | brightness 50 | cct 4500"
Write-Host "  Hotkeys:   Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
Write-Host ""
