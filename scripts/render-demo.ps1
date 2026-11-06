<#
.SYNOPSIS
    Converts and stitches demo recordings into an optimized split-screen GIF and MP4 for README.md.
.DESCRIPTION
    Uses FFmpeg (native or via WSL) to crop the terminal and tray regions, stitch them side-by-side (hstack),
    and generate a high-quality palette-optimized demo.gif (<2MB) and demo.mp4.
    If no input video is provided, generates an animated split-screen demo showcasing
    the CLI and adaptive tray icon transitions.
.PARAMETER InputVideo
    Path to raw recorded video file (e.g. raw_demo.mp4). Accepts -InputFile alias.
.PARAMETER OutputDir
    Directory for generated assets (default: docs/assets).
.PARAMETER Fps
    Frames per second for output GIF (default: 15).
.PARAMETER TotalWidth
    Target width in pixels for composite output (default: 960).
.PARAMETER Direct
    Directly encode the input video without split cropping.
.PARAMETER Sample
    Force generation of synthetic demonstration asset.
#>
[CmdletBinding()]
param(
    [Alias("InputFile", "Input", "File")]
    [string]$InputVideo,
    [string]$OutputDir,
    [int]$Fps = 15,
    [int]$TotalWidth = 960,
    [switch]$Direct,
    [switch]$Sample
)

$ErrorActionPreference = "Stop"

if (-not $OutputDir) {
    $scriptParent = if ($PSScriptRoot) { Split-Path $PSScriptRoot -Parent } else { (Get-Location).Path }
    $OutputDir = Join-Path $scriptParent "docs\assets"
}

if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}

$OutputDir = [System.IO.Path]::GetFullPath($OutputDir)
$OutputGif = Join-Path $OutputDir "demo.gif"
$OutputMp4 = Join-Path $OutputDir "demo.mp4"

if ($InputVideo) {
    if (Test-Path $InputVideo) {
        $InputVideo = (Resolve-Path $InputVideo).Path
    } else {
        Write-Error "Input video file '$InputVideo' not found."
    }
}

# 1. Locate FFmpeg (Host Windows or WSL)
$UseWsl = $false
$WslDistroName = ""

$prevEap = $ErrorActionPreference
$ErrorActionPreference = "SilentlyContinue"
try {
    if (Get-Command ffmpeg -ErrorAction SilentlyContinue) {
        $UseWsl = $false
    } else {
        $wslDistros = wsl -l -q 2>$null | Where-Object { $_ -and $_.Trim() -ne "" }
        if ($wslDistros) {
            $firstDistro = ($wslDistros[0] -replace '\0', '').Trim()
            $checkFfmpeg = wsl -d $firstDistro which ffmpeg 2>$null
            if ($checkFfmpeg) {
                $UseWsl = $true
                $WslDistroName = $firstDistro
            }
        }
    }
} finally {
    $ErrorActionPreference = $prevEap
}

if (-not $UseWsl -and -not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Error "FFmpeg not found. Please install ffmpeg on Windows or in WSL (sudo apt install ffmpeg)."
}

function Convert-ToWslPath {
    param([string]$Path)
    if (-not $Path) { return $Path }
    if (-not [System.IO.Path]::IsPathRooted($Path)) {
        try { $Path = (Resolve-Path $Path -ErrorAction SilentlyContinue).Path } catch {}
    }
    if ($Path -match '^([A-Za-z]):\\(.*)$') {
        $drive = $Matches[1].ToLower()
        $sub = $Matches[2] -replace '\\', '/'
        return "/mnt/$drive/$sub"
    }
    return ($Path -replace '\\', '/')
}

function Invoke-FFmpeg {
    param([string[]]$ArgumentList)
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "SilentlyContinue"
    try {
        if ($UseWsl) {
            # Translate Windows paths and safely wrap each argument for bash
            $wslArgs = @()
            foreach ($arg in $ArgumentList) {
                if ($arg -match '^[A-Za-z]:\\' -or (Test-Path $arg -ErrorAction SilentlyContinue)) {
                    $wslPath = Convert-ToWslPath $arg
                    $wslArgs += "'" + ($wslPath -replace "'", "'\''") + "'"
                } else {
                    $wslArgs += "'" + ($arg -replace "'", "'\''") + "'"
                }
            }
            # Execute bash safely with single-quoted arguments (prevents shell semicolon splitting)
            $bashCmd = "ffmpeg -hide_banner -loglevel warning -y " + ($wslArgs -join " ")
            wsl -d $WslDistroName bash -c $bashCmd
        } else {
            # Invoke native Windows ffmpeg safely with argument vector
            & ffmpeg -hide_banner -loglevel warning -y @ArgumentList
        }
    } finally {
        $ErrorActionPreference = $prevEap
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Error "FFmpeg failed with exit code $LASTEXITCODE"
    }
}

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "       amaranctl Demo Video Stitcher      " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

