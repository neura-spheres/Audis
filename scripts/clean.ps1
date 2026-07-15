<#
.SYNOPSIS
    Removes Audis build artifacts.

.DESCRIPTION
    Deletes build output only. It never touches your Audis user data under
    %LOCALAPPDATA%\NeuraAudis\Audis, use the in-app privacy controls for that.

.PARAMETER IncludeNodeModules
    Also remove installed node_modules directories.

.EXAMPLE
    ./scripts/clean.ps1

.EXAMPLE
    ./scripts/clean.ps1 -IncludeNodeModules
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [switch]$IncludeNodeModules
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot

$targets = @(
    (Join-Path $repoRoot 'target'),
    (Join-Path $repoRoot 'apps/desktop/dist'),
    (Join-Path $repoRoot 'apps/desktop/src-tauri/gen')
)

if ($IncludeNodeModules) {
    $targets += Get-ChildItem -Path $repoRoot -Filter 'node_modules' -Recurse -Directory -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -notlike '*node_modules*node_modules*' } |
        Select-Object -ExpandProperty FullName
}

foreach ($target in $targets) {
    if (Test-Path $target) {
        if ($PSCmdlet.ShouldProcess($target, 'Remove')) {
            Write-Host "  removing $target" -ForegroundColor DarkGray
            Remove-Item -Path $target -Recurse -Force -ErrorAction Stop
        }
    }
}

Write-Host "Clean complete. Your Audis user data was not touched." -ForegroundColor Green
