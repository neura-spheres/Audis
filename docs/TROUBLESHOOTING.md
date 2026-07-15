# Audis Troubleshooting & Known Limitations

## Known limitations (Milestone 0)

- **No audio, ASR, AI, export, updater, installer signing, or licensing yet.**
  These are Milestones 1-9. The UI intentionally exposes only what works today.
- **Placeholder brand assets.** The app icon (`src-tauri/icons/`) and the
  in-app wordmark are generated placeholders; final assets replace them.
- **Build machine MSVC is 2019 (14.29).** It links correctly, but upgrade to
  VS 2022 Build Tools before producing signed release builds.
- **Frontend toolchain pinned below the absolute latest.** Vite is pinned to 7.x
  and TypeScript to 5.9.x: Vite 8 (rolldown) dropped the bundled esbuild that
  Tailwind 4's plugin needs, and TS 7 (native) is ahead of the React/Vite type
  ecosystem. Revisit when those settle. See
  [ADR-001](adr/ADR-001-tauri-desktop-architecture.md).

## Common developer issues

**`cargo build` fails to link (`link.exe` not found).**
Install the MSVC C++ Build Tools and the Windows SDK, then restart the shell.
Verify with `rustup show` (target `x86_64-pc-windows-msvc`).

**`pnpm install` fails with `ERR_PNPM_NO_MATCHING_VERSION`.**
A pinned dependency version does not exist (npm `@types/*` packages version
independently from their runtime). Check the reported "latest release" and pin
that.

**`pnpm build` fails inside the Tailwind loader (`Cannot find package 'esbuild'`).**
You are on Vite 8. Audis targets Vite 7 for now; run `pnpm install` after
confirming `apps/desktop/package.json` pins `vite` to `7.x`.

**The app window is blank or does not open.**
Ensure the WebView2 Runtime is installed (evergreen; preinstalled on Windows
11). Check `%LOCALAPPDATA%\NeuraAudis\Audis\logs\audis.log`.

**Where are the logs?**
`%LOCALAPPDATA%\NeuraAudis\Audis\logs\audis.log.<date>` (the daily rolling
appender puts the date last). Raise verbosity with
`AUDIS_LOG=debug`. Logs never contain secrets or transcript text.

**Run against a scratch data directory.**
Set `AUDIS_DATA_DIR` to any path to relocate the entire data tree, used by the
test suite so no test touches your real profile.
