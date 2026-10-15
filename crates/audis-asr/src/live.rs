//! Live speech recognition over Gemini's bidirectional socket.
//!
//! [`crate::cloud::CloudEngine`] sends one finished utterance per request and
//! waits for the whole answer, so nothing appears until someone stops talking
//! and the round-trip completes. This holds a socket open for the length of a
//! session instead and pushes audio into it as it is captured. Gemini sends the
//! words back as it hears them, so a caption grows while the sentence is still
//! being spoken.
//!
//! Google's speech recognition here is a side effect of a conversational
//! endpoint: the Live API is built for a voice assistant, and the transcript of
//! what the user said is offered alongside the assistant's reply. Audis wants
//! only the transcript, so the model is asked for text, told to stay silent, and
//! everything it says back is discarded. What is kept is `inputTranscription`,
//! which is the ASR output proper.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, SyncSender};
use std::time::{Duration, Instant};

use audis_common::{Language, ProviderId};
use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::error::{AsrError, Result};

/// The bidirectional endpoint. The API key goes on the query string, which is
/// the only way this endpoint accepts one, so the built URL is a secret and is
/// never logged.
const ENDPOINT: &str = "wss://generativelanguage.googleapis.com/ws/\
                        google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent";

/// The only input rate the Live API takes, and what the resampler produces.
const SAMPLE_RATE: u32 = crate::prepare::TARGET_SAMPLE_RATE;

/// How much audio travels in one message.
///
/// 100 ms is short enough that it adds no latency anyone can hear and long
/// enough that the socket is not carrying more JSON framing than audio.
const CHUNK_SAMPLES: usize = SAMPLE_RATE as usize / 10;

/// How many chunks may wait for the socket before audio is dropped.
///
/// Five seconds. A stalled network must not turn into unbounded memory, and
/// audio that arrives five seconds late is no longer a live caption anyway.
const MAX_QUEUED_CHUNKS: usize = 50;

/// How long to wait for the socket to accept the session before giving up.
const SETUP_TIMEOUT: Duration = Duration::from_secs(20);

/// Silence after which the words collected so far are called a sentence.
///
/// Gemini closes a turn only once it has decided to answer, and Audis has asked
/// it not to answer, so `turnComplete` cannot be relied on to arrive promptly.
/// This is the endpointer for the text stream instead.
const SENTENCE_GAP: Duration = Duration::from_millis(1_200);

/// How often the sentence gap is checked.
const TICK: Duration = Duration::from_millis(200);

/// How long to wait before rebuilding a socket that dropped.
///
/// The Live API ends an audio session after about fifteen minutes, so a meeting
/// of any length will reconnect. It is a normal event, not a fault.
const RECONNECT_DELAY: Duration = Duration::from_millis(750);

/// What to connect to, and as whom.
#[derive(Clone)]
pub struct LiveConfig {
    /// The Live model, as the user chose it.
    pub model: String,
    /// The Gemini API key.
    pub api_key: String,
    /// The language to recognise. Always set, never detected.
    pub language: Language,
}

/// Written by hand so the key cannot reach a log.
impl std::fmt::Debug for LiveConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LiveConfig")
            .field("model", &self.model)
            .field("language", &self.language)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

/// What a live socket reports.
#[derive(Debug)]
pub enum LiveEvent {
    /// The sentence as it stands so far. Replaces the previous partial.
    Partial(String),
    /// A sentence that will not change again.
    Final(String),
    /// Recognition is failing. The socket keeps trying.
    Failed(AsrError),
}

/// The engine identifier this reports itself as, in logs and the UI.
pub const ENGINE_ID: &str = "gemini-live";

