#!/usr/bin/env pwsh
<#
.SYNOPSIS
  Set SMNSERVICEKEY in your environment (current session + persistent user-level).

.DESCRIPTION
  This script takes one mandatory parameter (the key) and:
    1. Exports it into the current process as $env:SMNSERVICEKEY.
    2. Persists it at the User level so it’s available in new PowerShell sessions.
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0, HelpMessage = 'Your SMN service key')]
    [string] $Key
)

try {
    # 1) Set for current session
    $env:SMNSERVICEKEY = $Key

    # 2) Persist at User scope
    [System.Environment]::SetEnvironmentVariable(
        'SMNSERVICEKEY',
        $Key,
        [System.EnvironmentVariableTarget]::User
    )

    Write-Host "✅ SMNSERVICEKEY set for current session and persisted for your user."
    Write-Host "   New PowerShell windows will also see it automatically."
    exit 0
}
catch {
    Write-Error "❌ Failed to set environment variable: $_"
    exit 1
}
