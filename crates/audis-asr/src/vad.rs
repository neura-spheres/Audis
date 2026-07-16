//! Voice activity detection and utterance endpointing.

use std::collections::VecDeque;

/// How much of a long sentence an interim caption decodes.
const INTERIM_WINDOW_MS: u32 = 3_000;

/// Tuning for [`Endpointer`].
#[derive(Debug, Clone, Copy)]
pub struct EndpointConfig {
    /// Speech below this RMS is treated as silence.
    pub silence_rms: f32,
    /// Silence this long ends the utterance.
    pub silence_ms: u32,
    /// Ignore blips shorter than this, so a cough is not an utterance.
    pub min_speech_ms: u32,
    /// Decode anyway once an utterance runs this long.
    pub max_utterance_ms: u32,
    /// Audio kept before speech was detected.
    pub preroll_ms: u32,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            silence_rms: 0.008,
            silence_ms: 900,
            min_speech_ms: 200,
            max_utterance_ms: 15_000,
            preroll_ms: 300,
        }
    }
}

/// What the endpointer decided about the block just pushed.
#[derive(Debug, Clone, PartialEq)]
pub enum EndpointEvent {
    /// Nothing to do.
    Idle,
    /// Speech started.
    SpeechStarted,
    /// Speech continues.
    Speaking,
    /// An utterance is ready to decode.
    Utterance(Utterance),
}

/// A complete chunk of speech, ready for an engine.
#[derive(Debug, Clone, PartialEq)]
pub struct Utterance {
    /// 16 kHz mono samples, including pre-roll.
    pub samples: Vec<f32>,
    /// Offset from session start where this utterance begins.
    pub start_ms: i64,
    /// Offset from session start where it ends.
    pub end_ms: i64,
    /// True when this was cut by `max_utterance_ms` rather than by silence, so
    pub truncated: bool,
}

/// Splits a continuous 16 kHz mono stream into utterances.
#[derive(Debug)]
pub struct Endpointer {
    config: EndpointConfig,
    sample_rate: u32,
    /// Audio of the utterance being collected.
    current: Vec<f32>,
    /// Rolling pre-roll, kept while idle.
    preroll: VecDeque<f32>,
    preroll_capacity: usize,
    in_speech: bool,
    silence_samples: u32,
    speech_samples: u32,
    /// Total samples seen, which is the session clock.
    total_samples: u64,
    /// Where the current utterance started.
    utterance_start_sample: u64,
}

impl Endpointer {
    /// A new endpointer for a 16 kHz mono stream.
    pub fn new(config: EndpointConfig) -> Self {
        let sample_rate = crate::prepare::TARGET_SAMPLE_RATE;
        let preroll_capacity = ms_to_samples(config.preroll_ms, sample_rate) as usize;

        Self {
            config,
            sample_rate,
            current: Vec::new(),
            preroll: VecDeque::with_capacity(preroll_capacity + 1),
            preroll_capacity,
            in_speech: false,
            silence_samples: 0,
            speech_samples: 0,
            total_samples: 0,
            utterance_start_sample: 0,
        }
    }

    /// Feed one block of 16 kHz mono audio.
    pub fn push(&mut self, samples: &[f32]) -> EndpointEvent {
        if samples.is_empty() {
            return EndpointEvent::Idle;
        }

        let is_speech = rms(samples) >= self.config.silence_rms;
        let block_len = samples.len() as u32;

        let event = if is_speech {
            self.on_speech(samples, block_len)
        } else {
            self.on_silence(samples, block_len)
        };

        self.total_samples += u64::from(block_len);
        event
    }

    fn on_speech(&mut self, samples: &[f32], block_len: u32) -> EndpointEvent {
        let started = !self.in_speech;

        if started {
            self.in_speech = true;
            self.speech_samples = 0;
            let preroll_len = self.preroll.len() as u64;
            self.utterance_start_sample = self.total_samples.saturating_sub(preroll_len);
            self.current.extend(self.preroll.drain(..));
        }

        self.current.extend_from_slice(samples);
        self.speech_samples = self.speech_samples.saturating_add(block_len);
        self.silence_samples = 0;

        let max_samples = ms_to_samples(self.config.max_utterance_ms, self.sample_rate);
        if self.current.len() as u32 >= max_samples {
            return self.finish(true, block_len);
        }

        if started {
            EndpointEvent::SpeechStarted
        } else {
            EndpointEvent::Speaking
        }
    }

    fn on_silence(&mut self, samples: &[f32], block_len: u32) -> EndpointEvent {
        if !self.in_speech {
            self.remember_preroll(samples);
            return EndpointEvent::Idle;
        }

        self.current.extend_from_slice(samples);
        self.silence_samples = self.silence_samples.saturating_add(block_len);

        let needed = ms_to_samples(self.config.silence_ms, self.sample_rate);
        if self.silence_samples >= needed {
            return self.finish(false, block_len);
        }

        EndpointEvent::Speaking
    }

