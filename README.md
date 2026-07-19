<div align="center">

# Audis

**Hear more. Understand faster.**

Windows audio intelligence by **Neura Audis**. Live captions, accurate
transcripts, and meeting assistance, with your microphone and your computer's
audio kept as separate sources.

</div>

---

> **Early development.** Audio capture, streaming transcription (local Whisper
> and cloud engines), live captions, the AI assistant, and provisional local
> speaker separation are working. Installer signing, licensing, and the richer
> speaker tools (rename/merge/split, saved voice profiles, post-session
> reconciliation) are not built yet. The UI deliberately exposes only what
> actually works, so there are no placeholder buttons.

## What Audis does

Audis captures your microphone and your system's playback audio as two
independent streams. That distinction is the whole design: your microphone is
you, and loopback is everyone else, so attribution is free and speaker
diarization only has to solve the harder half. From there a streaming ASR
pipeline produces live captions and transcripts, with an opt-in AI assistant on
top. Credentials live in the Windows Credential Manager, and nothing leaves the
machine unless you choose a cloud engine.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the design and
[docs/PRODUCT_SPEC.md](docs/PRODUCT_SPEC.md) for scope.

## Layout

```
audis/
├── apps/desktop/          Tauri 2 + React + TypeScript application
│   ├── src/               Frontend
│   └── src-tauri/         Rust application shell
├── crates/
│   └── audis-common/      Identity, paths, errors, IPC contracts
├── scripts/               PowerShell developer and release automation
├── docs/                  Documentation and architecture decision records
└── .github/workflows/     CI
```

Further crates (`audis-audio`, `audis-asr`, `audis-storage` and so on) are added
as the features that need them land.

## Prerequisites

| Tool             | Version       | Notes                         |
| ---------------- | ------------- | ----------------------------- |
| Windows          | 10 21H2+ / 11 | Only supported target         |
| Rust             | stable (MSVC) | `x86_64-pc-windows-msvc`      |
| MSVC Build Tools | 2019+         | C++ toolchain and Windows SDK |
| Node.js          | 22+           |                               |
| pnpm             | 10+           | `corepack enable`             |
| WebView2         | evergreen     | Preinstalled on Windows 11    |

## Getting started

```powershell
./scripts/setup.ps1     # check the environment, install dependencies
./scripts/dev.ps1       # run in development
./scripts/test.ps1      # format, lint, typecheck, tests
./scripts/build.ps1     # build a release binary
```

### Building

Use `./scripts/build.ps1` or `pnpm --dir apps/desktop tauri build`.

Do not build releases with `cargo build --release`. Cargo alone produces a
binary that still points at the Vite dev server, so it opens to a connection
error and shows a console window. `tauri build` is what bundles the frontend in.

## Design

The interface follows Apple's Human Interface Guidelines rendered honestly on
Windows: a strict type scale, hairline separators, layered neutral surfaces, one
accent colour, and no decorative motion. Audis runs for hours while you listen
to someone, and the UI should never compete with that. The foundation is in
[apps/desktop/src/styles/theme.css](apps/desktop/src/styles/theme.css).

## Privacy

Audis shows a visible indicator whenever it is listening, records only when you
ask, stores everything locally by default, and collects no analytics. See
[docs/PRIVACY.md](docs/PRIVACY.md) and [docs/SECURITY.md](docs/SECURITY.md).

## Licence

Proprietary. Copyright 2026 Neura Audis. See [LICENSE](LICENSE).
