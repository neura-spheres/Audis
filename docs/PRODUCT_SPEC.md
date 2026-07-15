# Audis Product Specification

**Audis by Neura Audis.** _Hear more. Understand faster._

The product definition engineering works against. Updated as features land.

## Identity

| Field           | Value                                         |
| --------------- | --------------------------------------------- |
| Application     | Audis                                         |
| Company / brand | Neura Audis                                   |
| Publisher       | Neura Audis                                   |
| Tagline         | Hear more. Understand faster.                 |
| Executable      | `Audis.exe`                                   |
| Installers      | `Audis-Setup-x64.exe` (NSIS), `Audis-x64.msi` |
| Bundle id       | `ai.neura.audis`                              |
| Protocol        | `audis://`                                    |
| Database        | `audis.db`                                    |
| Env prefix      | `AUDIS_`                                      |
| Data root       | `%LOCALAPPDATA%\NeuraAudis\Audis\`            |

## What Audis does

1. Captures microphone audio and system playback (loopback) as separate sources.
2. Transcribes both in real time through a streaming ASR pipeline.
3. Displays customizable live captions in independent windows.
4. Records and saves sessions to a searchable local library.
5. Separates remote speakers (diarization) on the computer-audio stream.
6. Generates summaries, action items, decisions and notes.
7. Detects questions and can suggest AI answers. Opt-in, text-first.
8. Lets the user ask AI about the ongoing conversation.
9. Exports transcripts, summaries, captions and recordings.
10. Supports cloud and local ASR engines, and multiple LLM providers.
11. Stores API credentials in the OS keystore.
12. Updates from signed GitHub Release artifacts.
13. Supports licensing without coupling the desktop app to a payment provider.

Windows only for the first release. Platform-specific code sits behind
interfaces so macOS and Linux stay possible without compromising Windows.

## ASR

Transcription is a proper Automatic Speech Recognition system. Product wording
is "Speech-to-Text", "Live Transcription" or "Live Captions"; the engine and
pipeline use ASR terminology internally (`AsrProvider`, `AsrStream`, `AsrError`).

The live pipeline is streaming-first:

```
capture -> normalize -> VAD -> endpointing -> streaming ASR
        -> interim result -> final result -> timestamp/source align
        -> speaker attribution -> caption + transcript storage
```

Batch transcription (`transcribe_file`) exists only for retranscription,
higher-accuracy post-processing, recovery after provider failure, and offline
files. It is never the live-caption path. See
[ADR-004](adr/ADR-004-asr-provider-abstraction.md) and
[AI_PIPELINE.md](AI_PIPELINE.md).

## Session modes

`LiveCaption`, `Transcription`, `MeetingAssistant`, `InterviewPractice`. One
shared session engine and state machine with four configurations, not four audio
implementations.

Interview Practice is for legitimate practice, accessibility and permitted use,
and shows a visible active-state indicator. Audis implements no covert recording
and no evasion features.

## Privacy stance

Audis always shows when it is listening or recording. There is no hidden
recording mode, no analytics by default, and no audio upload for analytics.
Recording is per-session and optional. Users are responsible for obtaining
consent where they record. See [PRIVACY.md](PRIVACY.md).

## Roadmap

Built: workspace, desktop shell, typed IPC, logging, tray, single instance, CI.

Planned, in rough order: WASAPI audio capture and the audio test window; the
session engine, controller chip and caption windows; ASR providers; the session
library and export; speaker diarization; the AI assistant; settings, privacy and
diagnostics; the updater and installers; licensing and hardening.

Known limitations are tracked in [../CHANGELOG.md](../CHANGELOG.md) and
[TROUBLESHOOTING.md](TROUBLESHOOTING.md).