/// Open a live socket and return its two ends.
///
/// Blocks until Gemini has accepted the session, so a wrong key or an unknown
/// model is refused where the user asked for it rather than silently producing
/// no captions once a session is running.
pub fn connect(config: &LiveConfig) -> Result<(LiveInput, LiveOutput)> {
    if config.api_key.trim().is_empty() {
        return Err(AsrError::ProviderKeyMissing {
            provider: provider_name(),
        });
    }

    let (audio_tx, audio_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    let (event_tx, event_rx) = std::sync::mpsc::channel::<LiveEvent>();
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel::<Result<()>>(1);

    let stop = Arc::new(AtomicBool::new(false));
    let queued = Arc::new(AtomicUsize::new(0));

    let handle = std::thread::Builder::new()
        .name("audis-gemini-live".to_owned())
        .spawn({
            let config = config.clone();
            let stop = Arc::clone(&stop);
            let queued = Arc::clone(&queued);
            move || run(config, audio_rx, event_tx, ready_tx, stop, queued)
        })
        .map_err(|error| AsrError::Recognition {
            detail: format!("the live recognition thread could not start: {error}"),
        })?;

    let worker = Arc::new(Worker {
        stop,
        handle: std::sync::Mutex::new(Some(handle)),
    });

    // A failure here is the worker's first connection attempt failing, which is
    // the one the user is waiting on. Later ones are reported as events.
    match ready_rx.recv_timeout(SETUP_TIMEOUT) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error),
        Err(RecvTimeoutError::Timeout) => {
            return Err(AsrError::ProviderUnreachable {
                provider: provider_name(),
                detail: format!(
                    "the live socket did not answer within {} seconds",
                    SETUP_TIMEOUT.as_secs()
                ),
            });
        }
        Err(RecvTimeoutError::Disconnected) => {
            return Err(AsrError::ProviderUnreachable {
                provider: provider_name(),
                detail: "the live socket closed before it was ready".to_owned(),
            });
        }
    }

    tracing::info!(model = %config.model, "gemini live socket ready");

    Ok((
        LiveInput {
            audio: audio_tx,
            buffer: Vec::with_capacity(CHUNK_SAMPLES * 2),
            queued,
            dropped: 0,
            worker: Arc::clone(&worker),
        },
        LiveOutput {
            events: event_rx,
            worker,
        },
    ))
}

/// The socket's own thread, and the flag that ends it.
///
/// Shared by both ends: the thread runs until the last of them is dropped.
struct Worker {
    stop: Arc<AtomicBool>,
    handle: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let handle = self.handle.lock().ok().and_then(|mut guard| guard.take());
        if let Some(handle) = handle {
            let _ = handle.join();
        }
    }
}

/// The audio end of a live socket.
pub struct LiveInput {
    audio: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    /// Samples still short of a full chunk.
    buffer: Vec<f32>,
    /// Chunks handed over but not yet sent, so a stall is visible here.
    queued: Arc<AtomicUsize>,
    /// Chunks dropped because the socket was too far behind.
    dropped: u64,
    /// Held, not read: the socket's thread lives until both ends are dropped.
    #[allow(dead_code)]
    worker: Arc<Worker>,
}

impl LiveInput {
    /// Feed one block of 16 kHz mono audio.
    ///
    /// Never blocks: audio arrives on the capture path, which cannot wait for a
    /// network.
    pub fn push(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);
        while self.buffer.len() >= CHUNK_SAMPLES {
            let chunk: Vec<f32> = self.buffer.drain(..CHUNK_SAMPLES).collect();
            self.send(&chunk);
        }
    }

    /// Send whatever is buffered, for when a session stops mid-sentence.
    pub fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let chunk = std::mem::take(&mut self.buffer);
            self.send(&chunk);
        }
    }

    fn send(&mut self, samples: &[f32]) {
        if self.queued.load(Ordering::Relaxed) >= MAX_QUEUED_CHUNKS {
            self.dropped += 1;
            if self.dropped == 1 || self.dropped.is_multiple_of(100) {
                tracing::warn!(
                    dropped = self.dropped,
                    "the live socket is behind; dropping audio"
                );
            }
            return;
        }

        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for &sample in samples {
            bytes.extend_from_slice(&to_i16(sample).to_le_bytes());
        }

        self.queued.fetch_add(1, Ordering::Relaxed);
        if self.audio.send(bytes).is_err() {
            self.queued.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

impl std::fmt::Debug for LiveInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LiveInput")
            .field("buffered", &self.buffer.len())
            .field("queued", &self.queued.load(Ordering::Relaxed))
            .field("dropped", &self.dropped)
            .finish()
    }
}

