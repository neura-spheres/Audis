# ADR-007: SQLite storage

**Status:** Accepted. Not yet implemented.

## Context

Audis stores sessions, transcript segments, speakers, summaries, bookmarks and
usage locally, must search transcripts quickly, and must survive crashes
mid-session without losing an hour of recording.

## Decision

Use **SQLite** (bundled with `rusqlite`, so no system dependency) at
`%LOCALAPPDATA%\NeuraAudis\Audis\database\audis.db`, with foreign keys on, WAL
mode where suitable, a busy timeout, versioned migrations, transactions, and
**parameterized statements only**. Access goes through repository abstractions;
**raw SQL is never exposed to the frontend**.

Full-text transcript search uses **SQLite FTS**. Large binaries (audio,
attachments) stay on the filesystem under `sessions\{uuid}\`, not in the
database; files are written temp-then-atomic-rename, with `temp\` on the same
volume.

Crash safety: a recovery journal plus incremental segment commits and periodic
recording finalization. On restart, incomplete sessions are detected and
offered for recovery, never silently deleted.

**No API secrets are stored in the database** (see [ADR-006](ADR-006-api-key-storage.md)).

## Consequences

- Zero-install, single-file, transactional local storage with good FTS.
- WAL means sidecar files (`-wal`, `-shm`) must be handled by backup/export.
- Concurrency is limited to one writer; the audio path never touches SQLite
  so this is never on a real-time path.
- Migrations must be forward-only and tested, user data cannot be recreated.

## Alternatives considered

- **Files only (JSON per session):** simple, but no real search and no
  transactions. Rejected.
- **Embedded server DB (Postgres/DuckDB):** heavier install, no benefit at this
  scale. Rejected.
