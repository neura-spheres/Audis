# ADR-001: Tauri 2 desktop architecture

**Status:** Accepted. Implemented.

## Context

Audis is a commercial Windows desktop app that must do heavy native work (WASAPI
capture, local inference) while presenting a polished, accessible UI. We need a
native Rust core and a modern web UI in one distributable, with a small binary
and strong security defaults.

## Decision

Use **Tauri 2** with a Rust backend and a React + TypeScript frontend rendered
in **WebView2**. The Rust core owns all session and provider logic; the frontend
is a view/controller that talks to Rust over typed, schema-validated IPC.

Toolchain pins (Milestone 0): React 19, TypeScript 5.9, **Vite 7**, Tailwind 4,
Vitest 4, `zod` 4, Zustand 5. Vite is held at 7 rather than 8 because Vite 8's
rolldown build dropped the bundled esbuild that Tailwind 4's plugin depends on;
TypeScript is held at 5.9 because TS 7 (native) is ahead of the React/Vite type
ecosystem. Both are revisited as the ecosystem catches up.

## Consequences

- Small installer, native performance, one language boundary to manage.
- WebView2 is required at runtime (evergreen; preinstalled on Windows 11, and
  the installer bootstraps it otherwise).
- The IPC boundary must be disciplined: every argument validated in Rust, every
  result validated in TypeScript. This is enforced by `zod` schemas and a
  Rust↔TS event contract test.
- Heavy AI/audio work must never run in the WebView.

## Alternatives considered

- **Electron:** far larger binaries, Node in the trust boundary, weaker native
  story. Rejected.
- **Native WinUI/WPF:** excellent native fit, but slower UI iteration and no
  cross-platform path. Rejected for first release.
