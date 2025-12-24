# ============================================================
# Sage Agent Installer for Windows
# https://github.com/majiayu000/sage
#
# Usage:
#   irm https://raw.githubusercontent.com/majiayu000/sage/main/install.ps1 | iex
#
# Options:
#   $env:SAGE_VERSION    - Specific version to install (default: latest)
#   $env:SAGE_INSTALL_DIR - Installation directory (default: ~/.local/bin)
#
# ============================================================

$ErrorActionPreference = "Stop"

$Version = if ($env:SAGE_VERSION) { $env:SAGE_VERSION } else { "latest" }
$InstallDir = if ($env:SAGE_INSTALL_DIR) { $env:SAGE_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }
$Repo = "majiayu000/sage"
$BinaryName = "sage"

# ============================================================
# Helper Functions
# ============================================================

function Write-Banner {
    Write-Host ""
    Write-Host @"
  ____
 / ___|  __ _  __ _  ___
 \___ \ / _` |/ _` |/ _ \
  ___) | (_| | (_| |  __/
 |____/ \__,_|\__, |\___|
              |___/

"@ -ForegroundColor Cyan
    Write-Host "Blazing fast code agent in pure Rust" -ForegroundColor White
    Write-Host ""
}

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] " -ForegroundColor Blue -NoNewline
    Write-Host $Message
}

function Write-Success {
    param([string]$Message)
    Write-Host "[OK] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error-Exit {
    param([string]$Message)
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

# ============================================================
# Version Management
# ============================================================

function Get-LatestVersion {
    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        return $response.tag_name
    }
    catch {
        Write-Error-Exit "Failed to fetch latest version. Please check your internet connection."
    }
}

# ============================================================
# Download and Install
# ============================================================

function Install-Sage {
    param([string]$Ver)

    if ($Ver -eq "latest") {
        Write-Info "Fetching latest version..."
        $Ver = Get-LatestVersion
    }

    Write-Info "Installing Sage $Ver..."

    # Determine architecture
    $arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "i686" }
    $platform = "$arch-pc-windows-msvc"

    # Try different filename patterns
    $filenames = @(
        "$BinaryName-$Ver-$platform.zip",
        "$BinaryName-v$($Ver.TrimStart('v'))-$platform.zip",
        "$BinaryName-$($Ver.TrimStart('v'))-$platform.zip"
    )

    $tmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    $zipPath = Join-Path $tmpDir "sage.zip"

    $downloaded = $false
    foreach ($filename in $filenames) {
        $url = "https://github.com/$Repo/releases/download/$Ver/$filename"
        Write-Info "Trying: $filename"

        try {
            Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
            $downloaded = $true
            Write-Success "Downloaded successfully"
            break
        }
        catch {
            continue
        }
    }

    if (-not $downloaded) {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Error-Exit "Failed to download. Please check if release exists for version $Ver"
    }

    # Extract
    Write-Info "Extracting..."
    try {
        Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force
    }
    catch {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Error-Exit "Failed to extract archive"
    }

    # Find binary
    $binaryPath = Get-ChildItem -Path $tmpDir -Recurse -Filter "$BinaryName.exe" | Select-Object -First 1

    if (-not $binaryPath) {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Error-Exit "Binary not found in archive"
    }

    # Install
    Write-Info "Installing to $InstallDir..."
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    $destPath = Join-Path $InstallDir "$BinaryName.exe"
    Move-Item -Path $binaryPath.FullName -Destination $destPath -Force

    # Cleanup
    Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue

    Write-Success "Installed successfully!"
    return $Ver
}

# ============================================================
# PATH Setup
# ============================================================

function Add-ToPath {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")

    if ($currentPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable(
            "Path",
            "$currentPath;$InstallDir",
            "User"
        )
        Write-Warn "Added $InstallDir to PATH"
        Write-Host ""
        Write-Host "  " -NoNewline
        Write-Host "Please restart your terminal for PATH changes to take effect." -ForegroundColor Yellow
        Write-Host ""
    }
}

# ============================================================
# Verification
# ============================================================

function Show-NextSteps {
    param([string]$Ver)

    Write-Host ""
    Write-Host "Installation complete!" -ForegroundColor Green
    Write-Host ""
    Write-Host ("=" * 55)
    Write-Host ""
    Write-Host "  Get started:" -ForegroundColor White
    Write-Host ""
    Write-Host "    sage --help" -ForegroundColor Cyan -NoNewline
    Write-Host "              Show help"
    Write-Host "    sage interactive" -ForegroundColor Cyan -NoNewline
    Write-Host "         Start interactive mode"
    Write-Host "    sage `"Your task`"" -ForegroundColor Cyan -NoNewline
    Write-Host "         Run a one-shot task"
    Write-Host ""
    Write-Host ("=" * 55)
    Write-Host ""
    Write-Host "  Documentation: " -NoNewline
    Write-Host "https://github.com/$Repo" -ForegroundColor Blue
    Write-Host ""

    $sagePath = Join-Path $InstallDir "$BinaryName.exe"
    if (Test-Path $sagePath) {
        try {
            $version = & $sagePath --version 2>$null
            if ($version) {
                Write-Host "  Installed version: $version"
                Write-Host ""
            }
        }
        catch {}
    }
}

# ============================================================
# Build from Source
# ============================================================

function Build-FromSource {
    Write-Info "Building from source..."

    # Check for cargo
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        Write-Error-Exit "Rust is not installed. Please install Rust first: https://rustup.rs"
    }

    # Check for git
    $git = Get-Command git -ErrorAction SilentlyContinue
    if (-not $git) {
        Write-Error-Exit "Git is not installed"
    }

    $tmpDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }

    try {
        Write-Info "Cloning repository..."
        git clone --depth 1 "https://github.com/$Repo.git" "$tmpDir\sage"

        Write-Info "Building (this may take a few minutes)..."
        Set-Location "$tmpDir\sage"
        cargo build --release

        Write-Info "Installing..."
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        $destPath = Join-Path $InstallDir "$BinaryName.exe"
        Copy-Item "target\release\$BinaryName.exe" $destPath -Force

        Write-Success "Built and installed successfully!"
    }
    finally {
        Set-Location $env:USERPROFILE
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# ============================================================
# Main
# ============================================================

function Main {
    param([string[]]$Args)

    Write-Banner

    # Check for help
    if ($Args -contains "--help" -or $Args -contains "-h") {
        Write-Host "Usage: install.ps1 [OPTIONS]"
        Write-Host ""
        Write-Host "Options:"
        Write-Host "  --help, -h     Show this help message"
        Write-Host "  --source       Build from source instead of downloading binary"
        Write-Host ""
        Write-Host "Environment variables:"
        Write-Host "  SAGE_VERSION      Version to install (default: latest)"
        Write-Host "  SAGE_INSTALL_DIR  Installation directory (default: ~/.local/bin)"
        Write-Host ""
        exit 0
    }

    # Check for source build
    if ($Args -contains "--source") {
        Build-FromSource
        Add-ToPath
        Show-NextSteps "source"
        exit 0
    }

    # Install from binary
    $installedVersion = Install-Sage -Ver $Version
    Add-ToPath
    Show-NextSteps -Ver $installedVersion
}

Main $args
