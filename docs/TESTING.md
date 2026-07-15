# Audis Testing

A feature is not done while its tests fail.

## What runs today

`cargo test --workspace` runs 16 Rust tests covering:

- Identity constants are populated and the event prefix matches the protocol
  scheme.
- The data directory layout matches the documented tree, `ensure_created` is
  idempotent, `session_dir` cannot escape the sessions directory, and `temp`
  shares the root volume so renames stay atomic.
- Every `UserFacingError` states whether data survived, explanations carry no
  developer jargon, diagnostic codes are stable and prefixed, and IO failures
  serialise as storage problems rather than config problems.
- `AudioSourceKind` labels are distinct and serialise as camelCase.
- `get_app_info` reports the right identity, a real version, and camelCase keys
  for the frontend.

`pnpm --dir apps/desktop test` runs 5 frontend tests covering:

- The Rust/TypeScript event-name contract. It parses `ipc.rs` and fails if the
  two lists drift apart.
- Event channels are prefixed and unique.
- `AboutView` renders backend identity through the validating IPC layer.
- `AboutView` renders the error card, including the reassurance line.

Static gates: `cargo fmt --check`, `cargo clippy -D warnings`, `tsc --noEmit`
under strict mode, and Prettier.

Run everything at once with `./scripts/test.ps1`.

## Planned

- **Audio fixtures.** Deterministic silence, single and alternating and
  overlapping speakers, mic plus computer audio, mixed sample rates, stereo and
  mono, clipping, and simulated device and network interruptions. No
  copyrighted recordings committed.
- **Provider contract tests.** Every ASR and AI adapter passes the same suite
  against mocked responses: auth failure, rate limit, timeout, malformed event,
  partial and final results, reconnection, cancellation, usage reporting, and
  structured-output failure.
- **Session state machine.** Valid and invalid transitions, and proof that
  finalization never loses a session because one task failed.
- **Repositories, transcript merging and ordering, question dedup, context
  compression, settings migration, export formatting, licence and update
  metadata.**
- **Windows integration.** Multiple microphones, USB and Bluetooth headsets,
  built-in mic, speakers, multi-monitor, display scaling, sleep and resume,
  device disconnect, default-device change.
- **Soak tests.** Two- and eight-hour runs with pause/resume, reconnection and
  device changes, recording memory, CPU, dropped frames and latency.

## Conventions

- Rust unit tests are colocated in `#[cfg(test)] mod tests`. `expect` and
  `unwrap` are allowed there and warned against everywhere else.
- Frontend tests use Vitest and Testing Library, with Tauri IPC mocked through
  `@tauri-apps/api/mocks` and fixtures shared from `src/test/fixtures.ts`.
- Tests must be deterministic and must never touch the real user profile. Use
  `AppPaths::rooted_at` or set `AUDIS_DATA_DIR`.
