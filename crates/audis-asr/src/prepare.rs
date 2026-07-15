//! Getting captured audio into the shape Whisper expects.
//!
//! Devices deliver 44.1 or 48 kHz, usually stereo. Whisper wants 16 kHz mono
//! `f32`. Two conversions, in this order: downmix first, then resample, because
//! resampling one channel is half the work of resampling two.
//!
//! [`Resampler`] is stateful on purpose. Audio arrives in small blocks, and a
//! filter that treats each block as an isolated buffer produces an artifact at
//! every block edge, which at 20 ms blocks means a click fifty times a second.
//! Carrying the filter history and the fractional read position across blocks
//! is what makes the stream continuous.

/// The rate every speech engine here runs at.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Taps in the anti-alias filter.
///
/// 63 gives roughly 40 dB of stopband rejection with a Hamming window, which
/// puts aliasing below Whisper's noise floor. Odd, so the filter has an exact
/// centre tap and a whole-sample group delay.
const FILTER_TAPS: usize = 63;

/// Cutoff as a fraction of the target Nyquist.
///
/// Below 1.0 so the transition band lands before the fold-over point rather
/// than straddling it. 8 kHz times 0.9 is 7.2 kHz, above everything that
/// matters in speech.
const CUTOFF_FRACTION: f64 = 0.9;

/// Average interleaved channels down to mono.
///
/// Averaging rather than taking channel 0: a stereo microphone with one dead
/// channel is common, and picking the dead one would silently produce silence.
pub fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return interleaved.to_vec();
    }

    let channels = channels as usize;
    interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// A continuous mono resampler to [`TARGET_SAMPLE_RATE`].
///
/// Feed it successive blocks from one stream. Create a new one per stream, and
/// never share one between the microphone and computer audio: they have
/// independent clocks and histories.
#[derive(Debug)]
pub struct Resampler {
    from_rate: u32,
    ratio: f64,
    kernel: Vec<f32>,
    /// Input that does not yet have full kernel support on both sides.
    ///
    /// A centred FIR needs `delay` samples of history *and* `delay` samples of
    /// lookahead. Emitting a sample before its lookahead has arrived is exactly
    /// the zero-pad artifact this design exists to avoid, so those samples wait
    /// here for the next block.
    pending: Vec<f32>,
    /// Filtered samples the decimator has not consumed yet.
    carry: Vec<f32>,
    /// Fractional read position within `carry`.
    ///
    /// Carried across blocks because 44.1 kHz to 16 kHz is not an integer
    /// ratio. Resetting it per block would drift the output by up to a sample
    /// each time and slowly desynchronise transcript timestamps from the audio.
    position: f64,
}

impl Resampler {
    /// A resampler for a stream running at `from_rate`.
    pub fn new(from_rate: u32) -> Self {
        let ratio = f64::from(from_rate) / f64::from(TARGET_SAMPLE_RATE);

        // Only meaningful when downsampling; upsampling invents no high content.
        let kernel = if ratio > 1.0 {
            let cutoff =
                (CUTOFF_FRACTION * f64::from(TARGET_SAMPLE_RATE) / 2.0) / f64::from(from_rate);
            lowpass_kernel(cutoff, FILTER_TAPS)
        } else {
            Vec::new()
        };

        Self {
            from_rate,
            ratio,
            kernel,
            pending: Vec::new(),
            carry: Vec::new(),
            position: 0.0,
        }
    }

    /// Resample one block of mono audio, continuing from the previous block.
    pub fn push(&mut self, mono: &[f32]) -> Vec<f32> {
        if self.from_rate == TARGET_SAMPLE_RATE {
            return mono.to_vec();
        }
        if mono.is_empty() {
            return Vec::new();
        }

        let filtered = self.filter(mono);
        self.carry.extend_from_slice(&filtered);
        self.decimate()
    }

    /// Low-pass the stream, emitting only samples with full kernel support.
    fn filter(&mut self, mono: &[f32]) -> Vec<f32> {
        if self.kernel.is_empty() {
            return mono.to_vec();
        }

        self.pending.extend_from_slice(mono);

        let delay = self.kernel.len() / 2;
        let len = self.pending.len();

        // Not enough context yet to emit anything without padding.
        if len < 2 * delay + 1 {
            return Vec::new();
        }

        let mut out = Vec::with_capacity(len - 2 * delay);
        for index in delay..(len - delay) {
            let mut sum = 0.0f32;
            for (tap_index, &tap) in self.kernel.iter().enumerate() {
                sum += self.pending[index + tap_index - delay] * tap;
            }
            out.push(sum);
        }

        // Keep the tail that the next block will need as its history, so the
        // next call resumes exactly where this one stopped.
        self.pending.drain(..len - 2 * delay);

        out
    }

