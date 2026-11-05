# scripts/install.ps1 - Automated installer for amaranctl on Windows
# No Python or developer tools required. Uses native Windows APIs.

$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "       amaranctl Windows Installer        " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Locate amaran.db
Write-Host "[1/5] Searching for Amaran Desktop database..." -ForegroundColor Yellow
$searchBase = "$env:APPDATA\amaran Desktop"
$dbPath = $null

if (Test-Path $searchBase) {
    $found = Get-ChildItem -Path $searchBase -Filter "amaran.db" -Recurse -File -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) {
        $dbPath = $found.FullName
    }
}

if (-not $dbPath -or -not (Test-Path $dbPath)) {
    Write-Host "[!] Could not find amaran.db automatically." -ForegroundColor Red
    Write-Host "    Make sure you have installed 'Amaran Desktop' and logged into your account once."
    $manual = Read-Host "    Or enter path to amaran.db manually (press Enter to cancel)"
    if ($manual -and (Test-Path $manual)) {
        $dbPath = $manual
    } else {
        Write-Host "Aborted. Please install Amaran Desktop and log in first." -ForegroundColor Red
        exit 1
    }
}

Write-Host "    Found database: $dbPath" -ForegroundColor Green

# 2. Extract credentials using native winsqlite3.dll
Write-Host "[2/5] Extracting Bluetooth Mesh credentials..." -ForegroundColor Yellow

if (-not ([System.Management.Automation.PSTypeName]'WinSqlite3').Type) {
    $csharp = @"
using System;
using System.Runtime.InteropServices;

public class WinSqlite3 {
    [DllImport("winsqlite3.dll", EntryPoint = "sqlite3_open", CallingConvention = CallingConvention.Cdecl)]
    public static extern int Open(string filename, out IntPtr db);

    [DllImport("winsqlite3.dll", EntryPoint = "sqlite3_close", CallingConvention = CallingConvention.Cdecl)]
    public static extern int Close(IntPtr db);

    [DllImport("winsqlite3.dll", EntryPoint = "sqlite3_prepare_v2", CallingConvention = CallingConvention.Cdecl)]
    public static extern int Prepare(IntPtr db, string sql, int numBytes, out IntPtr stmt, IntPtr pzTail);

    [DllImport("winsqlite3.dll", EntryPoint = "sqlite3_step", CallingConvention = CallingConvention.Cdecl)]
    public static extern int Step(IntPtr stmt);

    [DllImport("winsqlite3.dll", EntryPoint = "sqlite3_column_text", CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr ColumnText(IntPtr stmt, int col);

    [DllImport("winsqlite3.dll", EntryPoint = "sqlite3_column_int", CallingConvention = CallingConvention.Cdecl)]
    public static extern int ColumnInt(IntPtr stmt, int col);

    [DllImport("winsqlite3.dll", EntryPoint = "sqlite3_finalize", CallingConvention = CallingConvention.Cdecl)]
    public static extern int Finalize(IntPtr stmt);
}
"@
    Add-Type -TypeDefinition $csharp
}

$dbPtr = [IntPtr]::Zero
$openRes = [WinSqlite3]::Open($dbPath, [ref]$dbPtr)
if ($openRes -ne 0) {
    Write-Error "Failed to open SQLite database at $dbPath"
    exit 1
}

# Read mesh table
$meshStmt = [IntPtr]::Zero
$prepRes = [WinSqlite3]::Prepare($dbPtr, "SELECT uuid, net_key, app_key FROM mesh LIMIT 1;", -1, [ref]$meshStmt, [IntPtr]::Zero)
if ($prepRes -ne 0 -or [WinSqlite3]::Step($meshStmt) -ne 100) {
    [WinSqlite3]::Close($dbPtr) | Out-Null
    Write-Error "Failed to read mesh credentials from database. Is your mesh network set up?"
    exit 1
}

$meshUuid = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($meshStmt, 0))
$netKey   = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($meshStmt, 1))
$appKey   = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($meshStmt, 2))
[WinSqlite3]::Finalize($meshStmt) | Out-Null

# Read fixtures table
$fixStmt = [IntPtr]::Zero
$prepRes = [WinSqlite3]::Prepare($dbPtr, "SELECT name, mac_address, node_address, device_key, device_uuid, code FROM fixtures;", -1, [ref]$fixStmt, [IntPtr]::Zero)

