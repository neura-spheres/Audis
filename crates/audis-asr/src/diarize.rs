//! Speaker diarization for the computer-audio stream.

use crate::prepare::TARGET_SAMPLE_RATE;

const FRAME_MS: u32 = 25;
const HOP_MS: u32 = 10;
const FFT_SIZE: usize = 512;
const N_MELS: usize = 26;
const N_MFCC: usize = 13;
const PRE_EMPHASIS: f32 = 0.97;
const MEL_LOW_HZ: f32 = 20.0;

/// Policy for deciding when two utterances are the same speaker.
#[derive(Debug, Clone, Copy)]
pub struct DiarizeConfig {
    /// Cosine similarity at or above which an utterance joins an existing speaker.
    pub similarity_threshold: f32,
    /// Upper bound on the number of distinct speakers.
    pub max_speakers: usize,
    /// Utterances shorter than this are left unlabelled.
    pub min_duration_ms: u32,
}

impl Default for DiarizeConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.72,
            max_speakers: 8,
            min_duration_ms: 500,
        }
    }
}

/// The speaker an utterance was assigned to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeakerAssignment {
    /// Zero-based speaker index, in order of first appearance.
    pub index: usize,
    /// True when this utterance created the speaker.
    pub is_new: bool,
}

#[derive(Debug, Clone)]
struct Centroid {
    sum: Vec<f32>,
    count: u32,
}

impl Centroid {
    fn new(embedding: &[f32]) -> Self {
        Self {
            sum: embedding.to_vec(),
            count: 1,
        }
    }

    fn direction(&self) -> Vec<f32> {
        normalize(&self.sum)
    }

    fn absorb(&mut self, embedding: &[f32]) {
        for (slot, value) in self.sum.iter_mut().zip(embedding) {
            *slot += *value;
        }
        self.count += 1;
    }
}

/// Online speaker clustering for one stream.
#[derive(Debug)]
pub struct Diarizer {
    config: DiarizeConfig,
    centroids: Vec<Centroid>,
}

impl Diarizer {
    /// A diarizer with the given policy and no speakers yet.
    pub fn new(config: DiarizeConfig) -> Self {
        Self {
            config,
            centroids: Vec::new(),
        }
    }

    /// How many distinct speakers have been seen so far.
    pub fn speaker_count(&self) -> usize {
        self.centroids.len()
    }

    /// Place one utterance's 16 kHz mono samples with a speaker, or `None` when
    /// the audio is too short or too quiet to place.
    pub fn identify(&mut self, samples: &[f32]) -> Option<SpeakerAssignment> {
        let min_samples = (u64::from(self.config.min_duration_ms) * u64::from(TARGET_SAMPLE_RATE)
            / 1000) as usize;
        if samples.len() < min_samples {
            return None;
        }

        let embedding = embed(samples)?;
        Some(self.assign(&embedding))
    }

    fn assign(&mut self, embedding: &[f32]) -> SpeakerAssignment {
        let best = self
            .centroids
            .iter()
            .map(|centroid| cosine(&centroid.direction(), embedding))
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b));

        match best {
            Some((index, similarity)) if similarity >= self.config.similarity_threshold => {
                self.centroids[index].absorb(embedding);
                SpeakerAssignment {
                    index,
                    is_new: false,
                }
            }
            _ if self.centroids.len() < self.config.max_speakers => {
                self.centroids.push(Centroid::new(embedding));
                SpeakerAssignment {
                    index: self.centroids.len() - 1,
                    is_new: true,
                }
            }
            Some((index, _)) => {
                self.centroids[index].absorb(embedding);
                SpeakerAssignment {
                    index,
                    is_new: false,
                }
            }
            None => {
                self.centroids.push(Centroid::new(embedding));
                SpeakerAssignment {
                    index: 0,
                    is_new: true,
                }
            }
        }
    }
}

/// The provisional label for a speaker index: `Person 1`, `Person 2`, …
pub fn speaker_label(index: usize) -> String {
    format!("Person {}", index + 1)
}

/// A stable machine id for a speaker index, such as `person-1`.
pub fn speaker_id(index: usize) -> String {
    format!("person-{}", index + 1)
}

/// Turn 16 kHz mono samples into a fixed-length speaker embedding, or `None`
/// when the audio carries no analysable signal.
pub fn embed(samples: &[f32]) -> Option<Vec<f32>> {
    let frame_len = (FRAME_MS * TARGET_SAMPLE_RATE / 1000) as usize;
    let hop = (HOP_MS * TARGET_SAMPLE_RATE / 1000) as usize;
    if samples.len() < frame_len || hop == 0 {
        return None;
    }

    let mean_square = samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32;
    if mean_square < 1e-8 {
        return None;
    }

    let emphasised = pre_emphasis(samples);
    let window = hamming(frame_len);
    let filterbank = MelFilterbank::new(TARGET_SAMPLE_RATE, FFT_SIZE, N_MELS);

    let mut statics: Vec<[f32; N_MFCC]> = Vec::new();
    let mut start = 0;
    while start + frame_len <= emphasised.len() {
        let frame = &emphasised[start..start + frame_len];
        start += hop;

        let power = power_spectrum(frame, &window);
        let mel = filterbank.apply(&power);
        let mfcc = dct_ii(&mel, N_MFCC);
        let mut coeffs = [0.0f32; N_MFCC];
        coeffs.copy_from_slice(&mfcc);
        statics.push(coeffs);
    }

    if statics.is_empty() {
        return None;
    }

    let deltas = derivatives(&statics);

    let mut embedding = Vec::with_capacity(N_MFCC * 4);
    append_pooled(&mut embedding, &statics);
    append_pooled(&mut embedding, &deltas);

    let normalised = normalize(&embedding);
    if normalised.iter().all(|value| *value == 0.0) {
        return None;
    }
    Some(normalised)
}

