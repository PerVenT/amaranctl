# scripts/install.ps1 - Automated installer for amaranctl on Windows
# No Python or developer tools required. Uses native Windows APIs.

$ErrorActionPreference = "Stop"

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "       amaranctl Windows Installer        " -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

$configDir  = "$HOME\.config\amaranctl"
$configPath = "$configDir\config.json"

function Test-AmaranConfig([string]$path) {
    try {
        if (-not (Test-Path $path)) {
            return @{ Valid = $false; Error = "File does not exist" }
        }
        $raw = Get-Content -Path $path -Raw -ErrorAction Stop
        $obj = ConvertFrom-Json $raw -ErrorAction Stop

        if (-not $obj.meshUuid -or -not ($obj.meshUuid -is [string]) -or $obj.meshUuid.Trim() -eq "") {
            return @{ Valid = $false; Error = "Missing or empty 'meshUuid'" }
        }
        if (-not $obj.netKey -or -not ($obj.netKey -is [string]) -or $obj.netKey.Trim().Length -ne 32) {
            return @{ Valid = $false; Error = "Invalid 'netKey' (expected 32 hex chars)" }
        }
        if (-not $obj.appKey -or -not ($obj.appKey -is [string]) -or $obj.appKey.Trim().Length -ne 32) {
            return @{ Valid = $false; Error = "Invalid 'appKey' (expected 32 hex chars)" }
        }
        if (-not $obj.lights -or ($obj.lights.Count -eq 0)) {
            return @{ Valid = $false; Error = "No light fixtures configured in 'lights'" }
        }
        foreach ($light in $obj.lights) {
            if (-not $light.mac -or -not $light.address) {
                return @{ Valid = $false; Error = "Light fixture missing required 'mac' or 'address'" }
            }
        }
        return @{ Valid = $true; Config = $obj }
    } catch {
        return @{ Valid = $false; Error = "JSON parsing error: $_" }
    }
}

function Find-AmaranDb {
    $searchBase = "$env:APPDATA\amaran Desktop"
    if (Test-Path $searchBase) {
        $found = Get-ChildItem -Path $searchBase -Filter "amaran.db" -Recurse -File -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) {
            return $found.FullName
        }
    }
    return $null
}

function Extract-Credentials([string]$dbPath, [string]$outPath) {
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

    # Read mesh credentials
    $meshStmt = [IntPtr]::Zero
    $prepRes = [WinSqlite3]::Prepare($dbPtr, "SELECT uuid, net_key, app_key FROM mesh LIMIT 1;", -1, [ref]$meshStmt, [IntPtr]::Zero)
    if ($prepRes -ne 0 -or [WinSqlite3]::Step($meshStmt) -ne 100) {
        [WinSqlite3]::Close($dbPtr) | Out-Null
        Write-Error "Failed to read mesh credentials from database. Is your mesh network configured in Amaran Desktop?"
        exit 1
    }

    $meshUuid = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($meshStmt, 0))
    $netKey   = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($meshStmt, 1))
    $appKey   = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi([WinSqlite3]::ColumnText($meshStmt, 2))
    [WinSqlite3]::Finalize($meshStmt) | Out-Null

    # Read fixtures
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

    $configObj = [PSCustomObject]@{
        meshUuid = $meshUuid
        netKey   = $netKey
        appKey   = $appKey
        lights   = $fixtures
    }

    $dir = Split-Path -Parent $outPath
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
    $configObj | ConvertTo-Json -Depth 5 | Set-Content -Path $outPath -Encoding UTF8
    return $configObj
}

# --- Step 1 & 2: Configuration Check / Extraction ---
Write-Host "[1/4] Checking configuration ($configPath)..." -ForegroundColor Yellow

$needsExtraction = $true

