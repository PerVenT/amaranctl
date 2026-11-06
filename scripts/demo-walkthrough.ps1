<#
.SYNOPSIS
    Automated visual walkthrough script for recording the amaranctl demo.
.DESCRIPTION
    Runs a choreographed demo with smooth typewriter typing and on-screen hotkey banners
    exercising amaranctl via sub-50ms local IPC.
    Showcases dramatic color temperature shifts (2700K Sunset Gold <-> 6500K Ice Blue)
    and global hotkeys (Ctrl+Alt+Left, Ctrl+Alt+L).
.PARAMETER PaceSeconds
    Pause in seconds between choreographed steps (default: 1.8).
.PARAMETER InteractiveHotkeys
    Prompt user to physically press the hotkeys during the walkthrough.
.PARAMETER DryRun
    Simulate command outputs without executing real commands.
.PARAMETER SkipTypewriter
    Print commands instantly instead of typewriter effect.
.PARAMETER ExePath
    Explicit path to amaranctl executable.
#>
[CmdletBinding()]
param(
    [double]$PaceSeconds = 1.8,
    [switch]$InteractiveHotkeys,
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
    Write-Host ("-" * 62) -ForegroundColor DarkGray
    Write-Host " [Step] $Title" -ForegroundColor Cyan
    Write-Host ("-" * 62) -ForegroundColor DarkGray
}

function Show-HotkeyBanner {
    param(
        [string]$Keys,
        [string]$Action,
        [string]$Effect
    )
    Write-Host ""
    Write-Host "  +----------------------------------------------------------+" -ForegroundColor Magenta
    Write-Host "  |  >>> [GLOBAL HOTKEY TRIGGERED]                           |" -ForegroundColor Magenta
    Write-Host ("  |  Keys   : {0,-46}|" -f $Keys) -ForegroundColor Magenta
    Write-Host ("  |  Action : {0,-46}|" -f $Action) -ForegroundColor DarkGray
    Write-Host ("  |  Tray   : {0,-46}|" -f $Effect) -ForegroundColor Yellow
    Write-Host "  +----------------------------------------------------------+" -ForegroundColor Magenta
    Write-Host ""
}

try { Clear-Host } catch {}
Write-Host "+------------------------------------------------------------+" -ForegroundColor Cyan
Write-Host "|  amaranctl - BLE Mesh Controller & Tray Demo               |" -ForegroundColor Cyan
Write-Host "|  Sub-50ms IPC * Adaptive Tray Icon * Global Hotkeys        |" -ForegroundColor Cyan
Write-Host "+------------------------------------------------------------+" -ForegroundColor Cyan
Write-Host ""
Start-Sleep -Seconds 1

# Step 1: Query initial status (Lamp is OFF -> Gray Icon)
Show-StepHeader "1. Query status via IPC (Tray: Gray / Off)"
Write-Typewriter "$ amaranctl status" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Amaran 60x S: OFF (Ready) | Connected" -ForegroundColor DarkGray
} else {
    & $ExePath status
}
Start-Sleep -Seconds $PaceSeconds

# Step 2: Turn Power ON
Show-StepHeader "2. Turn light ON (Instant sub-50ms BLE Mesh dispatch)"
Write-Typewriter "$ amaranctl on" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Turned ON" -ForegroundColor Green
} else {
    & $ExePath on
}
Start-Sleep -Seconds $PaceSeconds

# Step 3: Extreme Warm CCT (2700K Candlelight Sunset) -> Rich Golden Amber Icon
Show-StepHeader "3. Set Warm Sunset CCT 2700K (Tray glows Vivid Golden Amber)"
Write-Typewriter "$ amaranctl cct 2700" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "CCT set to 2700K (Candlelight / Warm Sunset)" -ForegroundColor Yellow
} else {
    & $ExePath cct 2700
}
Start-Sleep -Seconds $PaceSeconds

# Step 4: Extreme Cool CCT (6500K Ice Blue) -> Crisp Sky Blue Icon
Show-StepHeader "4. Set Cool Daylight CCT 6500K (Tray shifts to Vivid Ice Blue)"
Write-Typewriter "$ amaranctl cct 6500" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "CCT set to 6500K (Cold Sky / Ice Blue)" -ForegroundColor Cyan
} else {
    & $ExePath cct 6500
}
Start-Sleep -Seconds $PaceSeconds

# Step 5: Global Hotkey 1 - Shift Warmer (-1000K via Hotkey)
Show-StepHeader "5. Global Hotkey: Shift CCT Warmer (Ctrl + Alt + Left)"
Show-HotkeyBanner -Keys "[Ctrl] + [Alt] + [Left]" -Action "Shift CCT Warmer (-1000K)" -Effect "Tray shifts from Ice Blue to Warm Gold"
if ($InteractiveHotkeys) {
    Write-Host "  >>> Press [Ctrl + Alt + Left] on your keyboard now (or wait 2s)..." -ForegroundColor Magenta
    Start-Sleep -Seconds 2
}
if ($DryRun) {
    Write-Host "CCT adjusted by -1000K -> 5500K Daylight" -ForegroundColor Green
} else {
    & $ExePath cct -- -1000
}
Start-Sleep -Seconds $PaceSeconds

# Step 6: Global Hotkey 2 - Power Toggle OFF (Ctrl + Alt + L)
Show-StepHeader "6. Global Hotkey: Power Toggle (Ctrl + Alt + L) -> Instant OFF"
Show-HotkeyBanner -Keys "[Ctrl] + [Alt] + [L]" -Action "Toggle Power State" -Effect "Tray icon instantly dims to Gray (OFF)"
if ($InteractiveHotkeys) {
    Write-Host "  >>> Press [Ctrl + Alt + L] on your keyboard now (or wait 2s)..." -ForegroundColor Magenta
    Start-Sleep -Seconds 2
}
if ($DryRun) {
    Write-Host "Turned OFF" -ForegroundColor DarkGray
} else {
    & $ExePath toggle
}
Start-Sleep -Seconds $PaceSeconds

# Step 7: Global Hotkey 2 - Power Toggle ON (Ctrl + Alt + L)
Show-StepHeader "7. Global Hotkey: Power Toggle (Ctrl + Alt + L) -> Instant ON"
Show-HotkeyBanner -Keys "[Ctrl] + [Alt] + [L]" -Action "Toggle Power State" -Effect "Tray icon instantly lights back up"
if ($InteractiveHotkeys) {
    Write-Host "  >>> Press [Ctrl + Alt + L] on your keyboard now (or wait 2s)..." -ForegroundColor Magenta
    Start-Sleep -Seconds 2
}
if ($DryRun) {
    Write-Host "Turned ON" -ForegroundColor Green
} else {
    & $ExePath toggle
}
Start-Sleep -Seconds $PaceSeconds

# Step 8: Query live hardware status
Show-StepHeader "8. Query live status verification"
Write-Typewriter "$ amaranctl status" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Amaran 60x S: ON | Brightness: 100.0% | CCT: 5500K | Connected" -ForegroundColor Green
} else {
    & $ExePath status
}
Start-Sleep -Seconds $PaceSeconds

# Step 9: Turn light OFF (Ready state)
Show-StepHeader "9. Turn light OFF (Clean shutdown)"
Write-Typewriter "$ amaranctl off" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Turned OFF" -ForegroundColor DarkGray
} else {
    & $ExePath off
}
Start-Sleep -Seconds 1

Write-Host ""
Write-Host "==============================================================" -ForegroundColor Green
Write-Host " Demo walkthrough completed successfully!                     " -ForegroundColor Green
Write-Host "==============================================================" -ForegroundColor Green
Write-Host ""
