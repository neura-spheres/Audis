# Audis Audio Pipeline

> **Status: planned.** The `AudioSourceKind` contract and the data layout
> exist; capture code does not. This is the plan the audio crates are built
> against.

## The one invariant

Microphone audio and computer-playback audio are **never mixed before source
attribution**. They flow as two independent streams:

```
Microphone (WASAPI capture)        →  source = Microphone     → "You"
Computer output (WASAPI loopback)  →  source = ComputerAudio  → "Computer Audio"
```

Diarization may later refine `ComputerAudio` into `Person 1`, `Person 2`, … but
the microphone stream stays the local user. Mixing early is unrecoverable, so it
is prohibited by construction: capture produces `AudioSourceKind`-tagged frames.

## Per-source pipeline

```
WASAPI capture callback (real-time)
    ↓  copy/move frames only, no allocation, no I/O, no locks, no network
lock-free bounded ring buffer
    ↓
format conversion → channel conversion → resampling
    ↓
level analysis (peak, RMS, clipping, silence)
    ↓
voice activity detection → endpointing
    ↓
timestamping (monotonic clock → session-relative ms)
    ↓
fan-out: streaming ASR · optional recording writer · optional diarization
```

## Capture callback rules (real-time safety)

The WASAPI callback must **never**: allocate on the heap (where avoidable), call
a provider, touch SQLite, wait on a mutex held by non-audio work, update the
frontend, run ASR, or write large files synchronously. It copies or moves frames
into a bounded ring buffer and returns.

On overrun: record a diagnostic event, increment a dropped-frame counter,
continue when safe, and surface a non-disruptive warning only if it persists.

## Formats and clocks

Device formats are arbitrary; speech inference streams are normalised to an
engine-appropriate representation (commonly mono, 16 or 24 kHz, PCM/float). No
provider sample-rate is assumed. Timestamps come from a monotonic clock and are
converted to session-relative milliseconds so the two streams stay synchronised
despite differing sample rates and callback cadences.

## Devices

Enumeration, stable IDs, friendly names, default communication vs multimedia
devices, format detection, hotplug and default-change detection, and
disconnect/sleep recovery. When a selected device disappears: pause that source,
keep the rest of the session running, inform the user, offer the new default,
and reconnect without restarting the app where safe.

## System (loopback) capture

Version 1 captures full output-device loopback and never requires a virtual
audio cable or a custom driver. Process-tree/selective capture stays behind a
feature flag until tested across supported Windows versions.

## Echo and duplicate speech

When the user is on speakers, remote speech can appear in both loopback and the
microphone. Audis detects likely duplication (time-aligned similarity /
cross-correlation), recommends headphones, shows an echo-risk indicator, and
offers optional suppression, but never silently removes uncertain speech;
low-confidence decisions are marked for diagnostics.

## Level events

Meter updates are throttled to ~20-30/sec, never one event per frame, and
carry `{ source_id, peak, rms, clipping, silence_duration_ms }`.
