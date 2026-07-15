//! End-to-end recognition against the real Whisper engine.
//!
//! Everything else in this crate tests a stage in isolation with synthetic
//! tones. That proves each stage does what it claims, but not that speech
//! survives the whole chain: a resampler can pass a sine-wave test and still
//! smear consonants, and only a real decode catches that.
//!
//! Ignored by default. It needs a ~148 MB model and takes seconds, so it is not
//! part of the normal gate. Run it deliberately:
//!
//! ```powershell
//! $env:AUDIS_TEST_MODEL = "$env:LOCALAPPDATA\NeuraAudis\Audis\models\ggml-base.bin"
//! $env:AUDIS_TEST_WAV = "path\to\speech.wav"
//! cargo test -p audis-asr --test pipeline -- --ignored --nocapture
//! ```

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use audis_asr::{AsrEngine, EndpointConfig, EndpointEvent, Endpointer, Resampler, WhisperEngine};
use audis_common::Language;

/// A 16-bit PCM WAV, as read from disk.
struct Wav {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
}

/// Parse a 16-bit PCM WAV.
///
/// Deliberately minimal: chunks are walked rather than assumed at fixed
/// offsets, because a WAV written by Windows TTS carries an extended `fmt `
/// chunk and a `fact` chunk that a naive 44-byte-header reader would swallow as
/// audio.
fn read_wav(path: &std::path::Path) -> Wav {
    let bytes = std::fs::read(path).expect("read the wav");
    assert_eq!(&bytes[0..4], b"RIFF", "not a RIFF file");
    assert_eq!(&bytes[8..12], b"WAVE", "not a WAVE file");

    let mut cursor = 12;
    let mut sample_rate = 0;
    let mut channels = 0;
    let mut bits = 0;
    let mut samples = Vec::new();

    while cursor + 8 <= bytes.len() {
        let id = &bytes[cursor..cursor + 4];
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let body = cursor + 8;

        match id {
            b"fmt " => {
                channels = u16::from_le_bytes(bytes[body + 2..body + 4].try_into().unwrap());
                sample_rate = u32::from_le_bytes(bytes[body + 4..body + 8].try_into().unwrap());
                bits = u16::from_le_bytes(bytes[body + 14..body + 16].try_into().unwrap());
            }
            b"data" => {
                assert_eq!(bits, 16, "this reader only handles 16-bit PCM");
                samples = bytes[body..body + size]
                    .chunks_exact(2)
                    .map(|pair| {
                        f32::from(i16::from_le_bytes([pair[0], pair[1]])) / f32::from(i16::MAX)
                    })
                    .collect();
            }
            _ => {}
        }

        // Chunks are word-aligned: an odd size is followed by a pad byte.
        cursor = body + size + (size % 2);
    }

    assert!(!samples.is_empty(), "the wav had no audio");
    Wav {
        samples,
        sample_rate,
        channels,
    }
}

fn env_path(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os(name).map(std::path::PathBuf::from)
}

/// Speech in, words out, through every stage the live session uses.
#[test]
#[ignore = "needs a downloaded model and a speech wav; see the module docs"]
fn real_speech_survives_the_whole_pipeline() {
    let (Some(model), Some(wav_path)) = (env_path("AUDIS_TEST_MODEL"), env_path("AUDIS_TEST_WAV"))
    else {
        panic!("set AUDIS_TEST_MODEL and AUDIS_TEST_WAV");
    };

    let wav = read_wav(&wav_path);
    println!("wav: {} Hz, {} channels", wav.sample_rate, wav.channels);

    let mut engine = WhisperEngine::load(&model).expect("load the model");
    let mut resampler = Resampler::new(wav.sample_rate);
    let mut endpointer = Endpointer::new(EndpointConfig::default());
    let mut text = String::new();

    // Fed in small blocks, exactly as a capture callback delivers it. A
    // resampler that only works on one big buffer would pass a naive test and
    // click at every seam here.
    for block in wav.samples.chunks(1024) {
        let mono = audis_asr::downmix_to_mono(block, wav.channels);
        let ready = resampler.push(&mono);
        if ready.is_empty() {
            continue;
        }

        if let EndpointEvent::Utterance(utterance) = endpointer.push(&ready) {
            let audio_ms = utterance.end_ms - utterance.start_ms;
            let started = std::time::Instant::now();
            let result = engine
                .transcribe(&utterance, Language::English)
                .expect("transcribe");
            let decode_ms = started.elapsed().as_millis() as i64;
            // The number that decides whether a model is usable live: below 1.0
            // it keeps up, above 1.0 captions fall further behind every sentence.
            println!(
                "utterance {audio_ms}ms audio -> decoded in {decode_ms}ms (x{:.2} real time) -> {:?}",
                decode_ms as f64 / audio_ms.max(1) as f64,
                result.text
            );
            text.push_str(&result.text);
            text.push(' ');
        }
    }

    // Whatever was still buffered when the audio ended is real speech too.
    if let Some(utterance) = endpointer.flush() {
        let result = engine
            .transcribe(&utterance, Language::English)
            .expect("transcribe the tail");
        println!("tail -> {:?}", result.text);
        text.push_str(&result.text);
    }

    let heard = text.to_lowercase();
    println!("\nfull transcript: {heard}");

    // Exact wording depends on the model, so this asserts on content words that
    // any working chain must produce rather than on a golden string.
    for word in ["quick", "brown", "fox", "lazy", "dog"] {
        assert!(
            heard.contains(word),
            "the pipeline lost {word:?}; heard {heard:?}"
        );
    }
}

/// The resampler must not invent or lose time.
///
/// Runs without a model, so it stays in the normal gate.
#[test]
fn resampling_preserves_duration_within_a_few_milliseconds() {
    let source_rate = 22_050;
    let seconds = 3;
    let input = vec![0.0f32; source_rate * seconds];

    let mut resampler = Resampler::new(source_rate as u32);
    let mut produced = 0;
    for block in input.chunks(1024) {
        produced += resampler.push(block).len();
    }

    let expected = 16_000 * seconds;
    let drift = (produced as i64 - expected as i64).abs();

    // A few milliseconds of filter latency is expected; drift that grows with
    // length is not, and would desynchronise every timestamp in a long meeting.
    assert!(
        drift < 16_000 / 20,
        "expected about {expected} samples, produced {produced} (drift {drift})"
    );
}