    /// Pick output samples out of the filtered stream at the resample ratio.
    fn decimate(&mut self) -> Vec<f32> {
        let mut out = Vec::new();

        while self.position + 1.0 < self.carry.len() as f64 {
            let left = self.position.floor() as usize;
            let fraction = (self.position - left as f64) as f32;
            let a = self.carry[left];
            let b = self.carry[left + 1];
            out.push(a + (b - a) * fraction);
            self.position += self.ratio;
        }

        // Drop what has been read, keeping the fractional remainder.
        let consumed = self.position.floor() as usize;
        if consumed > 0 {
            let consumed = consumed.min(self.carry.len());
            self.carry.drain(..consumed);
            self.position -= consumed as f64;
        }

        out
    }
}

/// A Hamming-windowed sinc low-pass kernel.
///
/// `cutoff` is normalised to the input sample rate, so 0.25 means one quarter
/// of the sample rate.
fn lowpass_kernel(cutoff: f64, taps: usize) -> Vec<f32> {
    let centre = (taps - 1) as f64 / 2.0;
    let mut kernel = Vec::with_capacity(taps);
    let mut sum = 0.0f64;

    for index in 0..taps {
        let position = index as f64 - centre;

        // sinc, with the removable singularity at the centre handled.
        let sinc = if position.abs() < f64::EPSILON {
            2.0 * cutoff
        } else {
            (2.0 * std::f64::consts::PI * cutoff * position).sin()
                / (std::f64::consts::PI * position)
        };

        // Hamming: a slightly wider main lobe for much lower sidelobes, which
        // is the right trade for suppressing fold-over.
        let window =
            0.54 - 0.46 * (2.0 * std::f64::consts::PI * index as f64 / (taps - 1) as f64).cos();

        let tap = sinc * window;
        sum += tap;
        kernel.push(tap);
    }

    // Normalise to unity DC gain, so filtering does not change loudness.
    kernel.into_iter().map(|tap| (tap / sum) as f32).collect()
}

/// One-shot resample of a complete buffer.
///
/// For batch work such as retranscribing a saved recording. The live path uses
/// [`Resampler`], which is continuous across blocks.
pub fn resample_to_target(mono: &[f32], from_rate: u32) -> Vec<f32> {
    let mut resampler = Resampler::new(from_rate);
    resampler.push(mono)
}