/// The words end of a live socket.
pub struct LiveOutput {
    events: Receiver<LiveEvent>,
    /// Held, not read: the socket's thread lives until both ends are dropped.
    #[allow(dead_code)]
    worker: Arc<Worker>,
}

impl LiveOutput {
    /// Wait up to `timeout` for the next thing the socket has to say.
    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> std::result::Result<LiveEvent, RecvTimeoutError> {
        self.events.recv_timeout(timeout)
    }
}

impl std::fmt::Debug for LiveOutput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("LiveOutput").finish()
    }
}

/// Whether a socket ended for good or should be rebuilt.
enum Outcome {
    /// The session is over: no more audio is coming.
    Done,
    /// Gemini closed the socket. A session has a time limit, so this is the
    /// normal end of a long meeting's first socket, not a fault.
    Reconnect,
}

/// The worker thread: a runtime, and a socket that is kept alive.
fn run(
    config: LiveConfig,
    audio: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    events: Sender<LiveEvent>,
    ready: SyncSender<Result<()>>,
    stop: Arc<AtomicBool>,
    queued: Arc<AtomicUsize>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(AsrError::Recognition {
                detail: format!("the live recognition runtime could not start: {error}"),
            }));
            return;
        }
    };

    runtime.block_on(sessions(config, audio, events, ready, stop, queued));
    tracing::debug!("gemini live thread finished");
}

/// Keep a socket open for as long as the session lasts.
async fn sessions(
    config: LiveConfig,
    mut audio: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    events: Sender<LiveEvent>,
    ready: SyncSender<Result<()>>,
    stop: Arc<AtomicBool>,
    queued: Arc<AtomicUsize>,
) {
    let mut ready = Some(ready);
    // Whether to tell Google which language is spoken. On by default; turned
    // off only if a model refuses a setup that carries it, so a model without
    // the option still transcribes rather than failing the session.
    let mut hint_language = true;

    loop {
        if stop.load(Ordering::SeqCst) {
            return;
        }

        let socket = match open(&config, hint_language).await {
            Ok(socket) => socket,
            Err(error @ AsrError::ProviderRejected { .. }) if hint_language => {
                tracing::warn!(
                    %error,
                    "the live model refused the language hint; continuing without it"
                );
                hint_language = false;
                continue;
            }
            Err(error) => {
                // The first attempt is the one `connect` is waiting on, so its
                // failure is the caller's failure and nothing is retried.
                if let Some(ready) = ready.take() {
                    let _ = ready.send(Err(error));
                    return;
                }
                tracing::warn!(%error, "the live socket could not be rebuilt");
                let _ = events.send(LiveEvent::Failed(error));
                tokio::time::sleep(RECONNECT_DELAY).await;
                continue;
            }
        };

        if let Some(ready) = ready.take() {
            let _ = ready.send(Ok(()));
        }

        match pump(socket, &mut audio, &events, &stop, &queued).await {
            Outcome::Done => return,
            Outcome::Reconnect => {
                tracing::info!("the live socket closed; opening another");
                tokio::time::sleep(RECONNECT_DELAY).await;
            }
        }
    }
}

