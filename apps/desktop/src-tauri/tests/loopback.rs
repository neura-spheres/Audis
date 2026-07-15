//! The full local path: speakers -> WASAPI loopback -> Whisper.
//!
//! The unit tests cover each stage, and `audis-asr`'s pipeline test covers
//! everything downstream of a buffer of samples. This covers the one link
//! neither can: that capture actually hands real frames to a sink. That link
//! was silently missing once already — capture fed only the level meter, so the
//! meter moved convincingly while nothing reached the engine.
//!
//! Ignored by default: it needs a model, a real output device, and it plays
//! audio out loud. Run it deliberately:
//!
//! ```powershell
//! $env:AUDIS_TEST_MODEL = "$env:LOCALAPPDATA\NeuraAudis\Audis\models\ggml-base.bin"
//! cargo test -p audis-desktop --test loopback -- --ignored --nocapture
//! ```
//!
//! Loopback captures *everything* the machine plays, not just this test's
//! speech. A notification chime or a video in another window lands in the same
//! stream and Whisper will do its best to turn it into words, so an occasional
//! failure here can mean the room was noisy rather than the code is broken.
//! The per-utterance line printed below shows what was actually heard.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};

use audis_asr::{
    AsrEngine, EndpointConfig, EndpointEvent, Endpointer, Resampler, WhisperEngine, downmix_to_mono,
};
use audis_common::Language;

/// Speak a sentence through the default output device, using Windows TTS.
///
/// Blocks until the audio has finished playing.
fn speak(text: &str) -> bool {
    let script = format!(
        "Add-Type -AssemblyName System.Speech; \
         $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
         $s.Speak('{text}'); $s.Dispose()"
    );

    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
#[ignore = "plays audio out loud and needs a model; see the module docs"]
fn speech_played_on_this_pc_is_captured_and_transcribed() {
    let Some(model) = std::env::var_os("AUDIS_TEST_MODEL") else {
        panic!("set AUDIS_TEST_MODEL");
    };

    let mut engine = WhisperEngine::load(std::path::Path::new(&model)).expect("load the model");

    // Unbounded, unlike the live pipeline. There, a prepare thread drains
    // continuously and a bounded queue is what protects the real-time audio
    // callback from a slow consumer. Here nothing drains until playback ends,
    // so a bounded queue would fill and drop the tail of the sentence — which
    // is exactly what the first version of this test did, losing everything
    // after "the quick brown". A few seconds of audio in memory is fine.
    let (frames_tx, frames_rx) = channel::<Vec<f32>>();
    let blocks = Arc::new(AtomicUsize::new(0));
    let loudest = Arc::new(Mutex::new(0.0f32));

    let sink_blocks = Arc::clone(&blocks);
    let sink_loudest = Arc::clone(&loudest);
    let sink: audis_audio::FrameSink = Arc::new(move |data: &[f32]| {
        sink_blocks.fetch_add(1, Ordering::Relaxed);

        let peak = data
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        // try_lock, never lock: this runs on the real-time audio thread, where
        // waiting on a mutex is what causes a system-wide audio glitch. Missing
        // one peak reading costs nothing.
        if let Ok(mut current) = sink_loudest.try_lock()
            && peak > *current
        {
            *current = peak;
        }

        let _ = frames_tx.send(data.to_vec());
    });

    // Loopback listens to an *output* endpoint. Passing None takes whatever
    // Windows currently calls the default, which is what a user would get.
    let capture = match audis_audio::start_computer_audio_with_sink(None, sink) {
        Ok(capture) => capture,
        Err(error) => {
            eprintln!("skipping: no loopback device available ({error})");
            return;
        }
    };

    println!(
        "capturing {} at {} Hz, {} channels",
        capture.device_name(),
        capture.sample_rate(),
        capture.channels()
    );

    let sample_rate = capture.sample_rate();
    let channels = capture.channels();

    assert!(
        speak("The quick brown fox jumps over the lazy dog."),
        "Windows text to speech did not run"
    );

    // Loopback delivers a little after playback; let the tail arrive.
    std::thread::sleep(std::time::Duration::from_millis(500));
    drop(capture);

    let captured: Vec<Vec<f32>> = frames_rx.try_iter().collect();
    let peak = *loudest.lock().expect("read the peak");

    println!(
        "{} callbacks, {} blocks queued, peak {peak:.3}",
        blocks.load(Ordering::Relaxed),
        captured.len()
    );

    assert!(
        blocks.load(Ordering::Relaxed) > 0,
        "the sink was never called: capture is not delivering frames"
    );
    assert!(
        peak > 0.01,
        "captured only silence (peak {peak:.4}). Is the default output device \
         the one actually playing?"
    );

    let mut resampler = Resampler::new(sample_rate);
    let mut endpointer = Endpointer::new(EndpointConfig::default());
    let mut heard = String::new();

    for block in captured {
        let mono = downmix_to_mono(&block, channels);
        let ready = resampler.push(&mono);
        if ready.is_empty() {
            continue;
        }
        if let EndpointEvent::Utterance(utterance) = endpointer.push(&ready) {
            let rms = (utterance.samples.iter().map(|s| s * s).sum::<f32>()
                / utterance.samples.len() as f32)
                .sqrt();
            let peak_u = utterance.samples.iter().fold(0.0f32, |p, s| p.max(s.abs()));
            let result = engine
                .transcribe(&utterance, Language::English)
                .expect("transcribe");
            println!(
                "UTT {}..{}ms rms={rms:.5} peak={peak_u:.4} -> {:?}",
                utterance.start_ms, utterance.end_ms, result.text
            );
            heard.push_str(&result.text);
            heard.push(' ');
        }
    }

    if let Some(utterance) = endpointer.flush() {
        let result = engine
            .transcribe(&utterance, Language::English)
            .expect("transcribe the tail");
        heard.push_str(&result.text);
    }

    let heard = heard.to_lowercase();
    println!("heard: {heard:?}");

    for word in ["quick", "brown", "fox"] {
        assert!(
            heard.contains(word),
            "loopback capture lost {word:?}; heard {heard:?}"
        );
    }
}