if (Test-Path $configPath) {
    $check = Test-AmaranConfig $configPath
    if ($check.Valid) {
        Write-Host "    Found existing valid configuration at: $configPath" -ForegroundColor Green
        Write-Host "    Configured lights ($($check.Config.lights.Count)):" -ForegroundColor Gray
        foreach ($l in $check.Config.lights) {
            Write-Host "      - $($l.name) (MAC: $($l.mac), Address: $($l.address))" -ForegroundColor Gray
        }
        $needsExtraction = $false
    } else {
        Write-Host "    [!] Existing config file is invalid: $($check.Error)" -ForegroundColor Red
        $dbPath = Find-AmaranDb
        if ($dbPath) {
            Write-Host "    Found local database: $dbPath" -ForegroundColor Cyan
            $replace = Read-Host "    Backup invalid config ($configPath.bak) and extract fresh keys from database? (Y/n)"
            if ($replace -eq "" -or $replace.ToLower() -eq "y") {
                Copy-Item -Path $configPath -Destination "$configPath.bak" -Force
                Write-Host "    Backed up corrupted config to: $configPath.bak" -ForegroundColor Gray
                $needsExtraction = $true
            } else {
                Write-Host "    Keeping current configuration as requested." -ForegroundColor Yellow
                $needsExtraction = $false
            }
        } else {
            Write-Host "    [!] Could not locate amaran.db to regenerate configuration." -ForegroundColor Red
            Write-Host "        Please fix $configPath manually or install Amaran Desktop and log in."
            exit 1
        }
    }
}

if ($needsExtraction) {
    Write-Host "[2/4] Extracting Bluetooth Mesh credentials from database..." -ForegroundColor Yellow
    $dbPath = Find-AmaranDb
    if (-not $dbPath) {
        Write-Host "    [!] Could not find amaran.db in $env:APPDATA\amaran Desktop" -ForegroundColor Red
        Write-Host "        Please make sure Amaran Desktop is installed and logged in."
        $manual = Read-Host "        Or enter path to amaran.db manually (press Enter to cancel)"
        if ($manual -and (Test-Path $manual)) {
            $dbPath = $manual
        } else {
            Write-Host "    Aborted. No valid configuration or database available." -ForegroundColor Red
            exit 1
        }
    }

    Write-Host "    Reading from: $dbPath" -ForegroundColor Green
    $extracted = Extract-Credentials -dbPath $dbPath -outPath $configPath
    Write-Host "    Successfully generated: $configPath" -ForegroundColor Green
    Write-Host "    Configured lights ($($extracted.lights.Count)):" -ForegroundColor Gray
    foreach ($l in $extracted.lights) {
        Write-Host "      - $($l.name) (MAC: $($l.mac), Address: $($l.address))" -ForegroundColor Gray
    }
} else {
    Write-Host "[2/4] Configuration ready (skipped database extraction)." -ForegroundColor Green
}

# --- Step 3: Install amaranctl binary ---
Write-Host "[3/4] Setting up amaranctl executable..." -ForegroundColor Yellow
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

# Add to User PATH if missing
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    $env:Path = "$env:Path;$installDir"
    Write-Host "    Added $installDir to User PATH." -ForegroundColor Green
}

# --- Step 4: Autostart setup ---
Write-Host "[4/4] Autostart configuration..." -ForegroundColor Yellow
$choice = Read-Host "Start amaranctl automatically when you log into Windows? (Y/n)"
if ($choice -eq "" -or $choice.ToLower() -eq "y") {
    $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
    Set-ItemProperty -Path $regPath -Name "amaranctl" -Value "`"$targetExe`""
    Write-Host "    Autostart enabled in Windows Registry." -ForegroundColor Green
}

Write-Host ""
Write-Host "==========================================" -ForegroundColor Green
Write-Host "        Installation Successful!          " -ForegroundColor Green
Write-Host "==========================================" -ForegroundColor Green
Write-Host "Config:  $configPath"
Write-Host "Binary:  $targetExe"
Write-Host ""
Write-Host "Usage:"
Write-Host "  Tray app:  Run 'amaranctl' or double-click amaranctl.exe"
Write-Host "  CLI:       amaranctl on | off | toggle | brightness 50 | cct 4500"
Write-Host "  Hotkeys:   Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
Write-Host ""