    /// Close the current utterance and emit it, unless it was too short.
    fn finish(&mut self, truncated: bool, block_len: u32) -> EndpointEvent {
        let min_samples = ms_to_samples(self.config.min_speech_ms, self.sample_rate);
        let long_enough = self.speech_samples >= min_samples;

        let samples = std::mem::take(&mut self.current);
        let start_sample = self.utterance_start_sample;
        let end_sample = self.total_samples + u64::from(block_len);

        self.in_speech = false;
        self.silence_samples = 0;
        self.speech_samples = 0;
        self.preroll.clear();

        if !long_enough {
            return EndpointEvent::Idle;
        }

        EndpointEvent::Utterance(Utterance {
            samples,
            start_ms: samples_to_ms(start_sample, self.sample_rate),
            end_ms: samples_to_ms(end_sample, self.sample_rate),
            truncated,
        })
    }

    fn remember_preroll(&mut self, samples: &[f32]) {
        if self.preroll_capacity == 0 {
            return;
        }
        for &sample in samples {
            if self.preroll.len() == self.preroll_capacity {
                self.preroll.pop_front();
            }
            self.preroll.push_back(sample);
        }
    }

    /// The utterance being spoken right now, without ending it.
    pub fn snapshot(&self) -> Option<Utterance> {
        if !self.in_speech {
            return None;
        }

        let min_samples = ms_to_samples(self.config.min_speech_ms, self.sample_rate);
        if self.speech_samples < min_samples {
            return None;
        }

        let window = ms_to_samples(INTERIM_WINDOW_MS, self.sample_rate) as usize;
        let start = self.current.len().saturating_sub(window);
        let samples = self.current[start..].to_vec();

        let dropped = self.current.len() - samples.len();
        let window_start_sample = self.utterance_start_sample + dropped as u64;

        Some(Utterance {
            samples,
            start_ms: samples_to_ms(window_start_sample, self.sample_rate),
            end_ms: samples_to_ms(self.total_samples, self.sample_rate),
            truncated: true,
        })
    }

    /// Emit whatever is buffered, for when a session stops mid-sentence.
    pub fn flush(&mut self) -> Option<Utterance> {
        if !self.in_speech || self.current.is_empty() {
            return None;
        }
        match self.finish(false, 0) {
            EndpointEvent::Utterance(utterance) => Some(utterance),
            _ => None,
        }
    }

    /// True while collecting speech.
    pub fn in_speech(&self) -> bool {
        self.in_speech
    }
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    (sum / samples.len() as f64).sqrt() as f32
}

fn ms_to_samples(ms: u32, sample_rate: u32) -> u32 {
    (u64::from(ms) * u64::from(sample_rate) / 1000) as u32
}

