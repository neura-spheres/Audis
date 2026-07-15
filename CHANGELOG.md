# Changelog

All notable changes to Audis are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Audis adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace with the `audis-common` base crate: identity, Windows data
  paths, error types with a mandatory user-facing presentation, IPC contracts.
- pnpm workspace and the Tauri 2 + React + TypeScript desktop application.
- Main window shell with a source-list sidebar, Dashboard and About views.
- System-tray icon with open and quit, single-instance enforcement, and
  focus-existing-window behaviour on relaunch.
- Structured logging to `%LOCALAPPDATA%\NeuraAudis\Audis\logs` with a
  per-launch correlation id and secret-safe defaults.
- Typed, schema-validated IPC boundary using zod, plus a contract test that
  fails if the Rust and TypeScript event lists drift apart.
- Design foundation with light and dark theming.
- GitHub Actions CI for Rust fmt, clippy and tests, and frontend format,
  typecheck, tests and build.
- PowerShell scripts: `setup`, `dev`, `test`, `build`, `clean`.
- Documentation and the first ten architecture decision records.

### Known limitations

- Audio capture, transcription, diarization, the AI assistant, export, the
  updater, installer signing and licensing are not implemented yet.
- The application icon and wordmark are placeholders pending final brand assets.
- Building the local Whisper engine needs a C++ toolchain: VS 2022 Build Tools,
  cmake 4.x, Ninja and libclang, and the repo must live in a short path because
  MSVC still enforces a 260-character limit. See
  [ADR-011](docs/adr/ADR-011-local-whisper-asr-engine.md). None of this affects
  people who install Audis; it is build-time only.

[Unreleased]: https://github.com/neura-audis/audis-desktop
