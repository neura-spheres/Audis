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

    The installers are signed for auto-update only when TAURI_SIGNING_PRIVATE_KEY
    is set, as it is in CI. Without it the installers are still built and install
    normally; only the update package that GitHub releases serve is left out,
    because it cannot be signed. See docs/releasing.md.

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

    # The updater key lives outside the repository. When it is not in the
    # environment (as it is in CI), the copy in the developer's profile signs the
    # installers after the build instead. It cannot go through the environment:
    # its password is empty, and Windows PowerShell deletes a variable set to
    # an empty string, which leaves tauri waiting on a hidden password prompt.
    $profileKey = Join-Path $env:USERPROFILE '.tauri\audis-updater.key'
    $signAfter = $Bundle -and -not $env:TAURI_SIGNING_PRIVATE_KEY -and (Test-Path $profileKey)

    $arguments = @('tauri', 'build')
    if (-not $Bundle) {
        $arguments += '--no-bundle'
    }
    elseif (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
        # Without the private key Tauri builds both installers and then fails
        # trying to sign the update package. Leaving that package out keeps the
        # installers, which need no key to install.
        if (-not $signAfter) {
            Write-Warning ('No updater signing key: building installers without the signed ' +
                'update package. Fine for installing; not for publishing a release.')
        }
        $localConfig = Join-Path ([System.IO.Path]::GetTempPath()) 'audis-unsigned-bundle.json'
        # WriteAllText writes UTF-8 without a byte-order mark, which the JSON
        # parser requires; Set-Content in Windows PowerShell would add one.
        [System.IO.File]::WriteAllText($localConfig, '{ "bundle": { "createUpdaterArtifacts": false } }')
        $arguments += @('--config', $localConfig)
    }

    Write-Host ''
    Write-Host '> Building Audis' -ForegroundColor Cyan
    Push-Location (Join-Path $repoRoot 'apps/desktop')
    try {
        # tauri reports progress on stderr, which Windows PowerShell 5.1 would
        # treat as a terminating error under 'Stop'. The exit code decides.
        $ErrorActionPreference = 'Continue'
        pnpm @arguments
        $buildExit = $LASTEXITCODE
        $ErrorActionPreference = 'Stop'
        if ($buildExit -ne 0) { throw "tauri build failed with exit code $buildExit." }

        if ($signAfter) {
            $version = (Get-Content -Raw 'src-tauri/tauri.conf.json' | ConvertFrom-Json).version
            $key = Get-Content -Raw $profileKey
            foreach ($installer in @(
                    "../../target/release/bundle/nsis/Audis_${version}_x64-setup.exe",
                    "../../target/release/bundle/msi/Audis_${version}_x64_en-US.msi")) {
                $ErrorActionPreference = 'Continue'
                pnpm tauri signer sign -k $key --password '""' $installer | Out-Null
                $signExit = $LASTEXITCODE
                $ErrorActionPreference = 'Stop'
                if ($signExit -ne 0) { throw "Signing $installer failed with exit code $signExit." }
                Write-Host "Signed $installer" -ForegroundColor Green
            }
        }
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