fn samples_to_ms(samples: u64, sample_rate: u32) -> i64 {
    (samples * 1000 / u64::from(sample_rate)) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixed thresholds for the tests below.
    fn test_config() -> EndpointConfig {
        EndpointConfig {
            silence_rms: 0.008,
            silence_ms: 700,
            min_speech_ms: 200,
            max_utterance_ms: 15_000,
            preroll_ms: 300,
        }
    }

    /// Interim captions are decoded from this, so it must hand out the sentence
    #[test]
    fn a_snapshot_shows_the_sentence_so_far_without_ending_it() {
        let mut endpointer = Endpointer::new(test_config());

        assert!(
            endpointer.snapshot().is_none(),
            "silence is not a sentence in progress"
        );

        endpointer.push(&speech(400));
        let first = endpointer.snapshot().expect("speech is in progress");
        assert!(!first.samples.is_empty());
        assert!(first.truncated, "an unfinished sentence must say so");

        endpointer.push(&speech(400));
        let second = endpointer.snapshot().expect("still in progress");
        assert!(
            second.samples.len() > first.samples.len(),
            "the sentence must keep growing after a snapshot"
        );

        let event = endpointer.push(&silence(800));
        match event {
            EndpointEvent::Utterance(utterance) => assert!(
                utterance.samples.len() >= second.samples.len(),
                "the finished utterance must contain everything the snapshots saw"
            ),
            other => panic!("expected the utterance to complete, got {other:?}"),
        }

        assert!(
            endpointer.snapshot().is_none(),
            "nothing is in progress once the sentence ended"
        );
    }

    /// A cough must not become an interim caption either.
    #[test]
    fn a_snapshot_ignores_a_blip_too_short_to_be_speech() {
        let mut endpointer = Endpointer::new(test_config());
        endpointer.push(&speech(50));

        assert!(endpointer.snapshot().is_none());
    }

    /// The shipped defaults, stated once so changing them is a decision.
    #[test]
    fn defaults_are_tuned_for_whole_sentences() {
        let config = EndpointConfig::default();

        assert!(
            config.silence_ms >= 900,
            "a shorter gate cuts sentences mid-thought and produces fragments"
        );
        assert!(
            config.min_speech_ms <= 250,
            "a higher floor discards short but complete answers"
        );
        assert!(
            config.preroll_ms > 0,
            "without pre-roll the first consonant of every utterance is clipped"
        );
    }

    const RATE: u32 = 16_000;

    fn block(ms: u32, amplitude: f32) -> Vec<f32> {
        vec![amplitude; ms_to_samples(ms, RATE) as usize]
    }

    fn speech(ms: u32) -> Vec<f32> {
        block(ms, 0.3)
    }

    fn silence(ms: u32) -> Vec<f32> {
        block(ms, 0.0)
    }

    #[test]
    fn silence_alone_never_produces_an_utterance() {
        let mut endpointer = Endpointer::new(test_config());
        for _ in 0..20 {
            assert_eq!(endpointer.push(&silence(100)), EndpointEvent::Idle);
        }
    }

    #[test]
    fn speech_then_silence_produces_one_utterance() {
        let mut endpointer = Endpointer::new(test_config());

        assert_eq!(endpointer.push(&speech(100)), EndpointEvent::SpeechStarted);
        assert_eq!(endpointer.push(&speech(400)), EndpointEvent::Speaking);

        assert_eq!(endpointer.push(&silence(400)), EndpointEvent::Speaking);

        let event = endpointer.push(&silence(400));
        match event {
            EndpointEvent::Utterance(utterance) => {
                assert!(!utterance.truncated);
                assert!(!utterance.samples.is_empty());
                assert!(utterance.end_ms > utterance.start_ms);
            }
            other => panic!("expected an utterance, got {other:?}"),
        }
    }

    /// A comma-length pause must not split a sentence.
    #[test]
    fn a_short_pause_does_not_end_the_utterance() {
        let mut endpointer = Endpointer::new(test_config());

        endpointer.push(&speech(300));
        assert_eq!(endpointer.push(&silence(300)), EndpointEvent::Speaking);
        assert_eq!(endpointer.push(&speech(300)), EndpointEvent::Speaking);
        assert!(endpointer.in_speech(), "the utterance should still be open");
    }

    /// A cough must not be sent to the model.
    #[test]
    fn a_blip_shorter_than_min_speech_is_discarded() {
        let mut endpointer = Endpointer::new(test_config());

        endpointer.push(&speech(50)); // below the 200 ms floor
        let event = endpointer.push(&silence(800));

        assert_eq!(event, EndpointEvent::Idle, "a 50 ms blip must be dropped");
    }

    /// Someone who never pauses must still get captions.
    #[test]
    fn a_monologue_is_cut_at_the_maximum() {
        let config = EndpointConfig {
            max_utterance_ms: 1000,
            ..EndpointConfig::default()
        };
        let mut endpointer = Endpointer::new(config);

        endpointer.push(&speech(600));
        let event = endpointer.push(&speech(600));

        match event {
            EndpointEvent::Utterance(utterance) => {
                assert!(utterance.truncated, "a forced cut must be marked truncated");
            }
            other => panic!("expected a truncated utterance, got {other:?}"),
        }
    }

    /// Without pre-roll the first consonant is clipped and Whisper mishears it.
    #[test]
    fn preroll_is_prepended_so_the_first_word_survives() {
        let mut endpointer = Endpointer::new(test_config());

        endpointer.push(&block(500, 0.001));
        endpointer.push(&speech(300));
        let event = endpointer.push(&silence(800));

        match event {
            EndpointEvent::Utterance(utterance) => {
                let speech_samples = ms_to_samples(300, RATE) as usize;
                assert!(
                    utterance.samples.len() > speech_samples,
                    "pre-roll was not included: {} <= {}",
                    utterance.samples.len(),
                    speech_samples
                );
            }
            other => panic!("expected an utterance, got {other:?}"),
        }
    }

    /// Stopping mid-sentence must not lose the sentence.
    #[test]
    fn flush_emits_a_sentence_still_in_progress() {
        let mut endpointer = Endpointer::new(test_config());
        endpointer.push(&speech(500));

        let flushed = endpointer
            .flush()
            .expect("in-progress speech must survive stop");
        assert!(!flushed.samples.is_empty());
    }

    #[test]
    fn flush_on_an_idle_stream_yields_nothing() {
        let mut endpointer = Endpointer::new(EndpointConfig::default());
        endpointer.push(&silence(500));
        assert!(endpointer.flush().is_none());
    }

    #[test]
    fn timestamps_advance_across_utterances() {
        let mut endpointer = Endpointer::new(test_config());

        endpointer.push(&speech(400));
        let first = match endpointer.push(&silence(800)) {
            EndpointEvent::Utterance(u) => u,
            other => panic!("expected utterance, got {other:?}"),
        };

        endpointer.push(&speech(400));
        let second = match endpointer.push(&silence(800)) {
            EndpointEvent::Utterance(u) => u,
            other => panic!("expected utterance, got {other:?}"),
        };

        assert!(
            second.start_ms >= first.end_ms,
            "utterances overlap: {first:?} then {second:?}"
        );
    }

    /// A real noise floor is not digital silence.
    #[test]
    fn a_quiet_room_tone_still_counts_as_silence() {
        let mut endpointer = Endpointer::new(EndpointConfig::default());
        for _ in 0..10 {
            assert_eq!(endpointer.push(&block(100, 0.001)), EndpointEvent::Idle);
        }
    }
}
