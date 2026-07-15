# Audis Database

> **Status: planned.** The location and filename are fixed; no schema exists yet.

## Engine and settings

SQLite (bundled via `rusqlite`) with foreign keys on, WAL mode where suitable,
a busy timeout, migrations, transactions, parameterized statements only, and
repository abstractions. The frontend never sees raw SQL; it calls typed
commands. Corruption is detected and surfaced; backups are supported.

## Location

```
%LOCALAPPDATA%\NeuraAudis\Audis\database\audis.db   (+ -wal, -shm)
```

## Planned tables

```
schema_migrations   sessions            audio_sources
transcript_segments transcript_revisions speakers
speaker_profiles    session_speakers    summaries
decisions           action_items        bookmarks
ai_messages         ai_requests         provider_usage
session_files       tags                session_tags
settings_metadata   update_history      license_cache
audit_events
```

**No API secrets are ever stored in these tables.** Secrets live in the OS
keystore; the database and settings hold only a `credential_ref`.

## Session files on disk

```
sessions\{session_uuid}\
    session.json        microphone.audio     computer.audio
    combined-preview.audio  transcript.json  transcript.txt
    summary.json        recovery.json        attachments\   exports\
```

Written with temp-file-then-atomic-rename where practical (`temp\` shares the
data volume). Recording container choice is documented when Milestone 1/2 lands.

## Crash recovery

During a session Audis writes a recovery journal, commits transcript segments
incrementally, finalizes recording chunks periodically, and tracks provider
sequence and the last safe timestamp. On restart it detects incomplete sessions,
offers recovery, preserves recoverable audio, marks incomplete sections, and
never silently deletes an interrupted session.

## Search

Local full-text transcript search via SQLite FTS: by phrase, speaker, date,
session, and tag, with jump-to-timestamp.
