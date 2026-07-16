//! Level metering.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// A snapshot of one source's signal level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevel {
    /// Loudest sample in the last block, 0.0 to 1.0.
    pub peak: f32,
    /// Root mean square of the last block, 0.0 to 1.0. Tracks perceived
    pub rms: f32,
    /// True when a sample hit full scale and was probably clipped.
    pub clipping: bool,
    /// How long the signal has been effectively silent.
    pub silence_duration_ms: u64,
}

/// Anything below this is treated as silence rather than signal.
const SILENCE_THRESHOLD: f32 = 0.001;

/// Full scale. Samples at or above this were probably clipped by the ADC.
const CLIP_THRESHOLD: f32 = 0.999;

/// Lock-free level state shared between a capture callback and readers.
#[derive(Debug, Default)]
pub struct LevelMeter {
    /// f32 bits, because there is no AtomicF32 in std.
    peak: AtomicU32,
    rms: AtomicU32,
    clipping: AtomicBool,
    silent_samples: AtomicU64,
    sample_rate: AtomicU32,
}

impl LevelMeter {
    /// A meter for a stream running at `sample_rate`.
    pub fn new(sample_rate: u32) -> Arc<Self> {
        let meter = Self::default();
        meter.sample_rate.store(sample_rate, Ordering::Relaxed);
        Arc::new(meter)
    }

    /// Record one block of samples.
    pub fn observe(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }

        let mut peak = 0.0f32;
        let mut sum_squares = 0.0f64;

        for &sample in samples {
            let magnitude = sample.abs();
            if magnitude > peak {
                peak = magnitude;
            }
            sum_squares += f64::from(sample) * f64::from(sample);
        }

        let rms = (sum_squares / samples.len() as f64).sqrt() as f32;

        self.peak.store(peak.to_bits(), Ordering::Relaxed);
        self.rms.store(rms.to_bits(), Ordering::Relaxed);

        if peak >= CLIP_THRESHOLD {
            self.clipping.store(true, Ordering::Relaxed);
        }

        if peak < SILENCE_THRESHOLD {
            self.silent_samples
                .fetch_add(samples.len() as u64, Ordering::Relaxed);
        } else {
            self.silent_samples.store(0, Ordering::Relaxed);
        }
    }

    /// Read the current level.
    pub fn read(&self) -> AudioLevel {
        let sample_rate = self.sample_rate.load(Ordering::Relaxed).max(1);
        let silent_samples = self.silent_samples.load(Ordering::Relaxed);

        AudioLevel {
            peak: f32::from_bits(self.peak.load(Ordering::Relaxed)),
            rms: f32::from_bits(self.rms.load(Ordering::Relaxed)),
            clipping: self.clipping.swap(false, Ordering::Relaxed),
            silence_duration_ms: silent_samples * 1000 / u64::from(sample_rate),
        }
    }

    /// Update the sample rate, when a device reports a different one than
    pub fn set_sample_rate(&self, sample_rate: u32) {
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_reads_as_zero() {
        let meter = LevelMeter::new(48_000);
        meter.observe(&[0.0; 480]);

        let level = meter.read();
        assert_eq!(level.peak, 0.0);
        assert_eq!(level.rms, 0.0);
        assert!(!level.clipping);
    }

    #[test]
    fn peak_and_rms_are_measured() {
        let meter = LevelMeter::new(48_000);
        meter.observe(&[0.5, -0.5, 0.5, -0.5]);

        let level = meter.read();
        assert!((level.peak - 0.5).abs() < 1e-6);
        assert!((level.rms - 0.5).abs() < 1e-6);
    }

    #[test]
    fn rms_is_below_peak_for_a_realistic_signal() {
        let meter = LevelMeter::new(48_000);
        let mut block = [0.01f32; 100];
        block[0] = 0.9;
        meter.observe(&block);

        let level = meter.read();
        assert!((level.peak - 0.9).abs() < 1e-6);
        assert!(level.rms < 0.2, "rms was {}", level.rms);
    }

    #[test]
    fn clipping_is_latched_then_cleared_once_read() {
        let meter = LevelMeter::new(48_000);
        meter.observe(&[1.0, 0.2]);

        assert!(
            meter.read().clipping,
            "a full-scale sample must report clipping"
        );
        assert!(
            !meter.read().clipping,
            "clipping must clear after being read"
        );
    }

    #[test]
    fn silence_duration_accumulates_and_resets_on_signal() {
        let meter = LevelMeter::new(48_000);

        meter.observe(&[0.0; 24_000]);
        assert_eq!(meter.read().silence_duration_ms, 500);

        meter.observe(&[0.0; 24_000]);
        assert_eq!(meter.read().silence_duration_ms, 1000);

        meter.observe(&[0.5; 480]);
        assert_eq!(meter.read().silence_duration_ms, 0);
    }

    /// A noise floor is not silence to a naive check, but it is to a listener.
    #[test]
    fn a_quiet_noise_floor_still_counts_as_silence() {
        let meter = LevelMeter::new(48_000);
        meter.observe(&[0.0001; 48_000]);
        assert_eq!(meter.read().silence_duration_ms, 1000);
    }

    #[test]
    fn an_empty_block_changes_nothing() {
        let meter = LevelMeter::new(48_000);
        meter.observe(&[0.5; 10]);
        let before = meter.read();
        meter.observe(&[]);
        let after = meter.read();
        assert_eq!(before.peak, after.peak);
    }
}