/// Downmix and resample a complete buffer in one step.
pub fn prepare(interleaved: &[f32], channels: u16, sample_rate: u32) -> Vec<f32> {
    let mono = downmix_to_mono(interleaved, channels);
    resample_to_target(&mono, sample_rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_passes_through_untouched() {
        let samples = vec![0.1, 0.2, 0.3];
        assert_eq!(downmix_to_mono(&samples, 1), samples);
    }

    #[test]
    fn stereo_averages_both_channels() {
        let interleaved = [1.0, 0.0, 1.0, 0.0];
        assert_eq!(downmix_to_mono(&interleaved, 2), vec![0.5, 0.5]);
    }

    /// A stereo mic with one dead channel must not resolve to silence.
    #[test]
    fn a_dead_channel_does_not_silence_the_signal() {
        let interleaved = [0.0, 0.8, 0.0, 0.6];
        let mono = downmix_to_mono(&interleaved, 2);
        assert!(mono.iter().all(|s| *s > 0.0), "signal was lost: {mono:?}");
    }

    #[test]
    fn a_partial_frame_is_dropped_rather_than_misaligned() {
        let mono = downmix_to_mono(&[1.0, 1.0, 1.0, 1.0, 1.0], 2);
        assert_eq!(mono.len(), 2);
    }

    #[test]
    fn matching_rate_is_a_no_op() {
        let samples = vec![0.1, 0.2, 0.3];
        assert_eq!(resample_to_target(&samples, TARGET_SAMPLE_RATE), samples);
    }

    /// The filter withholds `2 * delay` input samples as lookahead, so a
    /// one-shot call is short by that much once divided by the ratio. It is a
    /// fixed warm-up of about 1.3 ms, not cumulative drift, which
    /// `a_non_integer_ratio_does_not_drift_over_many_blocks` covers separately.
    const WARMUP_SLACK: i64 = 30;

    #[test]
    fn downsampling_produces_roughly_the_expected_length() {
        // 48 kHz to 16 kHz is 3:1.
        let out = resample_to_target(&vec![0.5f32; 4800], 48_000);
        assert!(
            (out.len() as i64 - 1600).abs() <= WARMUP_SLACK,
            "got {}",
            out.len()
        );

        let out = resample_to_target(&vec![0.5f32; 4410], 44_100);
        assert!(
            (out.len() as i64 - 1600).abs() <= WARMUP_SLACK,
            "got {}",
            out.len()
        );
    }

    /// The point of the filter: a tone above the new Nyquist must not fold back.
    #[test]
    fn content_above_the_new_nyquist_is_attenuated() {
        let tone: Vec<f32> = (0..9600)
            .map(|n| (2.0 * std::f32::consts::PI * 12_000.0 * n as f32 / 48_000.0).sin())
            .collect();

        let out = resample_to_target(&tone, 48_000);
        // Skip the warm-up, then measure the steady state.
        let peak = out.iter().skip(100).fold(0.0f32, |acc, s| acc.max(s.abs()));

        assert!(peak < 0.1, "12 kHz tone aliased through at {peak}");
    }

    /// The filter must not eat the voice band.
    #[test]
    fn speech_band_content_survives() {
        let tone: Vec<f32> = (0..9600)
            .map(|n| (2.0 * std::f32::consts::PI * 300.0 * n as f32 / 48_000.0).sin())
            .collect();

        let out = resample_to_target(&tone, 48_000);
        let peak = out.iter().skip(100).fold(0.0f32, |acc, s| acc.max(s.abs()));

        assert!(peak > 0.7, "voice-band tone was over-attenuated to {peak}");
    }

    /// The reason `Resampler` is stateful. Feeding a constant in small blocks
    /// must not produce a click at every seam.
    #[test]
    fn streaming_in_blocks_has_no_seam_artifacts() {
        let mut resampler = Resampler::new(48_000);
        let block = vec![0.5f32; 480]; // 10 ms at 48 kHz

        let mut out = Vec::new();
        for _ in 0..20 {
            out.extend(resampler.push(&block));
        }

        // Skip the warm-up, then every sample must still be the constant.
        for (index, sample) in out.iter().enumerate().skip(80) {
            assert!(
                (sample - 0.5).abs() < 0.02,
                "seam artifact at {index}: {sample}"
            );
        }
    }

    /// Block-by-block must match a single-shot resample of the same audio.
    #[test]
    fn streaming_matches_one_shot_length() {
        let tone: Vec<f32> = (0..4800)
            .map(|n| (2.0 * std::f32::consts::PI * 440.0 * n as f32 / 48_000.0).sin())
            .collect();

        let mut resampler = Resampler::new(48_000);
        let mut streamed = Vec::new();
        for block in tone.chunks(480) {
            streamed.extend(resampler.push(block));
        }

        // Ten blocks of 480 at 3:1 is 1600 output samples, less the warm-up.
        assert!(
            (streamed.len() as i64 - 1600).abs() <= WARMUP_SLACK,
            "streamed length drifted: {}",
            streamed.len()
        );
    }

    /// Fractional carry matters: 44.1 kHz is not an integer ratio, so without
    /// it the output would drift a sample per block and desync timestamps.
    #[test]
    fn a_non_integer_ratio_does_not_drift_over_many_blocks() {
        let mut resampler = Resampler::new(44_100);
        let block = vec![0.2f32; 441]; // 10 ms at 44.1 kHz

        let mut total = 0usize;
        for _ in 0..100 {
            total += resampler.push(&block).len();
        }

        // 100 blocks of 10 ms is 1 second, so 16000 samples at the target rate.
        let drift = (total as i64 - 16_000).abs();
        assert!(drift < 50, "drifted {drift} samples over one second");
    }

    #[test]
    fn prepare_handles_stereo_48k_the_common_case() {
        let out = prepare(&vec![0.3f32; 9600], 2, 48_000);
        assert!(
            (out.len() as i64 - 1600).abs() <= WARMUP_SLACK,
            "got {}",
            out.len()
        );
    }

    #[test]
    fn empty_input_does_not_panic() {
        assert!(prepare(&[], 2, 48_000).is_empty());
        assert!(resample_to_target(&[], 48_000).is_empty());

        let mut resampler = Resampler::new(48_000);
        assert!(resampler.push(&[]).is_empty());
    }
}
