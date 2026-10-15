# ADR-004: ASR provider abstraction (and ASR vs transcript naming)

**Status:** Accepted. Partly implemented — see "As built" below.

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

## As built

The single `AsrProvider` trait above was never written. What exists is two
pipelines, chosen per session by `session::Recogniser`, because the engines
Audis can reach divide cleanly in two and one trait covering both would have
been a trait with half its methods unimplemented on either side.

**Batch** — `audis-asr::AsrEngine`, implemented by `WhisperEngine` and
`CloudEngine`. The endpointer in `vad.rs` cuts sentences out of the stream and
each one is recognised on its own. Local Whisper additionally re-decodes the
sentence in progress every 400 ms for interim captions; a batch cloud engine
does not, because every preview would be a billed round-trip on audio it has
already been sent.

This is the "live captions by repeated batch upload" the decision above
prohibits, and it is what shipped: none of Groq, Deepgram or Gemini's
`generateContent` offers anything else to an API-key client. It is confined to
the engines that leave no alternative.

**Streaming** — `audis-asr::live`, for Google's Live models. A socket per audio
source is fed 16 kHz PCM as it is captured, and words come back as they are
spoken. Endpointing happens at Google's end; Audis closes a sentence on
`turnComplete` or a 1.2-second gap in the words. This is the path ADR-004 asked
for, and the first engine to offer it.

Which one a session runs is decided by the model the user picked, through
`ProviderId::streams_speech`. There is no separate setting: a Live model only
answers over a socket, and the rest only answer a request.

Gemini therefore offers exactly two speech models, fixed in the catalogue rather
than fetched: the Live one, and `gemini-3.5-flash-lite` for anyone who would
rather spend a request per sentence. Every other provider still fetches its list
from the provider. Gemini does not, because `models.list` describes models by the
methods they serve and a model serving only `bidiGenerateContent` has been seen
missing from it — a fetched list can silently lose the streaming model, and then
the streaming path cannot be selected at all.

Two things the streaming path does not do:

- **Speaker separation is off.** The diarizer identifies a voice from the audio
  of a finished utterance, and a socket hands over words without audio. A
  session that had it enabled says so once rather than mislabelling every line.
- **Confidence is absent.** The socket reports none per sentence, so segments
  carry `None` rather than an invented number.

### The chosen language

The language a user picks is held inside each engine rather than enforced by
filtering afterwards, so an ambiguous or borrowed word comes out as the nearest
word in that language instead of being dropped:

- **Gemini Live** names it to the transcriber with
  `inputAudioTranscription.languageCodes` (`id-ID`, `en-US`). Without it Google
  detects the language per sentence and code-switches. A model that refuses the
  hint is reconnected without it rather than failing the session.
- **Whisper** (local, Groq, OpenAI-compatible) forces the language token and is
  primed with a sentence written in that language, which it reads as the
  transcript so far and continues in the same vocabulary and spelling.
- **Gemini by request** is instructed to write every word in the language and to
  use the nearest word in it for anything that sounds foreign.
- **Deepgram** is sent the single language, never `multi`.

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
