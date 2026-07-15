# Audis Release Process

> **Status: planned.** CI gates every change today. Installer signing and the
> release pipeline are not built yet.

## Versioning

Semantic versioning. Stable tags `vX.Y.Z`; beta tags `vX.Y.Z-beta.N`. The
version in `Cargo.toml` (workspace), `apps/desktop/package.json`, and
`tauri.conf.json` must agree, the release pipeline verifies this and fails
closed on a mismatch.

## Channels

`stable` and `beta`, with separate update metadata. Clients on beta never
auto-adopt a stable-only release and vice versa.

## Release pipeline (Milestone 8)

Triggered by a version tag. It must:

1. Verify version consistency across manifests.
2. Run frontend checks (format, lint, typecheck, tests).
3. Run Rust `fmt --check`, `clippy -D warnings`, unit and integration tests.
4. Build the Windows release binary.
5. Produce NSIS (`Audis-Setup-x64.exe`) and MSI (`Audis-x64.msi`) bundles.
6. Produce and **sign** Tauri updater artifacts; apply Windows code signing when
   credentials are present. **Fail closed** if signing is missing for a
   production release.
7. Generate `SHA256SUMS.txt`, SBOMs, and release notes.
8. Create a **draft** GitHub Release first, upload all assets, validate their
   presence, and only then publish.

## Release assets

```
Audis-Setup-x64.exe        Audis-Setup-x64.exe.sig
Audis-x64.msi              Audis-x64.msi.sig
latest.json (per channel)  SHA256SUMS.txt
release notes              SBOM files
```

## Updates on the client

The installed client uses the **official Tauri updater** only. It checks
metadata, compares semver, shows release notes, downloads the signed artifact,
**verifies the signature against a pinned public key**, installs (with user
approval unless auto-install is configured), restarts, records the result, and
runs a post-update health check. Audis never updates by downloading and running
an arbitrary script, and never treats a GitHub release name as proof of
authenticity.

## Local automation

PowerShell scripts orchestrate building and publishing but are **not** the
installer. See `scripts/`: `build.ps1`, `package.ps1`, `release.ps1`,
`verify-release.ps1`. They use strict error handling, check exit codes, avoid
destructive behaviour, and keep secrets out of command history.
