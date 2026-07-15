<#
.SYNOPSIS
    Runs every check CI runs: formatting, linting, type checking and tests.

.DESCRIPTION
    Runs all gates and reports a summary at the end rather than stopping at the
    first failure, so one run tells you everything that is broken.

.PARAMETER SkipFrontend
    Run only the Rust gates.

.PARAMETER SkipRust
    Run only the frontend gates.

.EXAMPLE
    ./scripts/test.ps1

.EXAMPLE
    ./scripts/test.ps1 -SkipFrontend -Verbose
#>
[CmdletBinding()]
param(
    [switch]$SkipFrontend,
    [switch]$SkipRust
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot

# The local Whisper engine is C++, so the Rust build needs an MSVC environment,
# libclang and a real ninja. Sourcing this here means a contributor never has to
# know that; see docs/adr/ADR-011.
. (Join-Path $PSScriptRoot 'env.ps1')
$results = [System.Collections.Generic.List[pscustomobject]]::new()

function Invoke-Gate {
    param([string]$Name, [scriptblock]$Command)

    Write-Host ""
    Write-Host "→ $Name" -ForegroundColor Cyan

    & $Command
    $ok = ($LASTEXITCODE -eq 0)

    if ($ok) {
        Write-Host "  ✓ $Name passed" -ForegroundColor Green
    } else {
        Write-Host "  ✗ $Name failed (exit $LASTEXITCODE)" -ForegroundColor Red
    }
    $results.Add([pscustomobject]@{ Name = $Name; Passed = $ok })
}

Push-Location $repoRoot
try {
    if (-not $SkipRust) {
        Invoke-Gate 'Rust format'  { cargo fmt --all --check }
        Invoke-Gate 'Rust clippy'  { cargo clippy --workspace --all-targets -- -D warnings }
        Invoke-Gate 'Rust tests'   { cargo test --workspace }
    }

    if (-not $SkipFrontend) {
        Invoke-Gate 'Prettier'          { pnpm format:check }
        Invoke-Gate 'TypeScript'        { pnpm --dir apps/desktop typecheck }
        Invoke-Gate 'Frontend tests'    { pnpm --dir apps/desktop test }
    }
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Summary" -ForegroundColor White
foreach ($result in $results) {
    $mark = if ($result.Passed) { '✓' } else { '✗' }
    $colour = if ($result.Passed) { 'Green' } else { 'Red' }
    Write-Host ("  {0} {1}" -f $mark, $result.Name) -ForegroundColor $colour
}

$failed = @($results | Where-Object { -not $_.Passed })
Write-Host ""
if ($failed.Count -gt 0) {
    throw "$($failed.Count) of $($results.Count) gate(s) failed."
}
Write-Host "All $($results.Count) gates passed." -ForegroundColor Green
Write-Host ""
