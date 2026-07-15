# ADR-004: ASR provider abstraction (and ASR vs transcript naming)

**Status:** Accepted. Not yet implemented.

## Context

Audis must support several recognition engines (OpenAI, Gemini, local Whisper,
later Azure/Deepgram/AssemblyAI/custom) without leaking any one vendor's model
into the core. The build spec was also amended to require that transcription be
implemented as a **proper ASR system**, streaming-first, with interim and final
results, rather than by uploading completed recordings.

## Decision

**One provider-neutral trait, ASR vocabulary, streaming-first.**

```rust
trait AsrProvider {
    fn id(&self) -> &'static str;
    fn capabilities(&self) -> AsrCapabilities;
    async fn validate_configuration(&self, cfg: &AsrConfig) -> Result<ValidationResult, AsrError>;
    async fn start_stream(&self, req: StartAsrRequest) -> Result<Box<dyn AsrStream>, AsrError>;
    async fn transcribe_file(&self, req: BatchAsrRequest) -> Result<BatchAsrResult, AsrError>;
}
```

The crate is `audis-asr` (not `audis-transcription`).

**Naming rule, the engine is ASR, the artifact is the transcript.**

- Internal recognition layer uses ASR terms: `AsrProvider`, `AsrStream`,
  `AsrError`, `AsrConfig`, and the `audis://asr/status` event.
- Things the engine _produces_ are transcript segments, and the events carrying
  them to the UI keep artifact naming: `audis://transcript/partial`,
  `/final`, `/revision`.
- Product UI never says "ASR". It says **Speech-to-Text**, **Live
  Transcription**, or **Live Captions**.

**Streaming is the live path; batch is for recovery only.** `transcribe_file`
exists solely for retranscribing saved recordings, higher-accuracy post-session
processing, recovery after provider failure, final correction, and offline
files. Implementing live captions via repeated batch upload is prohibited.

Capabilities are declared per provider and must be **verified**, not asserted
Audis will not route audio to a provider configured only for text.

## Consequences

- Adding an engine means implementing one trait and passing the shared adapter
  contract test suite (auth failure, rate limit, timeout, malformed event,
  partial/final, reconnect, cancel, usage, structured-output failure).
- Two vocabularies coexist. The boundary is crisp (engine vs artifact) and is
  documented here precisely because it would otherwise drift.
- Interim results must never be persisted as final; only finals and explicit
  revisions reach the database.

## Alternatives considered

- **Name everything "transcription":** simpler, but loses the streaming-vs-batch
  and engine-vs-artifact distinctions this design depends on. Rejected.
- **Name the events `audis://asr/*`:** consistent with the engine, but the
  events carry transcript artifacts to a product surface. Rejected.
