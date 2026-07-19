# Changelog

All notable changes to Audis are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and Audis adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-07-18

### Added

- Professional session reports: generate a structured Markdown report
  (overview, key points, decisions, action items, open questions, quotes) from
  a saved transcript with your AI provider, saved to the exports folder.
- Speaker separation of the computer-audio stream: a local, offline diarizer
  (MFCC-statistics embeddings with delta features and online threshold
  clustering, no model download) that gives each distinct remote voice a
  provisional `Person N` label. Configurable in Settings ▸ Speakers, with a
  live roster. See
  [ADR-005](docs/adr/ADR-005-speaker-diarization-architecture.md).
- The "Ask the assistant" global shortcut now works: during a session with the
  assistant on, it answers the latest line immediately, bypassing question
  detection.
- Smarter assistant question detection (request verbs, request phrases and tag
  questions, in English and Indonesian) and wider, context-aware answering that
  reconstructs questions split or cut off across transcript lines.
- `config.yaml` at the repository root is the single source of truth for the
  app version; the build reads it and the app shows it everywhere.
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

- Installer signing and licensing are not implemented yet.
- Speaker separation is real-time and provisional. Renaming, merging and
  splitting speakers, saved voice profiles, and post-session reconciliation are
  not built yet, so a voice may occasionally be split or two similar voices
  merged.
- The application icon and wordmark are placeholders pending final brand assets.
- Building the local Whisper engine needs a C++ toolchain: VS 2022 Build Tools,
  cmake 4.x, Ninja and libclang, and the repo must live in a short path because
  MSVC still enforces a 260-character limit. See
  [ADR-011](docs/adr/ADR-011-local-whisper-asr-engine.md). None of this affects
  people who install Audis; it is build-time only.

[0.1.1]: https://github.com/neura-audis/audis-desktop
