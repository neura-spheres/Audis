<#
.SYNOPSIS
    Builds Audis for release.

.DESCRIPTION
    Produces Audis.exe with the frontend bundled in.

    This uses `tauri build`, not `cargo build --release`. The distinction
    matters: a plain cargo build produces a binary that still points at the Vite
    dev server, so it opens to a connection error and shows a console window.
    Never ship a cargo-built binary.

.PARAMETER Bundle
    Also produce the NSIS and MSI installers.

.PARAMETER SkipChecks
    Skip the format, lint and test gates. For local iteration only.

.EXAMPLE
    ./scripts/build.ps1

.EXAMPLE
    ./scripts/build.ps1 -Bundle
#>
[CmdletBinding()]
param(
    [switch]$Bundle,
    [switch]$SkipChecks
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot

# The local Whisper engine is C++, so the Rust build needs an MSVC environment,
# libclang and a real ninja. Sourcing this here means a contributor never has to
# know that; see docs/adr/ADR-011.
. (Join-Path $PSScriptRoot 'env.ps1')

Push-Location $repoRoot
try {
    if ($SkipChecks) {
        Write-Warning 'Skipping checks. Do not ship an artifact built this way.'
    }
    else {
        Write-Host '> Running checks' -ForegroundColor Cyan
        & (Join-Path $PSScriptRoot 'test.ps1')
    }

    $arguments = @('tauri', 'build')
    if (-not $Bundle) {
        $arguments += '--no-bundle'
    }

    Write-Host ''
    Write-Host '> Building Audis' -ForegroundColor Cyan
    Push-Location (Join-Path $repoRoot 'apps/desktop')
    try {
        pnpm @arguments
        if ($LASTEXITCODE -ne 0) { throw "tauri build failed with exit code $LASTEXITCODE." }
    }
    finally {
        Pop-Location
    }

    $exe = Join-Path $repoRoot 'target/release/audis-desktop.exe'
    if (-not (Test-Path $exe)) { throw "Expected binary was not produced: $exe" }

    $sizeMb = [math]::Round((Get-Item $exe).Length / 1MB, 2)
    Write-Host ''
    Write-Host "Built $exe ($sizeMb MB)" -ForegroundColor Green

    if ($Bundle) {
        $bundleDir = Join-Path $repoRoot 'target/release/bundle'
        Get-ChildItem -Path $bundleDir -Include '*.exe', '*.msi' -Recurse -ErrorAction SilentlyContinue |
            ForEach-Object { Write-Host "Bundled $($_.FullName)" -ForegroundColor Green }
    }
    Write-Host ''
}
finally {
    Pop-Location
}
