<#
.SYNOPSIS
    Validates the Audis development environment and installs dependencies.

.DESCRIPTION
    Checks Windows, Rust + the MSVC toolchain, Node, pnpm and the WebView2
    runtime, then installs frontend dependencies and runs a build sanity check.
    Reports every problem it finds rather than stopping at the first one, so a
    new machine can be fixed in a single pass.

.EXAMPLE
    ./scripts/setup.ps1

.EXAMPLE
    ./scripts/setup.ps1 -Verbose
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$problems = [System.Collections.Generic.List[string]]::new()

function Write-Step { param([string]$Message) Write-Host "→ $Message" -ForegroundColor Cyan }
function Write-Ok { param([string]$Message) Write-Host "  ✓ $Message" -ForegroundColor Green }
function Add-Problem {
    param([string]$Message, [string]$Fix)
    Write-Host "  ✗ $Message" -ForegroundColor Red
    $problems.Add("$Message`n     Fix: $Fix")
}

Write-Host ""
Write-Host "Audis, development environment setup" -ForegroundColor White
Write-Host "Neura Audis · Hear more. Understand faster." -ForegroundColor DarkGray
Write-Host ""

# ---- Windows ----------------------------------------------------------------
Write-Step "Checking Windows"
if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -ge 6) {
    Add-Problem "Audis targets Windows only." "Run this on Windows 10 21H2+ or Windows 11."
} else {
    $os = (Get-CimInstance Win32_OperatingSystem).Caption
    Write-Ok $os
}

# ---- Rust -------------------------------------------------------------------
Write-Step "Checking Rust"
if (Get-Command rustc -ErrorAction SilentlyContinue) {
    Write-Ok (rustc --version)
    $targets = rustup target list --installed 2>$null
    if ($targets -notcontains 'x86_64-pc-windows-msvc') {
        Add-Problem "The x86_64-pc-windows-msvc target is not installed." `
            "rustup target add x86_64-pc-windows-msvc"
    } else {
        Write-Ok "target x86_64-pc-windows-msvc"
    }
} else {
    Add-Problem "Rust is not installed." "Install from https://rustup.rs then reopen this shell."
}

# ---- MSVC linker ------------------------------------------------------------
# `rustup target` being present does not mean link.exe exists; a cargo build is
# the only honest check, so probe for the linker directly.
Write-Step "Checking the MSVC C++ toolchain"
$linker = Get-ChildItem -Path @(
    "${env:ProgramFiles}\Microsoft Visual Studio",
    "${env:ProgramFiles(x86)}\Microsoft Visual Studio"
) -Filter 'link.exe' -Recurse -ErrorAction SilentlyContinue |
    Where-Object FullName -like '*Hostx64\x64*' |
    Select-Object -First 1

if ($linker) {
    Write-Ok "link.exe found ($($linker.FullName))"
} else {
    Add-Problem "The MSVC linker (link.exe) was not found." `
        "Install 'Desktop development with C++' from the Visual Studio Build Tools."
}

# ---- Node -------------------------------------------------------------------
Write-Step "Checking Node.js"
if (Get-Command node -ErrorAction SilentlyContinue) {
    $nodeVersion = (node --version).TrimStart('v')
    if ([version]($nodeVersion -split '-')[0] -lt [version]'22.0.0') {
        Add-Problem "Node $nodeVersion is too old (need 22+)." "Install Node 22 LTS or newer."
    } else {
        Write-Ok "Node v$nodeVersion"
    }
} else {
    Add-Problem "Node.js is not installed." "Install Node 22 LTS from https://nodejs.org"
}

# ---- pnpm -------------------------------------------------------------------
Write-Step "Checking pnpm"
if (Get-Command pnpm -ErrorAction SilentlyContinue) {
    Write-Ok "pnpm $(pnpm --version)"
} else {
    Add-Problem "pnpm is not installed." "corepack enable && corepack prepare pnpm@latest --activate"
}

# ---- WebView2 ---------------------------------------------------------------
Write-Step "Checking the WebView2 runtime"
$webview2Keys = @(
    'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
    'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
)
$webview2 = $webview2Keys |
    ForEach-Object { Get-ItemProperty -Path $_ -Name pv -ErrorAction SilentlyContinue } |
    Select-Object -First 1

if ($webview2) {
    Write-Ok "WebView2 $($webview2.pv)"
} elseif (Test-Path "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application") {
    Write-Ok "WebView2 runtime present"
} else {
    Add-Problem "The WebView2 runtime was not found." `
        "Install the Evergreen runtime from https://developer.microsoft.com/microsoft-edge/webview2/ (preinstalled on Windows 11)."
}

# ---- Stop here if the environment is broken ---------------------------------
if ($problems.Count -gt 0) {
    Write-Host ""
    Write-Host "Setup found $($problems.Count) problem(s):" -ForegroundColor Red
    foreach ($problem in $problems) { Write-Host "  • $problem" -ForegroundColor Yellow }
    Write-Host ""
    throw "Environment is not ready. Fix the problems above and re-run ./scripts/setup.ps1"
}

# ---- Install ----------------------------------------------------------------
Write-Step "Installing frontend dependencies"
Push-Location $repoRoot
try {
    pnpm install
    if ($LASTEXITCODE -ne 0) { throw "pnpm install failed with exit code $LASTEXITCODE." }
    Write-Ok "dependencies installed"

    Write-Step "Build sanity check (cargo check)"
    cargo check --workspace
    if ($LASTEXITCODE -ne 0) { throw "cargo check failed with exit code $LASTEXITCODE." }
    Write-Ok "workspace compiles"
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Audis is ready. Next: ./scripts/dev.ps1" -ForegroundColor Green
Write-Host ""
