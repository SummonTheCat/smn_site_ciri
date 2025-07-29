#!/usr/bin/env pwsh
<#
.SYNOPSIS
   Build the Rust project and run it as a per-user systemd service named "smn-site".

.DESCRIPTION
   1. Builds the Rust project in release mode.
   2. Creates or updates ~/.config/systemd/user/smn-site.service.
   3. Reloads systemd --user, enables, and restarts the service.
   No root required, but you must have systemd user services enabled (e.g. via
   `loginctl enable-linger $USER` if you need it running when not logged in).
#>

# 1. Compute project root (one level up from this script) and cd there
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Set-Location $projectRoot

# 2. Build the Rust project in release
Write-Host "→ Building Rust project (release) ..." -ForegroundColor Cyan
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found. Make sure Rust is installed and on PATH."
    exit 1
}
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo build failed."
    exit 1
}

# 3. Paths
$binaryName   = "smn_site_ciri"   # adjust if your crate binary name differs
$binaryPath   = Join-Path $projectRoot "target/release/$binaryName"
$userSvcDir   = Join-Path $HOME ".config/systemd/user"
$serviceFile  = Join-Path $userSvcDir "smn-site.service"

if (-not (Test-Path $binaryPath)) {
    Write-Error "Built binary not found at $binaryPath"
    exit 1
}

# 4. Ensure the user service directory exists
if (-not (Test-Path $userSvcDir)) {
    Write-Host "→ Creating user service directory: $userSvcDir" -ForegroundColor Cyan
    New-Item -ItemType Directory -Path $userSvcDir -Force | Out-Null
}

# 5. Write the service unit
$serviceDef = @"
[Unit]
Description=smn-site Rust Service (user)
After=network.target

[Service]
Type=simple
ExecStart=$binaryPath
WorkingDirectory=$projectRoot
Restart=on-failure

[Install]
WantedBy=default.target
"@

Write-Host "→ Writing user service file to $serviceFile" -ForegroundColor Cyan
$serviceDef | Set-Content -Path $serviceFile -Encoding UTF8

# 6. Reload, enable & restart under --user
Write-Host "→ Reloading systemd user daemon ..." -ForegroundColor Cyan
systemctl --user daemon-reload

Write-Host "→ Enabling smn-site.service for user on login/start ..." -ForegroundColor Cyan
systemctl --user enable smn-site

Write-Host "→ Restarting smn-site.service ..." -ForegroundColor Cyan
systemctl --user restart smn-site

# 7. Show status
Write-Host "→ Current user service status:" -ForegroundColor Green
systemctl --user status smn-site --no-pager
