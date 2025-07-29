#!/usr/bin/env pwsh
<#
.SYNOPSIS
   Build the Rust project and run it as a systemd service named "smn-site".

.DESCRIPTION
   1. Builds the Rust project in release mode.
   2. Creates or updates /etc/systemd/system/smn-site.service.
   3. Reloads systemd, enables, and restarts the service.
   Must be run as root (e.g. via sudo).

.EXAMPLE
   cd smn_site_ciri/scripts
   sudo pwsh ./host.ps1
#>

# Ensure running as root
if ($EUID -ne 0) {
    Write-Error "This script must be run as root. Try: sudo pwsh $PSCommandPath"
    exit 1
}

# Determine project root (one level up from this script)
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $projectRoot

# Build
Write-Host "→ Building Rust project (release) ..." -ForegroundColor Cyan
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found. Make sure Rust is installed and on PATH."
    exit 1
}
& cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo build failed."
    exit 1
}

# Paths
$binaryName    = "smn_site_ciri"    # replace if your binary name differs
$binaryPath    = Join-Path $projectRoot "target/release/$binaryName"
$serviceFile   = "/etc/systemd/system/smn-site.service"

if (-not (Test-Path $binaryPath)) {
    Write-Error "Built binary not found at $binaryPath"
    exit 1
}

# Create or update the systemd service
$serviceDef = @"
[Unit]
Description=smn-site Rust Service
After=network.target

[Service]
Type=simple
ExecStart=$binaryPath
WorkingDirectory=$projectRoot
Restart=on-failure

[Install]
WantedBy=multi-user.target
"@

Write-Host "→ Writing systemd service file to $serviceFile" -ForegroundColor Cyan
$serviceDef | Set-Content -Path $serviceFile -Encoding UTF8

# Reload, enable, restart
Write-Host "→ Reloading systemd daemon ..." -ForegroundColor Cyan
& systemctl daemon-reload

Write-Host "→ Enabling smn-site service on boot ..." -ForegroundColor Cyan
& systemctl enable smn-site

Write-Host "→ Restarting smn-site service ..." -ForegroundColor Cyan
& systemctl restart smn-site

# Show status
Write-Host "→ Service status:" -ForegroundColor Green
& systemctl status smn-site --no-pager