/// A connected socket that has accepted the session.
type Socket =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Open a socket and get past setup.
async fn open(config: &LiveConfig, hint_language: bool) -> Result<Socket> {
    let url = format!("{ENDPOINT}?key={}", config.api_key.trim());

    let (mut socket, _) = tokio_tungstenite::connect_async_tls_with_config(
        url.as_str(),
        None,
        false,
        Some(tokio_tungstenite::Connector::Rustls(tls_config()?)),
    )
    .await
    .map_err(|error| {
        let failure = handshake_error(error);
        tracing::error!(%failure, model = %config.model, "the live socket handshake failed");
        failure
    })?;

    socket
        .send(Message::text(setup_message(config, hint_language)))
        .await
        .map_err(|error| AsrError::ProviderUnreachable {
            provider: provider_name(),
            detail: format!("the session could not be opened: {error}"),
        })?;

    // Nothing may be sent before setup is acknowledged, and a rejected setup is
    // reported by closing the socket rather than by an error message.
    loop {
        let message = tokio::time::timeout(SETUP_TIMEOUT, socket.next())
            .await
            .map_err(|_| AsrError::ProviderUnreachable {
                provider: provider_name(),
                detail: "the live socket accepted no session".to_owned(),
            })?;

        match message {
            Some(Ok(Message::Text(text))) if is_setup_complete(&text) => return Ok(socket),
            Some(Ok(Message::Binary(bytes))) if is_setup_complete(&decode(&bytes)) => {
                return Ok(socket);
            }
            Some(Ok(Message::Close(frame))) => {
                // Gemini reports a setup it will not accept — an unknown model,
                // a modality the model cannot serve — by closing with a reason
                // rather than by answering. That reason is the only description
                // of what was wrong, so it is logged before it is wrapped.
                tracing::error!(
                    code = close_status(&frame),
                    reason = %frame.as_ref().map(|frame| frame.reason.to_string()).unwrap_or_default(),
                    model = %config.model,
                    "gemini closed the live socket during setup"
                );
                return Err(AsrError::ProviderRejected {
                    provider: provider_name(),
                    status: close_status(&frame),
                    detail: frame
                        .map(|frame| frame.reason.to_string())
                        .unwrap_or_else(|| "the live socket was closed during setup".to_owned()),
                });
            }
            // Anything before `setupComplete` is unexpected and worth seeing:
            // it is usually an error the server chose to describe in a message.
            Some(Ok(Message::Text(text))) => {
                tracing::warn!(body = %text.chars().take(300).collect::<String>(), "unexpected message during setup");
                continue;
            }
            Some(Ok(_)) => continue,
            Some(Err(error)) => {
                return Err(AsrError::ProviderUnreachable {
                    provider: provider_name(),
                    detail: error.to_string(),
                });
            }
            None => {
                return Err(AsrError::ProviderUnreachable {
                    provider: provider_name(),
                    detail: "the live socket closed before it was ready".to_owned(),
                });
            }
        }
    }
}

/// Move audio one way and words the other until one end stops.
async fn pump(
    socket: Socket,
    audio: &mut tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
    events: &Sender<LiveEvent>,
    stop: &AtomicBool,
    queued: &AtomicUsize,
) -> Outcome {
    let (mut sink, mut stream) = socket.split();
    let mut sentence = Sentence::default();
    let mut ticker = tokio::time::interval(TICK);

    loop {
        if stop.load(Ordering::SeqCst) {
            sentence.finish(events);
            let _ = sink.send(Message::Close(None)).await;
            return Outcome::Done;
        }

        tokio::select! {
            chunk = audio.recv() => {
                let Some(chunk) = chunk else {
                    // The audio end was dropped: the session is over.
                    sentence.finish(events);
                    let _ = sink.send(Message::Close(None)).await;
                    return Outcome::Done;
                };
                queued.fetch_sub(1, Ordering::Relaxed);
                if sink.send(Message::text(audio_message(&chunk))).await.is_err() {
                    sentence.finish(events);
                    return Outcome::Reconnect;
                }
            }

            message = stream.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => sentence.absorb(&text, events),
                    Some(Ok(Message::Binary(bytes))) => sentence.absorb(&decode(&bytes), events),
                    Some(Ok(Message::Close(_))) | None => {
                        sentence.finish(events);
                        return Outcome::Reconnect;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        tracing::warn!(%error, "the live socket faulted");
                        sentence.finish(events);
                        return Outcome::Reconnect;
                    }
                }
            }

            _ = ticker.tick() => sentence.finish_if_quiet(events),
        }
    }
}

/// The sentence being spoken, assembled from the fragments Gemini sends.
#[derive(Default)]
struct Sentence {
    words: String,
    /// When the last fragment arrived, which is what ends the sentence.
    last: Option<Instant>,
}

impl Sentence {
    /// Take in one server message.
    fn absorb(&mut self, body: &str, events: &Sender<LiveEvent>) {
        let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
            return;
        };

