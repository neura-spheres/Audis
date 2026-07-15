# Audis Privacy

Audis is a listening tool. That imposes obligations, and the product is built
around them rather than bolting them on.

## Principles

1. **Always visible when listening.** Audis shows a clear active-state
   indicator whenever it captures audio. There is no hidden recording mode and
   no stealth-from-screen-recording feature.
2. **Local by default.** Recordings, transcripts, models, logs and the database
   are stored on the user's PC under `%LOCALAPPDATA%\NeuraAudis\Audis`. Nothing
   is uploaded unless the user selects a cloud engine.
3. **Cloud is explicit and legible.** Before a session, Audis states whether
   audio or text will leave the device, whether transcription is local or
   cloud, and whether recording is on.
4. **Recording is optional and per-session.** The user chooses each time.
5. **No analytics by default.** Analytics and crash reporting are opt-in. Audio
   is never uploaded for analytics.
6. **The user owns their data.** They can delete a session and its files,
   delete all local history, and export their data. Expiry of a licence never
   deletes or locks user files.
7. **Consent is the user's responsibility, and Audis reminds them.** Onboarding
   states that the user is responsible for obtaining any consent required where
   they record.

## What is stored, and where

| Data            | Location                         | Default retention |
| --------------- | -------------------------------- | ----------------- |
| Sessions & meta | `sessions\`, `database\audis.db` | Until deleted     |
| Recordings      | `recordings\`                    | Off unless chosen |
| Transcripts     | `sessions\{id}\transcript.*`     | Until deleted     |
| Models          | `models\`                        | Until removed     |
| Logs            | `logs\` (rolling, secret-safe)   | Rolling limit     |
| Exports         | `exports\` (or chosen location)  | User-managed      |

Retention controls, "delete all local data", and data export are delivered in
Milestone 7.

## What is never stored or logged

API keys, authorization headers, full provider payloads containing transcripts,
raw audio, voice profiles, résumé text, and job-description text are never
written to normal logs, exports, or crash reports. Diagnostic bundles redact
secrets and redact transcript text by default, and show the user what will be
included before export.

## Voice profiles

Optional speaker enrollment requires an explicit user action to save a voice
profile, and provides deletion and export controls. Audis never infers
identity, age, gender, ethnicity, or personality from voice.
