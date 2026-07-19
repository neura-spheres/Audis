# ADR-005: Speaker diarization architecture

**Status:** Accepted. Partially implemented: real-time provisional labelling of
the computer-audio stream is built with a local, offline MFCC-embedding +
online-clustering diarizer (`crates/audis-asr/src/diarize.rs`). Rename/merge/
split, saved voice profiles, and post-session reconciliation are not built yet.

## Context

Audis must show _who_ said what among remote participants, in near-real-time,
while being honest that real-time diarization is uncertain.

## Decision

Diarize the **computer-audio stream only** (per [ADR-003](ADR-003-separate-microphone-and-loopback-streams.md);
the microphone is already known to be "You"). Pipeline:

```
computer audio → VAD segmentation → speaker embedding extraction
→ online/near-real-time clustering → provisional labels
→ offline final reconciliation after the session
```

Labels are `Person 1`, `Person 2`, …. Identification uses **speaker embeddings
and clustering**, never voice pitch/"depth" as the primary identity signal.
Audis never infers identity, age, gender, ethnicity, or personality from voice.

Real-time labels are **provisional** and may change; the UI must show them as
provisional, and revisions are emitted as explicit events. Users can rename,
merge, split, and reassign; corrections persist and are auditable/reversible.
Overlapping speech is represented explicitly (`Person 1 + Person 2`) rather than
forced into one speaker when confidence is poor.

Speaker enrollment is optional and requires explicit user action to save an
(encrypted) voice profile, with deletion and export controls.

## Consequences

- A smaller, better-posed problem than diarizing a mixed stream.
- The UI and data model must both tolerate labels changing after the fact
  hence `transcript_revisions` and provisional styling.
- Post-session reconciliation gives a better final transcript than the live view.
- Accuracy is never promised as perfect.

## Alternatives considered

- **Provider-side diarization only:** simple, but unavailable/inconsistent
  across engines and impossible offline. Kept as an optional input, not the
  architecture.
- **Pitch-based speaker ID:** cheap and unreliable, and drifts toward inferring
  demographics. Explicitly rejected.