        if let Some(go_away) = json.get("goAway") {
            // The session is near its time limit. The socket closing right
            // after this is expected, and `pump` reconnects.
            tracing::info!(detail = %go_away, "the live session is about to end");
        }

        let Some(content) = json.get("serverContent") else {
            return;
        };

        if let Some(text) = content
            .get("inputTranscription")
            .and_then(|transcription| transcription.get("text"))
            .and_then(|text| text.as_str())
            && !text.is_empty()
        {
            self.words.push_str(text);
            self.last = Some(Instant::now());
            let words = self.words.trim();
            if !words.is_empty() {
                let _ = events.send(LiveEvent::Partial(words.to_owned()));
            }
        }

        // The model answering is waste, not output: it was asked to stay quiet,
        // and a native-audio model bills for what it says whether or not anyone
        // listens. Nothing here reaches a caption — this is only so a model that
        // has started chattering through a meeting is visible in a log rather
        // than showing up as an unexplained quota bill.
        if content.get("modelTurn").is_some() {
            tracing::trace!("dropped a spoken reply the model was asked not to give");
        }

        // Gemini decided the speaker was done. Whatever it heard is a sentence,
        // whether or not it went on to say anything itself.
        if content
            .get("turnComplete")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            self.finish(events);
        }
    }

    /// End the sentence if nothing has been heard for long enough.
    fn finish_if_quiet(&mut self, events: &Sender<LiveEvent>) {
        if self.last.is_some_and(|last| last.elapsed() >= SENTENCE_GAP) {
            self.finish(events);
        }
    }

    /// Publish what has been collected, if anything.
    fn finish(&mut self, events: &Sender<LiveEvent>) {
        self.last = None;
        let words = std::mem::take(&mut self.words);
        let words = words.trim();
        if !words.is_empty() {
            let _ = events.send(LiveEvent::Final(words.to_owned()));
        }
    }
}

/// The opening message, which declares what the session is for.
///
/// `hint_language` names the spoken language to the transcriber. Without it
/// Google detects the language itself, sentence by sentence, and switches
/// freely; with it, recognition leans towards the chosen language's words even
/// when a sound is ambiguous or borrowed.
fn setup_message(config: &LiveConfig, hint_language: bool) -> String {
    let language = config.language.english_name();

    // The Live API is a voice assistant that happens to publish what it heard.
    // Audis wants only what it heard, so the model is given nothing to answer
    // and told plainly to stay quiet. Anything it says anyway is dropped on
    // arrival, so this is an economy measure rather than a correctness one:
    // `inputTranscription` is produced from the audio either way.
    let instruction = format!(
        "You are a silent transcription service. The audio is {language} speech. \
         Never reply, never greet, never comment, and never answer anything you hear. \
         Produce no output at all."
    );

    // The transcript comes from a recogniser separate from the model, which
    // the instruction above does not reach. This is the one setting that does.
    let transcription = if hint_language {
        serde_json::json!({ "languageCodes": [config.language.bcp47()] })
    } else {
        serde_json::json!({})
    };

    serde_json::json!({
        "setup": {
            "model": format!("models/{}", config.model.trim()),
            "generationConfig": {
                // AUDIO, not TEXT, and not a preference. This is a native-audio
                // model: it speaks or it does nothing, and asking for text is
                // refused outright — "the requested combination of response
                // modalities (TEXT) is not supported by the model", closing the
                // socket during setup. The reply it speaks is never read and
                // never played; only `inputAudioTranscription` is wanted, and
                // that is produced regardless of what the model answers in.
                "responseModalities": ["AUDIO"],
                "temperature": 0.0,
            },
            "systemInstruction": { "parts": [{ "text": instruction }] },
            "inputAudioTranscription": transcription,
        }
    })
    .to_string()
}

/// One chunk of audio, as the Live API takes it.
fn audio_message(pcm: &[u8]) -> String {
    let data = base64::engine::general_purpose::STANDARD.encode(pcm);
    serde_json::json!({
        "realtimeInput": {
            "audio": {
                "mimeType": format!("audio/pcm;rate={SAMPLE_RATE}"),
                "data": data,
            }
        }
    })
    .to_string()
}

