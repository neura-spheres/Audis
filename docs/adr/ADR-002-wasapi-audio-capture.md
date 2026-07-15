# ADR-002: WASAPI audio capture

**Status:** Accepted. Not yet implemented.

## Context

Audis must capture the microphone **and** everything the computer plays, on
ordinary Windows machines, without asking users to install anything extra.

## Decision

Use **Windows WASAPI** in shared mode with event-driven capture, accessed from
Rust. Prefer a reliable WASAPI abstraction where it suffices, and drop to the
`windows` crate directly for APIs the abstraction does not expose correctly. All
Windows-specific code is isolated in `audis-audio-windows` behind the
platform-neutral interfaces in `audis-audio`.

System audio uses **WASAPI loopback** on the output device. Audis will **not**
require a virtual audio cable and will **not** ship a custom audio driver for
the initial product. Process-selective capture stays behind a feature flag until
validated across supported Windows versions.

## Consequences

- Works on stock Windows with no user setup, which is the key adoption property.
- Loopback captures the whole output device, so any app's audio is included;
  selective capture is a later, flagged capability.
- Using speakers can duplicate remote speech across loopback and microphone;
  handled explicitly (see `AUDIO_PIPELINE.md`), never by silently dropping
  uncertain speech.
- Windows-only for now; the interface split keeps macOS/Linux possible.

## Alternatives considered

- **Virtual audio cable:** reliable but demands user installation and breaks
  their default audio setup. Rejected.
- **Custom driver:** best capability, requires WHQL signing and a large support
  burden. Rejected for v1.
- **cpal only:** convenient, but its loopback and device-event coverage on
  Windows is insufficient for this product. Rejected as the sole layer.