if ($InputVideo -and (Test-Path $InputVideo) -and -not $Sample) {
    Write-Host "[1/2] Processing raw recording from: $InputVideo..." -ForegroundColor Yellow

    if ($Direct) {
        Write-Host "    Mode: Direct scaling ($TotalWidth px width)..." -ForegroundColor DarkGray
        $filterComplex = "scale=${TotalWidth}:-2:flags=lanczos"

        Write-Host "    Encoding MP4 video..." -ForegroundColor DarkGray
        Invoke-FFmpeg @("-i", $InputVideo, "-vf", $filterComplex, "-r", "30", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "22", $OutputMp4)

        Write-Host "    Generating palette-optimized GIF..." -ForegroundColor DarkGray
        $paletteFilter = "$filterComplex,split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5"
        Invoke-FFmpeg @("-i", $InputVideo, "-vf", $paletteFilter, "-r", "$Fps", $OutputGif)
    } else {
        Write-Host "    Mode: Split-screen stitch (Terminal on Left, Zoomed Tray on Right)..." -ForegroundColor DarkGray

        # Calculate even dimensions: split screen 50/50
        $halfWidth = [int]($TotalWidth / 2)
        if ($halfWidth % 2 -ne 0) { $halfWidth++ }
        $height = 360

        # Crop terminal (y=80..630) and taskbar notification area (x=560..840, y=0..56)
        $filterComplex = "[0:v]crop=840:550:0:80,scale=${halfWidth}:${height}[term];" +
                         "[0:v]crop=280:56:560:0,scale=${halfWidth}:96:flags=lanczos,pad=${halfWidth}:${height}:0:132:0x181825," +
                         "drawtext=text='Windows System Tray':fontcolor=0xa6adc8:fontsize=17:x=(w-text_w)/2:y=65," +
                         "drawtext=text='Live Adaptive Tray Icon':fontcolor=0x89b4fa:fontsize=13:x=(w-text_w)/2:y=255[tray];" +
                         "[term][tray]hstack[v]"

        Write-Host "    Encoding MP4 video..." -ForegroundColor DarkGray
        Invoke-FFmpeg @("-i", $InputVideo, "-filter_complex", $filterComplex, "-map", "[v]", "-r", "30", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "22", $OutputMp4)

        Write-Host "    Generating palette-optimized GIF (<2MB)..." -ForegroundColor DarkGray
        $paletteFilter = "$filterComplex;[v]split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5"
        Invoke-FFmpeg @("-i", $InputVideo, "-filter_complex", $paletteFilter, "-r", "$Fps", $OutputGif)
    }
} else {
    Write-Host "[1/2] Generating synchronized animated split-screen demo..." -ForegroundColor Yellow

    $duration = 10
    $halfWidth = [int]($TotalWidth / 2)
    if ($halfWidth % 2 -ne 0) { $halfWidth++ }
    $height = 360

    # Build an FFmpeg filtergraph:
    # Left: Terminal with simulated typing and IPC responses
    # Right: Taskbar Tray with live adaptive CCT color transitions (Gray -> Warm 3200K -> Daylight 5600K -> Off)
    $fg = "color=c=0x1e1e2e:s=${halfWidth}x${height}:d=${duration}[term_bg];" +
          "color=c=0x181825:s=${halfWidth}x${height}:d=${duration}[tray_bg];" +
          "[term_bg]drawtext=text='amaranctl terminal (IPC)':fontsize=16:fontcolor=0x89b4fa:x=20:y=20," +
          "drawtext=text='$ amaranctl status':fontsize=15:fontcolor=0xf9e2af:x=20:y=60:enable='gte(t,0.5)'," +
          "drawtext=text='Amaran 60x S\: OFF (Ready)':fontsize=14:fontcolor=0xa6adc8:x=20:y=85:enable='gte(t,1.0)'," +
          "drawtext=text='$ amaranctl on':fontsize=15:fontcolor=0xf9e2af:x=20:y=125:enable='gte(t,2.0)'," +
          "drawtext=text='Turned ON  [IPC <50ms]':fontsize=14:fontcolor=0xa6e3a1:x=20:y=150:enable='gte(t,2.4)'," +
          "drawtext=text='$ amaranctl cct 5600':fontsize=15:fontcolor=0xf9e2af:x=20:y=190:enable='gte(t,4.5)'," +
          "drawtext=text='CCT set to 5600K Daylight':fontsize=14:fontcolor=0x89dceb:x=20:y=215:enable='gte(t,4.9)'," +
          "drawtext=text='[Hotkey] Ctrl+Alt+Down (-10 pct)':fontsize=14:fontcolor=0xcba6f7:x=20:y=255:enable='gte(t,7.0)'," +
          "drawtext=text='$ amaranctl off':fontsize=15:fontcolor=0xf9e2af:x=20:y=295:enable='gte(t,8.5)'[term];" +
          "[tray_bg]drawtext=text='Windows System Tray':fontsize=16:fontcolor=0xa6adc8:x=20:y=20," +
          "drawbox=x=${halfWidth}/2-45:y=${height}/2-45:w=90:h=90:color=0x313244:t=fill," +
          "drawbox=x=${halfWidth}/2-25:y=${height}/2-25:w=50:h=50:color=0x585b70:t=fill:enable='lt(t,2.0)+gte(t,8.8)'," +
          "drawtext=text='[o] OFF':fontsize=16:fontcolor=0xa6adc8:x=${halfWidth}/2-35:y=${height}/2+60:enable='lt(t,2.0)+gte(t,8.8)'," +
          "drawbox=x=${halfWidth}/2-25:y=${height}/2-25:w=50:h=50:color=0xffaa33:t=fill:enable='between(t,2.0,4.8)'," +
          "drawtext=text='(*) ON | 3200K Warm':fontsize=16:fontcolor=0xffaa33:x=${halfWidth}/2-75:y=${height}/2+60:enable='between(t,2.0,4.8)'," +
          "drawbox=x=${halfWidth}/2-25:y=${height}/2-25:w=50:h=50:color=0xcce6ff:t=fill:enable='between(t,4.8,8.8)'," +
          "drawtext=text='(*) ON | 5600K Daylight':fontsize=16:fontcolor=0xcce6ff:x=${halfWidth}/2-85:y=${height}/2+60:enable='between(t,4.8,8.8)'," +
          "drawbox=x=${halfWidth}/2-120:y=300:w=240:h=36:color=0x45475a:t=fill:enable='between(t,7.0,8.4)'," +
          "drawtext=text='[KEY] Ctrl+Alt+Down':fontsize=14:fontcolor=0xf5c2e7:x=${halfWidth}/2-75:y=310:enable='between(t,7.0,8.4)'[tray];" +
          "[term][tray]hstack[v]"

    Write-Host "    Generating MP4 demo..." -ForegroundColor DarkGray
    Invoke-FFmpeg @("-f", "lavfi", "-i", "nullsrc=s=${TotalWidth}x${height}:d=${duration}", "-filter_complex", $fg, "-map", "[v]", "-r", "$Fps", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-crf", "20", $OutputMp4)

    Write-Host "    Generating optimized GIF..." -ForegroundColor DarkGray
    $gifFg = "$fg;[v]split[s0][s1];[s0]palettegen=stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=5"
    Invoke-FFmpeg @("-f", "lavfi", "-i", "nullsrc=s=${TotalWidth}x${height}:d=${duration}", "-filter_complex", $gifFg, "-r", "$Fps", $OutputGif)
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Green
Write-Host "        Render Successful!                " -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green
$gifItem = Get-Item $OutputGif
$mp4Item = Get-Item $OutputMp4
Write-Host ("GIF:  {0} ({1:N1} KB)" -f $OutputGif, ($gifItem.Length / 1KB)) -ForegroundColor Green
Write-Host ("MP4:  {0} ({1:N1} KB)" -f $OutputMp4, ($mp4Item.Length / 1KB)) -ForegroundColor Green
Write-Host ""
