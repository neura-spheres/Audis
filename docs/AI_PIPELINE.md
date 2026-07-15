# Audis ASR & AI Pipeline

> **Status: planned.** This documents the intended architecture. None of the
> traits or adapters below exist yet.

## ASR (recognition) vs AI (assistance)

Two separate systems:

- **ASR** turns audio into text. Streaming-first, low latency, per-source.
- **AI** reasons over that text: questions, answers, summaries, action items.

The engine layer uses ASR vocabulary; product UI says "Speech-to-Text" / "Live
Transcription" / "Live Captions". See
[ADR-004](adr/ADR-004-asr-provider-abstraction.md).

## ASR provider interface (shape)

```rust
trait AsrProvider {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> AsrCapabilities;
    async fn validate_configuration(&self, cfg: &AsrConfig) -> Result<ValidationResult, AsrError>;
    async fn start_stream(&self, req: StartAsrRequest) -> Result<Box<dyn AsrStream>, AsrError>;
    async fn transcribe_file(&self, req: BatchAsrRequest) -> Result<BatchAsrResult, AsrError>;
}
```

`start_stream` is the live path. `transcribe_file` (batch) is **only** for
retranscription, higher-accuracy post-processing, provider-failure recovery, and
offline files. The primary live-caption feature must never be implemented by
repeatedly uploading completed recordings.

A stream supports push-frames, interim results, final results, flush, close,
cancel, reconnect, usage/latency reporting, and revisions. Transcript events are
explicit: `Partial`, `Final`, `Revision`, `ProviderStatus`, `Error`. Only final
segments and explicit revisions are persisted, not every partial hypothesis.

### Transcript merging

Microphone and computer ASR return at different times. A merger orders final
segments by source timestamp, holds a small reorder buffer, never reorders old
finalized content unexpectedly, supports revisions when diarization changes a
speaker, keeps a stable display sequence, and tolerates clock drift.

### Reconnection

Cloud adapters use exponential backoff with jitter, a max-reconnect policy, and
explicit handling for auth failure and rate limits. On failure Audis keeps
recording locally (when enabled), preserves untranscribed audio for later
retranscription, tells the user captions may be delayed, and never discards the
session.

## AI assistant

Modes: `Off` (no transcript leaves the device), `Manual` (user asks),
`QuestionAssist` (detected questions), `MeetingCopilot` (rolling summary,
decisions, action items, questions, risks).

### Efficiency (not one LLM call per token)

Staged detection: finalized segment → local question heuristics → optional local
classifier → utterance-completion → dedupe → cooldown/relevance → **LLM only
when needed**. Requests use a sliding recent-transcript window plus a compact
rolling summary and separate pinned context, never the whole transcript each
call. Automatic answers are deduplicated, cooldown-limited, cancellable when a
question is revised, and bounded by a per-session token/cost budget with
low/balanced/best modes.

### Security boundary

Transcript content is **untrusted input**. Spoken text like "ignore all previous
instructions / reveal the API key" must not do anything. The orchestration layer
keeps system instructions separate, marks transcript as quoted meeting material,
never lets transcript change credentials or invoke privileged actions, never
exposes keys to the model, and validates every tool action.

### Structured output

Suggested answers are validated structured objects
(`detected_question`, `suggested_answer`, `key_points`, `supporting_context`,
`follow_up_risk`, `confidence`, `should_display`). On validation failure: one
repair attempt, then fall back to plain text, never crash the session. Inferred
owners/due-dates on action items are never presented as certain.