fn derivatives(frames: &[[f32; N_MFCC]]) -> Vec<[f32; N_MFCC]> {
    let len = frames.len();
    (0..len)
        .map(|i| {
            let prev = frames[i.saturating_sub(1)];
            let next = frames[(i + 1).min(len - 1)];
            let mut delta = [0.0f32; N_MFCC];
            for c in 0..N_MFCC {
                delta[c] = (next[c] - prev[c]) / 2.0;
            }
            delta
        })
        .collect()
}

fn append_pooled(out: &mut Vec<f32>, frames: &[[f32; N_MFCC]]) {
    let count = frames.len() as f64;
    for c in 0..N_MFCC {
        let mut sum = 0.0f64;
        let mut sum_sq = 0.0f64;
        for frame in frames {
            sum += f64::from(frame[c]);
            sum_sq += f64::from(frame[c]) * f64::from(frame[c]);
        }
        let mean = sum / count;
        let variance = (sum_sq / count - mean * mean).max(0.0);
        out.push(mean as f32);
        out.push(variance.sqrt() as f32);
    }
}

fn pre_emphasis(samples: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(samples.len());
    let mut previous = 0.0f32;
    for &sample in samples {
        out.push(sample - PRE_EMPHASIS * previous);
        previous = sample;
    }
    out
}

fn hamming(len: usize) -> Vec<f32> {
    if len <= 1 {
        return vec![1.0; len.max(1)];
    }
    let denom = (len - 1) as f32;
    (0..len)
        .map(|n| 0.54 - 0.46 * (std::f32::consts::TAU * n as f32 / denom).cos())
        .collect()
}

fn power_spectrum(frame: &[f32], window: &[f32]) -> Vec<f32> {
    let mut re = vec![0.0f32; FFT_SIZE];
    let mut im = vec![0.0f32; FFT_SIZE];
    for (i, (&sample, &weight)) in frame.iter().zip(window).enumerate() {
        re[i] = sample * weight;
    }

    fft(&mut re, &mut im);

    let bins = FFT_SIZE / 2 + 1;
    (0..bins).map(|k| re[k] * re[k] + im[k] * im[k]).collect()
}

fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two());

    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2;
    while len <= n {
        let angle = -std::f32::consts::TAU / len as f32;
        let (wr_step, wi_step) = (angle.cos(), angle.sin());
        let half = len / 2;

        let mut start = 0;
        while start < n {
            let (mut wr, mut wi) = (1.0f32, 0.0f32);
            for k in 0..half {
                let a = start + k;
                let b = a + half;
                let tr = wr * re[b] - wi * im[b];
                let ti = wr * im[b] + wi * re[b];
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next_wr = wr * wr_step - wi * wi_step;
                wi = wr * wi_step + wi * wr_step;
                wr = next_wr;
            }
            start += len;
        }
        len <<= 1;
    }
}

struct MelFilterbank {
    filters: Vec<Vec<(usize, f32)>>,
}

impl MelFilterbank {
    fn new(sample_rate: u32, fft_size: usize, n_mels: usize) -> Self {
        let bins = fft_size / 2 + 1;
        let high_hz = sample_rate as f32 / 2.0;
        let low_mel = hz_to_mel(MEL_LOW_HZ);
        let high_mel = hz_to_mel(high_hz);

        let points: Vec<f32> = (0..n_mels + 2)
            .map(|i| {
                let mel = low_mel + (high_mel - low_mel) * i as f32 / (n_mels + 1) as f32;
                let hz = mel_to_hz(mel);
                hz * fft_size as f32 / sample_rate as f32
            })
            .collect();

        let filters = (0..n_mels)
            .map(|m| {
                let (left, centre, right) = (points[m], points[m + 1], points[m + 2]);
                let mut taps = Vec::new();
                for bin in 0..bins {
                    let x = bin as f32;
                    let weight = if x >= left && x <= centre && centre > left {
                        (x - left) / (centre - left)
                    } else if x > centre && x <= right && right > centre {
                        (right - x) / (right - centre)
                    } else {
                        0.0
                    };
                    if weight > 0.0 {
                        taps.push((bin, weight));
                    }
                }
                taps
            })
            .collect();

        Self { filters }
    }