$fixtures = @()
if ($prepRes -eq 0) {
    while ([WinSqlite3]::Step($fixStmt) -eq 100) {
        $name     = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($fixStmt, 0))
        $mac      = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($fixStmt, 1))
        $nodeAddr = [WinSqlite3]::ColumnInt($fixStmt, 2)
        $devKey   = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($fixStmt, 3))
        $devUuid  = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($fixStmt, 4))
        $code     = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($fixStmt, 5))

        $fixtures += [PSCustomObject]@{
            name       = $name
            mac        = $mac
            address    = $nodeAddr
            deviceKey  = $devKey
            deviceUuid = $devUuid
            code       = $code
        }
    }
    [WinSqlite3]::Finalize($fixStmt) | Out-Null
}
[WinSqlite3]::Close($dbPtr) | Out-Null

if ($fixtures.Count -eq 0) {
    Write-Host "[!] Warning: No lights found in fixtures table." -ForegroundColor Yellow
} else {
    Write-Host "    Found $($fixtures.Count) light(s):" -ForegroundColor Green
    foreach ($f in $fixtures) {
        Write-Host "      - $($f.name) (MAC: $($f.mac), Address: $($f.address))" -ForegroundColor Gray
    }
}

# 3. Save config.json
Write-Host "[3/5] Saving configuration..." -ForegroundColor Yellow
$configDir = "$env:APPDATA\amaranctl"
if (-not (Test-Path $configDir)) {
    New-Item -ItemType Directory -Path $configDir -Force | Out-Null
}
$configPath = "$configDir\config.json"

$configObj = [PSCustomObject]@{
    meshUuid = $meshUuid
    netKey   = $netKey
    appKey   = $appKey
    lights   = $fixtures
}

$configObj | ConvertTo-Json -Depth 5 | Set-Content -Path $configPath -Encoding UTF8
Write-Host "    Saved config to: $configPath" -ForegroundColor Green

# 4. Install amaranctl binary
Write-Host "[4/5] Setting up amaranctl executable..." -ForegroundColor Yellow
$installDir = "$env:LOCALAPPDATA\Programs\amaranctl"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}
$targetExe = "$installDir\amaranctl.exe"

# If running from repo root or dist directory, copy existing binary
$localExe = Join-Path $PSScriptRoot "..\amaranctl.exe"
if (Test-Path $localExe) {
    Copy-Item -Path $localExe -Destination $targetExe -Force
    Write-Host "    Copied local binary to: $targetExe" -ForegroundColor Green
} else {
    # Download latest release from GitHub
    Write-Host "    Downloading latest amaranctl.exe from GitHub..." -ForegroundColor Cyan
    $repo = "amaranctl/amaranctl" # Update when official repo is live
    $downloadUrl = "https://github.com/$repo/releases/latest/download/amaranctl-windows.exe"
    try {
        Invoke-WebRequest -Uri $downloadUrl -OutFile $targetExe -UseBasicParsing
        Write-Host "    Downloaded to: $targetExe" -ForegroundColor Green
    } catch {
        Write-Host "    Could not download automatically. Please place amaranctl.exe in $installDir" -ForegroundColor Yellow
    }
}

# Add to User PATH if not already present
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path = "$env:Path;$installDir"
    Write-Host "    Added $installDir to User PATH." -ForegroundColor Green
}

# 5. Autostart setup
Write-Host "[5/5] Autostart configuration..." -ForegroundColor Yellow
$choice = Read-Host "Start amaranctl automatically when you log into Windows? (Y/n)"
if ($choice -eq "" -or $choice.ToLower() -eq "y") {
    $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    Set-ItemProperty -Path $regPath -Name "amaranctl" -Value "`"$targetExe`""
    Write-Host "    Autostart enabled!" -ForegroundColor Green
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Green
Write-Host "        Installation Successful!          " -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green
Write-Host "Usage:"
Write-Host "  Tray app:  Run 'amaranctl' or double-click amaranctl.exe"
Write-Host "  CLI:       amaranctl on | off | toggle | brightness 50 | cct 4500"
Write-Host "  Hotkeys:   Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
Write-Host ""
