#!/usr/bin/env pwsh
<#
.SYNOPSIS
   Update the smn_site_ciri Git repository to its latest remote state.

.DESCRIPTION
   Changes into the project root (one level up from this script), fetches and merges
   the latest commits from the current branch’s remote, and updates any submodules.

.EXAMPLE
   cd smn_site_ciri/scripts
   sudo pwsh ./update.ps1
#>

# Ensure pwsh (PowerShell Core) on Linux
if (-not (Get-Command pwsh -ErrorAction SilentlyContinue)) {
    Write-Error "PowerShell Core (pwsh) not found. Install it first: https://docs.microsoft.com/powershell/scripting/install/installing-powershell"
    exit 1
}

try {
    # Compute project root (assumes this script lives in <project>/scripts/)
    $projectRoot = Resolve-Path (Join-Path $PSScriptRoot '..')

    Write-Host "Changing directory to project root: $projectRoot" -ForegroundColor Cyan
    Set-Location $projectRoot

    Write-Host "Fetching latest from remote…" -ForegroundColor Cyan
    git fetch --all

    # Determine current branch
    $branch = git rev-parse --abbrev-ref HEAD
    Write-Host "On branch '$branch'. Pulling updates…" -ForegroundColor Cyan
    git pull origin $branch

    # If you use submodules:
    if (Test-Path .gitmodules) {
        Write-Host "Updating submodules…" -ForegroundColor Cyan
        git submodule update --init --recursive
    }

    $status = git status --short
    if (-not [string]::IsNullOrWhiteSpace($status)) {
        Write-Host "Repository updated. Current status:" -ForegroundColor Green
        git status
    } else {
        Write-Host "Repository is up to date." -ForegroundColor Green
    }
}
catch {
    Write-Error "An error occurred: $_"
    exit 1
}
