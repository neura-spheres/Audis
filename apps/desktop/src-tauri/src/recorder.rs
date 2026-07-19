use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

use audiopus::coder::Encoder as OpusEncoder;
use audiopus::{Application, Bitrate, Channels, SampleRate};
use ogg::writing::{PacketWriteEndInfo, PacketWriter};

const OPUS_RATE: u32 = 48_000;
const FRAME_SAMPLES: usize = 960;
const BITRATE: i32 = 64_000;
const SERIAL: u32 = 0x00A0_D157;

pub fn spawn(
    path: PathBuf,
    input_rate: u32,
    channels: u16,
    frames: Receiver<Vec<f32>>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name("audis-record".to_owned())
        .spawn(move || match record(&path, input_rate, channels, frames) {
            Ok(()) => tracing::info!(?path, "session recording saved"),
            Err(error) => tracing::error!(%error, ?path, "session recording failed"),
        })
}

fn record(
    path: &PathBuf,
    input_rate: u32,
    channels: u16,
    frames: Receiver<Vec<f32>>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut writer = OggOpus::new(BufWriter::new(file), input_rate, channels)?;

    while let Ok(frame) = frames.recv() {
        writer.push(&frame)?;
    }
    writer.finish()
}

struct OggOpus<W: std::io::Write> {
    encoder: OpusEncoder,
    ogg: PacketWriter<'static, W>,
    channels: u16,
    resampler: Resampler,
    pending: Vec<f32>,
    granule: u64,
    out: Vec<u8>,
}

impl<W: std::io::Write> OggOpus<W> {
    fn new(file: W, input_rate: u32, channels: u16) -> Result<Self, String> {
        let mut encoder = OpusEncoder::new(SampleRate::Hz48000, Channels::Mono, Application::Audio)
            .map_err(|error| format!("opus encoder: {error}"))?;
        encoder
            .set_bitrate(Bitrate::BitsPerSecond(BITRATE))
            .map_err(|error| format!("opus bitrate: {error}"))?;

        let pre_skip: u16 = encoder
            .lookahead()
            .ok()
            .and_then(|value| u16::try_from(value).ok())
            .unwrap_or(312);

        let mut ogg = PacketWriter::new(file);
        ogg.write_packet(
            opus_head(1, pre_skip, input_rate),
            SERIAL,
            PacketWriteEndInfo::EndPage,
            0,
        )
        .map_err(|error| error.to_string())?;
        ogg.write_packet(opus_tags(), SERIAL, PacketWriteEndInfo::EndPage, 0)
            .map_err(|error| error.to_string())?;

        Ok(Self {
            encoder,
            ogg,
            channels,
            resampler: Resampler::new(input_rate),
            pending: Vec::new(),
            granule: u64::from(pre_skip),
            out: vec![0u8; 4000],
        })
    }

    fn push(&mut self, interleaved: &[f32]) -> Result<(), String> {
        let mono = downmix(interleaved, self.channels);
        self.resampler.process(&mono, &mut self.pending);
        while self.pending.len() >= FRAME_SAMPLES {
            let frame: Vec<i16> = self.pending.drain(..FRAME_SAMPLES).map(to_i16).collect();
            self.granule += FRAME_SAMPLES as u64;
            self.write_frame(&frame, false)?;
        }
        Ok(())
    }

    fn write_frame(&mut self, samples: &[i16], last: bool) -> Result<(), String> {
        let written = self
            .encoder
            .encode(samples, &mut self.out)
            .map_err(|error| format!("opus encode: {error}"))?;
        let packet = self.out[..written].to_vec();
        let info = if last {
            PacketWriteEndInfo::EndStream
        } else {
            PacketWriteEndInfo::NormalPacket
        };
        self.ogg
            .write_packet(packet, SERIAL, info, self.granule)
            .map_err(|error| error.to_string())
    }

    fn finish(mut self) -> Result<(), String> {
        let remaining = self.pending.len();
        if remaining > 0 {
            self.granule += remaining as u64;
            let mut frame: Vec<i16> = self.pending.iter().map(|s| to_i16(*s)).collect();
            frame.resize(FRAME_SAMPLES, 0);
            self.write_frame(&frame, true)
        } else {
            self.write_frame(&[0i16; FRAME_SAMPLES], true)
        }
    }
}

struct Resampler {
    step: f64,
    t: f64,
    index: f64,
    prev: f32,
}

impl Resampler {
    fn new(input_rate: u32) -> Self {
        Self {
            step: f64::from(input_rate) / f64::from(OPUS_RATE),
            t: 0.0,
            index: 0.0,
            prev: 0.0,
        }
    }

    fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        for &sample in input {
            while self.t <= self.index {
                let frac = (self.t - (self.index - 1.0)) as f32;
                out.push(self.prev * (1.0 - frac) + sample * frac);
                self.t += self.step;
            }
            self.prev = sample;
            self.index += 1.0;
        }
    }
}

fn downmix(interleaved: &[f32], channels: u16) -> Vec<f32> {
    let ch = channels.max(1) as usize;
    if ch == 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(ch)
        .map(|frame| frame.iter().sum::<f32>() / ch as f32)
        .collect()
}

fn to_i16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * 32767.0) as i16
}

fn opus_head(channels: u8, pre_skip: u16, input_rate: u32) -> Vec<u8> {
    let mut head = Vec::with_capacity(19);
    head.extend_from_slice(b"OpusHead");
    head.push(1);
    head.push(channels);
    head.extend_from_slice(&pre_skip.to_le_bytes());
    head.extend_from_slice(&input_rate.to_le_bytes());
    head.extend_from_slice(&0u16.to_le_bytes());
    head.push(0);
    head
}

fn opus_tags() -> Vec<u8> {
    let vendor = b"Audis";
    let mut tags = Vec::new();
    tags.extend_from_slice(b"OpusTags");
    tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
    tags.extend_from_slice(vendor);
    tags.extend_from_slice(&0u32.to_le_bytes());
    tags
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_playable_opus_stream() {
        let mut bytes = Vec::new();
        {
            let mut writer = OggOpus::new(BufWriter::new(&mut bytes), 44_100, 1).expect("encoder");
            let tone: Vec<f32> = (0..44_100)
                .map(|n| (std::f32::consts::TAU * 440.0 * n as f32 / 44_100.0).sin() * 0.3)
                .collect();
            writer.push(&tone).expect("push");
            writer.finish().expect("finish");
        }
        assert_eq!(&bytes[..4], b"OggS", "must be an Ogg stream");
        assert!(
            bytes.windows(8).any(|w| w == b"OpusHead"),
            "must carry an Opus header"
        );
        assert!(
            bytes.len() > 2000,
            "a second of tone is not tiny: {}",
            bytes.len()
        );
    }

    #[test]
    fn resampler_passes_through_at_48k() {
        let mut r = Resampler::new(48_000);
        let mut out = Vec::new();
        r.process(&[0.1, 0.2, 0.3, 0.4], &mut out);
        assert_eq!(out.len(), 4);
        assert!((out[0] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn resampler_upsamples_44100_to_48000() {
        let mut r = Resampler::new(44_100);
        let mut out = Vec::new();
        let input = vec![0.0f32; 44_100];
        r.process(&input, &mut out);
        let ratio = out.len() as f64 / 48_000.0;
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "expected ~48000 samples, got {}",
            out.len()
        );
    }

    #[test]
    fn opus_head_is_well_formed() {
        let head = opus_head(1, 312, 48_000);
        assert_eq!(&head[..8], b"OpusHead");
        assert_eq!(head.len(), 19);
        assert_eq!(head[9], 1);
    }
}