/// Whether a server message is the acknowledgement that setup landed.
fn is_setup_complete(body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .is_some_and(|json| json.get("setupComplete").is_some())
}

fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Turn a failed handshake into something [`AsrError`] can explain.
///
/// A rejected key arrives as an HTTP status on the upgrade, not as a message on
/// the socket, so it has to be pulled out here for the 401 and 429 advice in
/// [`AsrError::to_user_facing`] to reach the user.
fn handshake_error(error: tokio_tungstenite::tungstenite::Error) -> AsrError {
    use tokio_tungstenite::tungstenite::Error;

    match error {
        Error::Http(response) => {
            let status = response.status().as_u16();
            let detail = response
                .body()
                .as_ref()
                .map(|body| String::from_utf8_lossy(body).chars().take(300).collect())
                .unwrap_or_default();
            AsrError::ProviderRejected {
                provider: provider_name(),
                status,
                detail,
            }
        }
        other => AsrError::ProviderUnreachable {
            provider: provider_name(),
            detail: other.to_string(),
        },
    }
}

/// A close frame carries a WebSocket code, not an HTTP one. Report it as a
/// refusal without a status rather than inventing an HTTP status it never had.
fn close_status(frame: &Option<tokio_tungstenite::tungstenite::protocol::CloseFrame>) -> u16 {
    frame
        .as_ref()
        .map(|frame| u16::from(frame.code))
        .unwrap_or(0)
}

/// TLS for the socket, built with an explicit provider.
///
/// Both rustls backends are in this build's dependency tree, one through the
/// HTTP client and one through its QUIC support. rustls refuses to guess
/// between them, so leaving the choice to its default would panic on the first
/// connection. Naming one here settles it.
fn tls_config() -> Result<Arc<rustls::ClientConfig>> {
    use std::sync::OnceLock;

    static CONFIG: OnceLock<std::result::Result<Arc<rustls::ClientConfig>, String>> =
        OnceLock::new();

    CONFIG
        .get_or_init(build_tls_config)
        .clone()
        .map_err(|detail| AsrError::ProviderUnreachable {
            provider: provider_name(),
            detail,
        })
}

fn build_tls_config() -> std::result::Result<Arc<rustls::ClientConfig>, String> {
    let native = rustls_native_certs::load_native_certs();
    let mut roots = rustls::RootCertStore::empty();
    let (added, _) = roots.add_parsable_certificates(native.certs);

    if added == 0 {
        return Err(format!(
            "no trusted certificates could be read from Windows: {:?}",
            native.errors
        ));
    }

    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|error| error.to_string())?
    .with_root_certificates(roots)
    .with_no_client_auth();

    Ok(Arc::new(config))
}

fn provider_name() -> String {
    ProviderId::Gemini.info().name
}

fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn config() -> LiveConfig {
        LiveConfig {
            model: "gemini-3.8-live".to_owned(),
            api_key: "sk-super-secret-value".to_owned(),
            language: Language::Indonesian,
        }
    }

    #[test]
    fn the_setup_message_asks_for_the_transcript_and_nothing_else() {
        let setup: serde_json::Value =
            serde_json::from_str(&setup_message(&config(), true)).expect("valid json");
        let setup = &setup["setup"];

        assert_eq!(setup["model"], "models/gemini-3.8-live");
        // Without this the socket carries audio and never a word of text.
        assert!(
            setup.get("inputAudioTranscription").is_some(),
            "the transcript has to be asked for or it is never sent"
        );
        // A native-audio model refuses TEXT outright and closes the socket
        // during setup, so this is the only modality that opens a session at
        // all. Asking for text here cost a release once already.
        assert_eq!(setup["generationConfig"]["responseModalities"][0], "AUDIO");
        assert!(
            setup["systemInstruction"]["parts"][0]["text"]
                .as_str()
                .expect("an instruction")
                .contains("Indonesian")
        );
    }

    /// Without a language the recogniser detects one per sentence and switches
    /// mid-conversation; naming it keeps recognition in the chosen language.
    #[test]
    fn the_transcriber_is_told_which_language_is_spoken() {
        let setup: serde_json::Value =
            serde_json::from_str(&setup_message(&config(), true)).expect("valid json");
        assert_eq!(
            setup["setup"]["inputAudioTranscription"]["languageCodes"],
            serde_json::json!(["id-ID"])
        );

        let english = LiveConfig {
            language: Language::English,
            ..config()
        };
        let setup: serde_json::Value =
            serde_json::from_str(&setup_message(&english, true)).expect("valid json");
        assert_eq!(
            setup["setup"]["inputAudioTranscription"]["languageCodes"],
            serde_json::json!(["en-US"])
        );
    }

    /// The fallback for a model that refuses the hint must still ask for a
    /// transcript, or the session would connect and caption nothing.
    #[test]
    fn without_the_hint_the_transcript_is_still_requested() {
        let setup: serde_json::Value =
            serde_json::from_str(&setup_message(&config(), false)).expect("valid json");
        let transcription = &setup["setup"]["inputAudioTranscription"];
        assert!(transcription.is_object());
        assert!(transcription.get("languageCodes").is_none());
    }

    #[test]
    fn audio_travels_as_little_endian_pcm_at_the_rate_gemini_expects() {
        let message: serde_json::Value =
            serde_json::from_str(&audio_message(&[0x01, 0x02])).expect("valid json");
        let audio = &message["realtimeInput"]["audio"];

        assert_eq!(audio["mimeType"], "audio/pcm;rate=16000");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(audio["data"].as_str().expect("data"))
            .expect("base64");
        assert_eq!(decoded, vec![0x01, 0x02]);
    }

    /// The URL carries the key, so anything that prints the config prints it too
    /// unless Debug is written by hand.
    #[test]
    fn debug_never_prints_the_api_key() {
        let printed = format!("{:?}", config());

        assert!(!printed.contains("sk-super-secret-value"), "{printed}");
        assert!(printed.contains("<redacted>"));
    }

    #[test]
    fn an_empty_key_is_refused_before_a_socket_is_opened() {
        let error = connect(&LiveConfig {
            api_key: "   ".to_owned(),
            ..config()
        })
        .expect_err("an empty key must be refused");

        assert!(matches!(error, AsrError::ProviderKeyMissing { .. }));
    }

    /// Fragments arrive a few words at a time and have to read as one sentence.
    #[test]
    fn fragments_are_joined_and_published_as_they_arrive() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut sentence = Sentence::default();

        sentence.absorb(&transcription("selamat "), &tx);
        sentence.absorb(&transcription("pagi semua"), &tx);

        match rx.try_recv().expect("a first partial") {
            LiveEvent::Partial(text) => assert_eq!(text, "selamat"),
            other => panic!("expected a partial, got {other:?}"),
        }
        match rx.try_recv().expect("a second partial") {
            LiveEvent::Partial(text) => assert_eq!(text, "selamat pagi semua"),
            other => panic!("expected a partial, got {other:?}"),
        }
    }

    #[test]
    fn a_completed_turn_publishes_the_sentence_once_and_clears_it() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut sentence = Sentence::default();

        sentence.absorb(&transcription("halo dunia"), &tx);
        sentence.absorb(r#"{"serverContent":{"turnComplete":true}}"#, &tx);

        assert!(matches!(rx.try_recv(), Ok(LiveEvent::Partial(_))));
        match rx.try_recv().expect("a final") {
            LiveEvent::Final(text) => assert_eq!(text, "halo dunia"),
            other => panic!("expected a final, got {other:?}"),
        }

        // A second turn boundary has nothing left to publish.
        sentence.finish(&tx);
        assert!(rx.try_recv().is_err(), "a sentence was published twice");
    }

    /// Audis asks the model to stay silent, so it may never close the turn. The
    /// gap has to end the sentence or a caption would never become final.
    #[test]
    fn a_gap_in_the_words_ends_the_sentence_without_a_completed_turn() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut sentence = Sentence::default();

        sentence.absorb(&transcription("apa kabar"), &tx);
        assert!(matches!(rx.try_recv(), Ok(LiveEvent::Partial(_))));

        sentence.finish_if_quiet(&tx);
        assert!(rx.try_recv().is_err(), "the gap has not passed yet");

        sentence.last = Some(Instant::now() - SENTENCE_GAP);
        sentence.finish_if_quiet(&tx);
        match rx.try_recv().expect("a final") {
            LiveEvent::Final(text) => assert_eq!(text, "apa kabar"),
            other => panic!("expected a final, got {other:?}"),
        }
    }

    /// The model's own speech is not the transcript, and captioning it would put
    /// an assistant's words in someone else's mouth.
    #[test]
    fn what_the_model_says_back_is_never_captioned() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut sentence = Sentence::default();

        sentence.absorb(
            r#"{"serverContent":{"outputTranscription":{"text":"Hi, how can I help?"}}}"#,
            &tx,
        );
        sentence.absorb(
            r#"{"serverContent":{"modelTurn":{"parts":[{"text":"Hi, how can I help?"}]}}}"#,
            &tx,
        );

        assert!(
            rx.try_recv().is_err(),
            "the model's reply reached a caption"
        );
    }

    #[test]
    fn a_message_that_is_not_json_is_ignored_rather_than_captioned() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut sentence = Sentence::default();

        sentence.absorb("not json at all", &tx);
        sentence.absorb(r#"{"setupComplete":{}}"#, &tx);

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn setup_completion_is_recognised_and_nothing_else_is() {
        assert!(is_setup_complete(r#"{"setupComplete":{}}"#));
        assert!(!is_setup_complete(r#"{"serverContent":{}}"#));
        assert!(!is_setup_complete("not json"));
    }

    /// A rejected key arrives as an HTTP status on the upgrade. Losing it would
    /// leave the user with "could not connect" for a fixable problem.
    #[test]
    fn a_refused_upgrade_keeps_the_status_that_explains_it() {
        let response = tokio_tungstenite::tungstenite::http::Response::builder()
            .status(401)
            .body(Some(b"API key not valid".to_vec()))
            .expect("a response");

        let error = handshake_error(tokio_tungstenite::tungstenite::Error::Http(Box::new(
            response,
        )));

        match error {
            AsrError::ProviderRejected { status, .. } => assert_eq!(status, 401),
            other => panic!("expected a refusal, got {other:?}"),
        }
        assert!(
            error
                .to_user_facing()
                .suggested_action
                .contains("Providers")
        );
    }

    #[test]
    fn full_scale_audio_clips_rather_than_wrapping() {
        assert_eq!(to_i16(2.0), i16::MAX);
        assert_eq!(to_i16(-2.0), -i16::MAX);
        assert_eq!(to_i16(0.0), 0);
    }

    /// One chunk is what the socket sends at a time, so partial blocks have to
    /// wait rather than going out as undersized messages.
    #[test]
    fn audio_is_buffered_into_whole_chunks() {
        let (audio, mut received) = tokio::sync::mpsc::unbounded_channel();
        let mut input = LiveInput {
            audio,
            buffer: Vec::new(),
            queued: Arc::new(AtomicUsize::new(0)),
            dropped: 0,
            worker: Arc::new(Worker {
                stop: Arc::new(AtomicBool::new(false)),
                handle: std::sync::Mutex::new(None),
            }),
        };

        input.push(&vec![0.0; CHUNK_SAMPLES / 2]);
        assert!(received.try_recv().is_err(), "half a chunk went out early");

        input.push(&vec![0.0; CHUNK_SAMPLES / 2]);
        let chunk = received.try_recv().expect("a whole chunk");
        assert_eq!(chunk.len(), CHUNK_SAMPLES * 2, "16-bit samples");

        // Stopping mid-sentence must not strand the tail.
        input.push(&[0.0; 10]);
        input.flush();
        assert_eq!(received.try_recv().expect("the tail").len(), 20);
    }

    fn transcription(text: &str) -> String {
        serde_json::json!({
            "serverContent": { "inputTranscription": { "text": text } }
        })
        .to_string()
    }
}
