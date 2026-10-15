<#
.SYNOPSIS
    Writes latest.json, the update manifest a release must carry for in-app updates.

.DESCRIPTION
    Run after ./scripts/build.ps1 -Bundle with the signing key available. Reads
    the version from tauri.conf.json, the NSIS installer's signature from the
    bundle folder, and the release notes from CHANGELOG.md, and writes
    target/release/bundle/latest.json. Upload it to the GitHub release next to
    the installer; Audis finds it there by name.

    CI produces the same file through tauri-action when the signing key is set
    as a repository secret. See docs/releasing.md.
#>
[CmdletBinding()]
param(
    [string]$Repository = 'neura-spheres/Audis'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$config = Get-Content -Raw (Join-Path $repoRoot 'apps/desktop/src-tauri/tauri.conf.json') | ConvertFrom-Json
$version = $config.version

$installerName = "Audis_${version}_x64-setup.exe"
$bundleDir = Join-Path $repoRoot 'target/release/bundle'
$signaturePath = Join-Path $bundleDir "nsis/$installerName.sig"
if (-not (Test-Path $signaturePath)) {
    throw "No signature at $signaturePath. Build with ./scripts/build.ps1 -Bundle and the signing key."
}
$signature = (Get-Content -Raw $signaturePath).Trim()

# The notes shown in About are this version's CHANGELOG section.
$changelog = (Get-Content -Raw (Join-Path $repoRoot 'CHANGELOG.md')) -replace "`r`n", "`n"
$start = $changelog.IndexOf("## [$version]")
if ($start -lt 0) { throw "CHANGELOG.md has no section for $version." }
$body = $changelog.Substring($start).Split("`n", 2)[1]
$next = $body.IndexOf("`n## [")
if ($next -ge 0) { $body = $body.Substring(0, $next) }

$url = "https://github.com/$Repository/releases/download/v$version/$installerName"
$platform = @{ signature = $signature; url = $url }
$manifest = [ordered]@{
    version  = $version
    notes    = $body.Trim()
    pub_date = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    # The updater looks for the installer-specific key first, then the generic one.
    platforms = [ordered]@{
        'windows-x86_64-nsis' = $platform
        'windows-x86_64'      = $platform
    }
}

$out = Join-Path $bundleDir 'latest.json'
# UTF-8 without a byte-order mark: the updater's JSON parser rejects one.
[System.IO.File]::WriteAllText($out, ($manifest | ConvertTo-Json -Depth 5))
Write-Host "Wrote $out for $version" -ForegroundColor Green
