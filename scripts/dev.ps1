<#
.SYNOPSIS
    Runs Audis in development mode.

.DESCRIPTION
    Starts the Vite dev server and the Tauri shell together. Tauri owns the
    process: it launches Vite via `beforeDevCommand` in tauri.conf.json, so this
    script only sets logging and hands off.

.PARAMETER LogLevel
    Value for AUDIS_LOG (trace|debug|info|warn|error). Defaults to debug.

.EXAMPLE
    ./scripts/dev.ps1

.EXAMPLE
    ./scripts/dev.ps1 -LogLevel trace
#>
[CmdletBinding()]
param(
    [ValidateSet('trace', 'debug', 'info', 'warn', 'error')]
    [string]$LogLevel = 'debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot

if (-not (Test-Path (Join-Path $repoRoot 'node_modules'))) {
    throw "Dependencies are not installed. Run ./scripts/setup.ps1 first."
}

# Keep third-party crates quiet even at trace; their output drowns ours.
$env:AUDIS_LOG = "$LogLevel,wry=warn,tao=warn,hyper=warn"
# Rust backtraces are useful in development and never shipped.
$env:RUST_BACKTRACE = '1'

Write-Host ""
Write-Host "Starting Audis (log level: $LogLevel)" -ForegroundColor Cyan
Write-Host "Press Ctrl+C to stop." -ForegroundColor DarkGray
Write-Host ""

Push-Location (Join-Path $repoRoot 'apps/desktop')
try {
    pnpm tauri dev
    if ($LASTEXITCODE -ne 0) { throw "tauri dev exited with code $LASTEXITCODE." }
} finally {
    Pop-Location
}
