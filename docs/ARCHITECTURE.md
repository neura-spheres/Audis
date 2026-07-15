# Audis Architecture

This document describes how Audis is built **today** and the shape it grows
into. It is kept honest: sections describing unbuilt milestones say so.

## Guiding principles

1. **The Rust core owns the truth.** The frontend renders state and issues
   commands; it never owns an audio session, a provider key, or a transcript.
2. **The frontend is not a trust boundary.** Every IPC argument is re-validated
   in Rust, and every Rust result is schema-validated in TypeScript.
3. **Microphone and computer audio never mix before attribution.** This one
   invariant makes speaker separation recoverable; losing it is unrecoverable.
4. **Audio callbacks are real-time.** No allocation, no I/O, no locks held by
   non-audio work, no networking, just move frames into a bounded buffer.
5. **Low-level crates never depend on the app.** Dependencies point inward, so
   audio, ASR and storage stay testable without a desktop shell.
6. **Secrets live in the OS keystore, handled only in Rust.** They never reach
   the frontend, logs, exports, or crash reports.

## Layers

```
┌──────────────────────────────────────────────────────────────┐
│                 React + TypeScript UI (WebView2)               │
│  Main window · Controller chip · Captions · Assistant · Test   │
└───────────────────────────┬──────────────────────────────────┘
                            │ Typed, schema-validated Tauri IPC
┌───────────────────────────▼──────────────────────────────────┐
│                     Audis Application Core (Rust)              │
│    Session state machine · Commands · Settings · Entitlements  │
└──────────┬────────────────┬────────────────┬─────────────────┘
           │                │                │
┌──────────▼──────┐ ┌───────▼────────┐ ┌─────▼───────────────┐
│  Audio Engine   │ │  Intelligence  │ │  Storage & Export   │
│ mic · loopback  │ │ VAD · ASR ·    │ │ SQLite · sessions · │
│ resample · meter│ │ diarize · AI   │ │ search · export     │
└──────────┬──────┘ └───────┬────────┘ └─────────────────────┘
           │                │
┌──────────▼────────────────▼──────────────────────────────────┐
│        Provider adapters & local engines                      │
│  OpenAI · Gemini · Anthropic · DeepSeek · Local · Custom       │
└──────────────────────────────────────────────────────────────┘
```

## Crate map

Built in Milestone 0:

| Crate           | Responsibility                                             |
| --------------- | ---------------------------------------------------------- |
| `audis-common`  | Identity, Windows paths, error types, IPC contracts. Base. |
| `audis-desktop` | Tauri shell: commands, tray, logging, single-instance.     |

Planned, added as the features that need them land:

| Crate                 | Milestone | Responsibility                           |
| --------------------- | --------- | ---------------------------------------- |
| `audis-audio`         | 1         | Platform-neutral capture interfaces.     |
| `audis-audio-windows` | 1         | WASAPI capture, loopback, device events. |
| `audis-dsp`           | 1         | Resampling, level analysis, ring buffer. |
| `audis-asr`           | 3         | Streaming + batch ASR provider traits.   |
| `audis-diarization`   | 5         | Speaker embedding and clustering.        |
| `audis-ai`            | 6         | AI orchestration and context manager.    |
| `audis-providers`     | 3/6       | Concrete provider adapters.              |
| `audis-storage`       | 2/4       | SQLite, migrations, repositories, FTS.   |
| `audis-security`      | 7         | OS keystore, redaction, secret memory.   |
| `audis-export`        | 4         | Exporters (txt, md, json, srt, vtt, …).  |
| `audis-updater`       | 8         | Signed Tauri updater integration.        |
| `audis-licensing`     | 9         | Entitlement verification, offline grace. |
| `audis-test-support`  | 1+        | Deterministic fixtures and fakes.        |

The dependency rule: every crate above may depend on `audis-common` and on
crates lower in its own domain, but **nothing depends on `audis-desktop`**.

## IPC contract

Event channel names are defined once in `audis_common::ipc::events` and
mirrored in `apps/desktop/src/services/events.ts`. A Vitest contract test parses
the Rust source and fails if the two lists diverge, so a renamed channel is
caught at test time, not by captions silently vanishing.

Command results cross the boundary as concrete types; the frontend parses every
one with a `zod` schema in `apps/desktop/src/schemas/`. A shape mismatch throws
a typed `AudisIpcError` at the boundary rather than surfacing as `undefined`
inside a component.

## Errors

`audis_common::AudisError` is the engineering error type. It serialises, and is
converted at the UI boundary, into `UserFacingError`, which always states
whether the user's data survived, gives one suggested next step, and carries a
stable `AUDIS-*` diagnostic code. Stack traces and file paths never reach the
UI. This is implemented and tested in `crates/audis-common/src/error.rs`.

## Windows data layout

Everything large or machine-specific lives under `%LOCALAPPDATA%`:

```
%LOCALAPPDATA%\NeuraAudis\Audis\
    database\audis.db     sessions\   recordings\   models\
    cache\                logs\       updates\      exports\   temp\
```

`temp\` shares the data volume so that write-then-rename stays atomic.
`%APPDATA%` is reserved for small roaming preferences only. The layout is
implemented and tested in `crates/audis-common/src/paths.rs`; the override
`AUDIS_DATA_DIR` supports portable installs and hermetic tests.

## Security posture (Milestone 0)

- Restrictive Tauri capabilities: only `core:default`, single window, no shell,
  no arbitrary filesystem or HTTP from the frontend.
- Strict CSP: `default-src 'self'`, no remote scripts, `object-src 'none'`.
- Prototype freeze enabled; asset protocol disabled until a feature needs it.
- DevTools compiled out of release builds (behind the `devtools` feature).

Deeper security work (keystore, redaction, sidecar auth, update signature
pinning) lands with Milestones 7-9. See [`SECURITY.md`](SECURITY.md).