    fn apply(&self, power: &[f32]) -> Vec<f32> {
        self.filters
            .iter()
            .map(|taps| {
                let energy: f32 = taps.iter().map(|&(bin, weight)| power[bin] * weight).sum();
                (energy + 1e-10).ln()
            })
            .collect()
    }
}

fn hz_to_mel(hz: f32) -> f32 {
    2595.0 * (1.0 + hz / 700.0).log10()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * (10.0f32.powf(mel / 2595.0) - 1.0)
}

fn dct_ii(input: &[f32], n_out: usize) -> Vec<f32> {
    let n = input.len();
    (1..=n_out)
        .map(|k| {
            let mut acc = 0.0f32;
            for (i, &value) in input.iter().enumerate() {
                acc +=
                    value * (std::f32::consts::PI * (i as f32 + 0.5) * k as f32 / n as f32).cos();
            }
            acc
        })
        .collect()
}

fn normalize(vector: &[f32]) -> Vec<f32> {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return vec![0.0; vector.len()];
    }
    vector.iter().map(|v| v / norm).collect()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const RATE: u32 = TARGET_SAMPLE_RATE;

    fn voice(fundamental: f32, harmonics: &[(f32, f32)], seed: u64, ms: u32) -> Vec<f32> {
        let len = (ms * RATE / 1000) as usize;
        let mut state = seed.wrapping_mul(2862933555777941757).wrapping_add(1);
        (0..len)
            .map(|n| {
                let t = n as f32 / RATE as f32;
                let mut sample = 0.0f32;
                for (index, &(multiple, gain)) in harmonics.iter().enumerate() {
                    let phase = std::f32::consts::TAU * fundamental * multiple * t;
                    sample += gain * (phase + index as f32).sin();
                }
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let noise = ((state >> 33) as f32 / (1u64 << 31) as f32) - 1.0;
                (sample + 0.05 * noise) * 0.3
            })
            .collect()
    }

    fn alice(seed: u64) -> Vec<f32> {
        voice(
            120.0,
            &[(1.0, 1.0), (2.0, 0.5), (3.0, 0.7), (5.0, 0.3)],
            seed,
            1200,
        )
    }

    fn bob(seed: u64) -> Vec<f32> {
        voice(
            210.0,
            &[(1.0, 0.6), (2.0, 1.0), (4.0, 0.5), (7.0, 0.4)],
            seed,
            1200,
        )
    }

    #[test]
    fn embedding_is_stable_for_one_voice_and_differs_between_voices() {
        let a1 = embed(&alice(1)).expect("alice embeds");
        let a2 = embed(&alice(2)).expect("alice embeds again");
        let b1 = embed(&bob(3)).expect("bob embeds");

        let same = cosine(&a1, &a2);
        let different = cosine(&a1, &b1);

        assert!(
            same > 0.9,
            "the same voice should embed consistently: {same}"
        );
        assert!(
            same > different + 0.05,
            "two voices should be less similar than one voice to itself: same={same}, different={different}"
        );
    }

    #[test]
    fn short_or_silent_audio_is_left_unlabelled() {
        let mut diarizer = Diarizer::new(DiarizeConfig::default());
        assert!(diarizer.identify(&vec![0.2; 1000]).is_none());
        assert!(embed(&vec![0.0; RATE as usize]).is_none());
    }

    #[test]
    fn two_voices_become_two_speakers_and_repeats_reuse_them() {
        let mut diarizer = Diarizer::new(DiarizeConfig::default());

        let first = diarizer.identify(&alice(10)).expect("placed");
        assert_eq!(
            first,
            SpeakerAssignment {
                index: 0,
                is_new: true
            }
        );

        let second = diarizer.identify(&bob(11)).expect("placed");
        assert_eq!(
            second,
            SpeakerAssignment {
                index: 1,
                is_new: true
            }
        );

        let alice_again = diarizer.identify(&alice(12)).expect("placed");
        assert_eq!(alice_again.index, 0);
        assert!(!alice_again.is_new);

        let bob_again = diarizer.identify(&bob(13)).expect("placed");
        assert_eq!(bob_again.index, 1);
        assert!(!bob_again.is_new);

        assert_eq!(diarizer.speaker_count(), 2);
    }

    #[test]
    fn the_speaker_cap_is_never_exceeded() {
        let config = DiarizeConfig {
            max_speakers: 1,
            ..DiarizeConfig::default()
        };
        let mut diarizer = Diarizer::new(config);

        diarizer.identify(&alice(20)).expect("placed");
        let crowded = diarizer.identify(&bob(21)).expect("placed");
        assert_eq!(crowded.index, 0);
        assert!(!crowded.is_new);
        assert_eq!(diarizer.speaker_count(), 1);
    }

    #[test]
    fn labels_and_ids_are_one_based_and_stable() {
        assert_eq!(speaker_label(0), "Person 1");
        assert_eq!(speaker_label(4), "Person 5");
        assert_eq!(speaker_id(0), "person-1");
    }
}
