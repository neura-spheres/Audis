# ADR-003: Separate microphone and loopback streams

**Status:** Accepted. Contract implemented, capture not yet.

## Context

A meeting has two kinds of participant: the local user, and everyone else. On
Windows those map exactly onto two capture paths, the microphone, and output
loopback. Many tools mix them into one stream and then try to recover "who
spoke" with diarization alone.

## Decision

Keep the two streams **completely independent through capture, buffering,
normalisation, ASR and storage**. Every frame, segment and level reading is
tagged with `AudioSourceKind` (`Microphone` | `ComputerAudio`). The microphone
is labelled **"You"**; loopback is **"Computer Audio"** until diarization
resolves it into `Person 1`, `Person 2`, ….

Diarization runs primarily on the computer-audio stream. It is not run on the
microphone unless the user explicitly captures multiple local speakers through
one microphone.

## Consequences

- Perfect, free attribution for the local user, no model can beat knowing which
  device the audio came from.
- Diarization solves a strictly smaller problem (remote speakers only), which
  measurably improves it.
- Two ASR sessions run concurrently, roughly doubling streaming cost when both
  sources are on. Accepted: attribution quality is the product.
- The merger must reconcile two clocks and two arrival orders (see
  `AI_PIPELINE.md`).
- Mixing early would be **unrecoverable**, so this is enforced by construction:
  capture emits source-tagged frames, and `AudioSourceKind` has no "mixed"
  variant.

## Alternatives considered

- **Mix then diarize:** one ASR stream and lower cost, but the local user
  becomes just another cluster, and echo makes clusters collide. Rejected.
