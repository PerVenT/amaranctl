<#
.SYNOPSIS
    Automated visual walkthrough script for recording the amaranctl demo.
.DESCRIPTION
    Runs a choreographed demo with smooth typewriter typing and on-screen hotkey banners
    exercising amaranctl via sub-50ms local IPC.
.PARAMETER PaceSeconds
    Pause in seconds between choreographed steps (default: 1.5).
.PARAMETER DryRun
    Simulate command outputs without executing real commands.
.PARAMETER SkipTypewriter
    Print commands instantly instead of typewriter effect.
.PARAMETER ExePath
    Explicit path to amaranctl executable.
#>
[CmdletBinding()]
param(
    [double]$PaceSeconds = 1.5,
    [switch]$DryRun,
    [switch]$SkipTypewriter,
    [string]$ExePath
)

$ErrorActionPreference = "Stop"

# Locate amaranctl executable
if (-not $ExePath) {
    $candidates = @(
        (Join-Path $HOME ".local\bin\amaranctl.exe"),
        (Join-Path (Split-Path $PSScriptRoot -Parent) "target\x86_64-pc-windows-gnu\release\amaranctl.exe"),
        (Join-Path (Split-Path $PSScriptRoot -Parent) "target\release\amaranctl.exe")
    )
    $cmd = Get-Command amaranctl -ErrorAction SilentlyContinue
    if ($cmd) { $candidates += $cmd.Source }

    foreach ($cand in $candidates) {
        if ($cand -and (Test-Path $cand)) {
            $ExePath = $cand
            break
        }
    }
}

if (-not $DryRun) {
    if (-not $ExePath -or -not (Test-Path $ExePath)) {
        Write-Error "amaranctl executable not found. Please run 'scripts/install.ps1' or 'make build-windows' first."
    }

    # Verify that the background tray daemon is running
    $daemon = Get-Process -Name "amaranctl" -ErrorAction SilentlyContinue
    if (-not $daemon) {
        Write-Warning "amaranctl daemon is not currently running."
        Write-Warning "Start the daemon with 'amaranctl run' to see the live system tray icon update during recording."
        Start-Sleep -Seconds 2
    }
}

function Write-Typewriter {
    param(
        [string]$Text,
        [int]$CharDelayMs = 28,
        [string]$ForegroundColor = "Yellow"
    )
    if ($SkipTypewriter) {
        Write-Host $Text -ForegroundColor $ForegroundColor
        return
    }
    try {
        $origColor = [Console]::ForegroundColor
        [Console]::ForegroundColor = [ConsoleColor]::$ForegroundColor
        foreach ($char in $Text.ToCharArray()) {
            [Console]::Write($char)
            Start-Sleep -Milliseconds $CharDelayMs
        }
        [Console]::WriteLine()
        [Console]::ForegroundColor = $origColor
    } catch {
        Write-Host $Text -ForegroundColor $ForegroundColor
    }
}

function Show-StepHeader {
    param([string]$Title)
    Write-Host ""
    Write-Host ("-" * 60) -ForegroundColor DarkGray
    Write-Host " [Step] $Title" -ForegroundColor Cyan
    Write-Host ("-" * 60) -ForegroundColor DarkGray
}

try { Clear-Host } catch {}
Write-Host "+----------------------------------------------------------+" -ForegroundColor Cyan
Write-Host "|  amaranctl - BLE Mesh Controller & Tray Demo             |" -ForegroundColor Cyan
Write-Host "|  Sub-50ms Local IPC * Adaptive Tray Icon * Hotkeys       |" -ForegroundColor Cyan
Write-Host "+----------------------------------------------------------+" -ForegroundColor Cyan
Write-Host ""
Start-Sleep -Seconds 1

# Step 1: Query initial status
Show-StepHeader "1. Check current status via IPC"
Write-Typewriter "$ amaranctl status" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Amaran 60x S: OFF (Ready) | Connected" -ForegroundColor Gray
} else {
    & $ExePath status
}
Start-Sleep -Seconds $PaceSeconds

# Step 2: Power On
Show-StepHeader "2. Turn light ON (Tray glows Warm Gold)"
Write-Typewriter "$ amaranctl on" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Turned ON" -ForegroundColor Green
} else {
    & $ExePath on
}
Start-Sleep -Seconds $PaceSeconds

# Step 3: Set Daylight CCT (5600K)
Show-StepHeader "3. Set CCT to Daylight 5600K (Tray shifts to Cool White)"
Write-Typewriter "$ amaranctl cct 5600" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "CCT set to 5600K" -ForegroundColor Green
} else {
    & $ExePath cct 5600
}
Start-Sleep -Seconds $PaceSeconds

# Step 4: Hotkey 1 - Decrease Brightness
Show-StepHeader "4. Global Hotkey: Decrease Brightness (-10%)"
Write-Host "[HOTKEY] Ctrl + Alt + Down (-10% Brightness)" -ForegroundColor Magenta
if ($DryRun) {
    Write-Host "Brightness adjusted by -10.0%" -ForegroundColor Gray
} else {
    & $ExePath brightness -- -10
}
Start-Sleep -Seconds $PaceSeconds

# Step 5: Hotkey 2 - Shift Warmer
Show-StepHeader "5. Global Hotkey: Shift CCT Warmer (-200K)"
Write-Host "[HOTKEY] Ctrl + Alt + Left (-200K Warmer)" -ForegroundColor Magenta
if ($DryRun) {
    Write-Host "CCT adjusted by -200K" -ForegroundColor Gray
} else {
    & $ExePath cct -- -200
}
Start-Sleep -Seconds $PaceSeconds

# Step 6: Query updated live status
Show-StepHeader "6. Query updated live hardware status"
Write-Typewriter "$ amaranctl status" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Amaran 60x S: ON | Brightness: 40.0% | CCT: 5400K" -ForegroundColor Green
} else {
    & $ExePath status
}
Start-Sleep -Seconds $PaceSeconds

# Step 7: Turn light OFF
Show-StepHeader "7. Turn light OFF (Tray dims to Ready state)"
Write-Typewriter "$ amaranctl off" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Turned OFF" -ForegroundColor DarkGray
} else {
    & $ExePath off
}
Start-Sleep -Seconds $PaceSeconds

Write-Host ""
Write-Host "============================================================" -ForegroundColor Green
Write-Host " Demo sequence completed successfully!                      " -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green
Write-Host ""

