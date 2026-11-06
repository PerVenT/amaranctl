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
#>
[CmdletBinding()]
param(
    [double]$PaceSeconds = 1.5,
    [switch]$DryRun,
    [switch]$SkipTypewriter
)

$ErrorActionPreference = "Stop"

$ExePath = Join-Path $HOME ".local\bin\amaranctl.exe"
if (-not (Test-Path $ExePath) -and -not $DryRun) {
    Write-Error "amaranctl executable not found at $ExePath. Run 'scripts/install.ps1' first."
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
    Write-Host ("─" * 60) -ForegroundColor DarkGray
    Write-Host " ▶ $Title" -ForegroundColor Cyan
    Write-Host ("─" * 60) -ForegroundColor DarkGray
}

try { Clear-Host } catch {}
Write-Host "┌──────────────────────────────────────────────────────────┐" -ForegroundColor Cyan
Write-Host "│  amaranctl — BLE Mesh Controller & Tray Demo             │" -ForegroundColor Cyan
Write-Host "│  Sub-50ms Local IPC • Adaptive Tray Icon • Hotkeys       │" -ForegroundColor Cyan
Write-Host "└──────────────────────────────────────────────────────────┘" -ForegroundColor Cyan
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
    Write-Host "✓ Turned ON" -ForegroundColor Green
} else {
    & $ExePath on
}
Start-Sleep -Seconds $PaceSeconds

# Step 3: Set Daylight CCT (5600K)
Show-StepHeader "3. Set CCT to Daylight 5600K (Tray shifts to Cool White)"
Write-Typewriter "$ amaranctl cct 5600" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "✓ CCT set to 5600K" -ForegroundColor Green
} else {
    & $ExePath cct 5600
}
Start-Sleep -Seconds $PaceSeconds

# Step 4: Hotkey 1 - Decrease Brightness
Show-StepHeader "4. Global Hotkey: Decrease Brightness (-10%)"
Write-Host "⌨️  [HOTKEY PRESSED]  Ctrl + Alt + Down" -ForegroundColor Magenta
if ($DryRun) {
    Write-Host "✓ Adjusted brightness: -10%" -ForegroundColor Gray
} else {
    & $ExePath brightness -- -10
}
Start-Sleep -Seconds $PaceSeconds

# Step 5: Hotkey 2 - Shift Warmer
Show-StepHeader "5. Global Hotkey: Shift CCT Warmer (-200K)"
Write-Host "⌨️  [HOTKEY PRESSED]  Ctrl + Alt + Left" -ForegroundColor Magenta
if ($DryRun) {
    Write-Host "✓ Adjusted CCT: -200K" -ForegroundColor Gray
} else {
    & $ExePath cct -- -200
}
Start-Sleep -Seconds $PaceSeconds

# Step 6: Query updated live status
Show-StepHeader "6. Query updated live hardware status"
Write-Typewriter "$ amaranctl status" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "Amaran 60x S: 🟢 ON | 40% | 5400K" -ForegroundColor Green
} else {
    & $ExePath status
}
Start-Sleep -Seconds $PaceSeconds

# Step 7: Turn light OFF
Show-StepHeader "7. Turn light OFF (Tray dims to Ready state)"
Write-Typewriter "$ amaranctl off" -ForegroundColor Yellow
if ($DryRun) {
    Write-Host "✓ Turned OFF" -ForegroundColor DarkGray
} else {
    & $ExePath off
}
Start-Sleep -Seconds $PaceSeconds

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host " ✓ Demo sequence completed successfully!                   " -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
