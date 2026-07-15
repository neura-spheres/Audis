# Contributing to Audis

## Ground rules (from the build spec)

1. Inspect files before changing them; don't overwrite useful work or delete
   files just to make a build pass.
2. Small, testable modules. No god-files: not everything in `main.rs`, not
   everything in one React component.
3. No `unwrap()`/`expect()` on production Rust paths without a documented
   invariant. No suppressed Clippy warnings without justification.
4. No `any` in TypeScript without a documented boundary. Strict mode stays on.
5. Validate every IPC payload with a schema. Keep audio callbacks real-time
   safe. Keep secrets out of the frontend and out of logs.
6. Keep provider-specific code behind adapters. Use feature flags for
   incomplete/experimental capabilities.
7. Add a test for every meaningful bug fix. Update docs when architecture
   changes. Keep the known-limitations list current.
8. No fake buttons in shipped UI. Don't claim a milestone complete while its
   required tests fail. Never promise perfect diarization/transcription.

## Before you push

```powershell
./scripts/test.ps1     # fmt + clippy + typecheck + all tests
```

Equivalently:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir apps/desktop typecheck
pnpm --dir apps/desktop test
pnpm format:check
```

## Commit and branch conventions

- Work on a branch; do not commit directly to the default branch.
- Conventional-commits style is encouraged (`feat:`, `fix:`, `docs:`, …).
- Reference the milestone where relevant.

## Architecture decisions

Non-trivial decisions get an ADR in `docs/adr/`. Copy the format of the existing
records: context, decision, consequences, and the alternatives considered.

## Dependency direction

Low-level crates never depend on `audis-desktop`. New crates depend only on
`audis-common` and on crates lower in their own domain. See
[`ARCHITECTURE.md`](ARCHITECTURE.md).
