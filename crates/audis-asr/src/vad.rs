//! Voice activity detection and utterance endpointing.
//!
//! Whisper is not a streaming engine: it decodes a buffer and returns text. The
//! way you get live captions out of it is to decide *when* an utterance ended
//! and decode that. That decision is this module's whole job, and it is what
//! sets the latency the user actually feels.
//!
//! The trade-off is one number, [`EndpointConfig::silence_ms`]. Too short and a
//! natural pause mid-sentence splits the sentence in two. Too long and captions
//! lag behind the speaker. 700 ms sits above a comma pause and below a turn
//! boundary for both Indonesian and English.

use std::collections::VecDeque;

/// Tuning for [`Endpointer`].
#[derive(Debug, Clone, Copy)]
pub struct EndpointConfig {
    /// Speech below this RMS is treated as silence.
    ///
    /// Not zero: every microphone has a noise floor, so an exact-zero test
    /// would never fire.
    pub silence_rms: f32,
    /// Silence this long ends the utterance.
    pub silence_ms: u32,
    /// Ignore blips shorter than this, so a cough is not an utterance.
    pub min_speech_ms: u32,
    /// Decode anyway once an utterance runs this long.
    ///
    /// Someone who talks without pausing would otherwise never see a caption.
    pub max_utterance_ms: u32,
    /// Audio kept before speech was detected.
    ///
    /// VAD always notices slightly late, so without this the first consonant of
    /// every utterance is clipped and Whisper mishears the first word.
    pub preroll_ms: u32,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        Self {
            silence_rms: 0.008,
            silence_ms: 700,
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
    /// the text may end mid-sentence.
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
            // Rewind the start by the pre-roll we are about to prepend, so the
            // reported timestamp matches the audio actually decoded.
            let preroll_len = self.preroll.len() as u64;
            self.utterance_start_sample = self.total_samples.saturating_sub(preroll_len);
            self.current.extend(self.preroll.drain(..));
        }

        self.current.extend_from_slice(samples);
        self.speech_samples = self.speech_samples.saturating_add(block_len);
        self.silence_samples = 0;

        // Cut a long monologue so captions keep appearing.
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

        // Keep the trailing silence: Whisper uses it to know the utterance
        // ended, and trimming it makes the last word less reliable.
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
            // A cough or a door. Drop it rather than sending it to a model.
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

    /// Emit whatever is buffered, for when a session stops mid-sentence.
    ///
    /// A user who stops talking and clicks Stop must not lose their last
    /// sentence just because the silence timer had not elapsed.
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
        let mut endpointer = Endpointer::new(EndpointConfig::default());
        for _ in 0..20 {
            assert_eq!(endpointer.push(&silence(100)), EndpointEvent::Idle);
        }
    }

    #[test]
    fn speech_then_silence_produces_one_utterance() {
        let mut endpointer = Endpointer::new(EndpointConfig::default());

        assert_eq!(endpointer.push(&speech(100)), EndpointEvent::SpeechStarted);
        assert_eq!(endpointer.push(&speech(400)), EndpointEvent::Speaking);

        // Not yet: below the 700 ms threshold.
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
        let mut endpointer = Endpointer::new(EndpointConfig::default());

        endpointer.push(&speech(300));
        // 300 ms is a breath, not a turn boundary.
        assert_eq!(endpointer.push(&silence(300)), EndpointEvent::Speaking);
        assert_eq!(endpointer.push(&speech(300)), EndpointEvent::Speaking);
        assert!(endpointer.in_speech(), "the utterance should still be open");
    }

    /// A cough must not be sent to the model.
    #[test]
    fn a_blip_shorter_than_min_speech_is_discarded() {
        let mut endpointer = Endpointer::new(EndpointConfig::default());

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
        let mut endpointer = Endpointer::new(EndpointConfig::default());

        // Fill the pre-roll with quiet, non-zero room tone below the threshold.
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
        let mut endpointer = Endpointer::new(EndpointConfig::default());
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
        let mut endpointer = Endpointer::new(EndpointConfig::default());

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
        // 0.001 RMS is well under the 0.008 threshold.
        for _ in 0..10 {
            assert_eq!(endpointer.push(&block(100, 0.001)), EndpointEvent::Idle);
        }
    }
}
