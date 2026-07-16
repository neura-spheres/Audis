//! Audio capture.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use audis_common::AudioSourceKind;
use cpal::traits::{DeviceTrait, StreamTrait};

use crate::device::{self, DeviceKind};
use crate::error::{AudioError, Result};
use crate::level::{AudioLevel, LevelMeter};

/// A running capture.
pub struct CaptureHandle {
    source: AudioSourceKind,
    meter: Arc<LevelMeter>,
    device_name: String,
    sample_rate: u32,
    channels: u16,
    /// Dropping the sender closes the channel, which stops the capture thread.
    _stop: Sender<()>,
}

impl CaptureHandle {
    /// Which source this capture represents.
    pub fn source(&self) -> AudioSourceKind {
        self.source
    }

    /// The device actually opened. May differ from the one requested if that
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// The stream's real sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// The stream's channel count.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Current signal level. Cheap: reads a few atomics.
    pub fn level(&self) -> AudioLevel {
        self.meter.read()
    }
}

impl std::fmt::Debug for CaptureHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CaptureHandle")
            .field("source", &self.source)
            .field("device_name", &self.device_name)
            .field("sample_rate", &self.sample_rate)
            .finish()
    }
}

/// Receives captured audio as interleaved `f32` frames in the device's native
pub type FrameSink = Arc<dyn Fn(&[f32]) + Send + Sync>;

/// Start capturing from a microphone, for levels only.
pub fn start_microphone(device_id: Option<&str>) -> Result<CaptureHandle> {
    start(
        AudioSourceKind::Microphone,
        DeviceKind::Input,
        device_id,
        None,
    )
}

/// Start capturing what the computer is playing, via WASAPI loopback.
pub fn start_computer_audio(device_id: Option<&str>) -> Result<CaptureHandle> {
    start(
        AudioSourceKind::ComputerAudio,
        DeviceKind::Output,
        device_id,
        None,
    )
}

/// Start microphone capture and deliver every frame to `sink`.
pub fn start_microphone_with_sink(
    device_id: Option<&str>,
    sink: FrameSink,
) -> Result<CaptureHandle> {
    start(
        AudioSourceKind::Microphone,
        DeviceKind::Input,
        device_id,
        Some(sink),
    )
}

/// Start loopback capture and deliver every frame to `sink`.
pub fn start_computer_audio_with_sink(
    device_id: Option<&str>,
    sink: FrameSink,
) -> Result<CaptureHandle> {
    start(
        AudioSourceKind::ComputerAudio,
        DeviceKind::Output,
        device_id,
        Some(sink),
    )
}

fn start(
    source: AudioSourceKind,
    kind: DeviceKind,
    device_id: Option<&str>,
    sink: Option<FrameSink>,
) -> Result<CaptureHandle> {
    let device = device::find(kind, device_id)?;
    let device_name = device::display_name(&device);

    let config = match kind {
        DeviceKind::Input => device.default_input_config(),
        DeviceKind::Output => device.default_output_config(),
    }
    .map_err(|error| AudioError::Format {
        device: device_name.clone(),
        detail: error.to_string(),
    })?;

    let sample_rate = config.sample_rate();
    let channels = config.channels();
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let meter = LevelMeter::new(sample_rate);
    let (stop_tx, stop_rx) = channel::<()>();
    let (ready_tx, ready_rx) = channel::<Result<()>>();

    let setup = StreamSetup {
        device,
        device_name: device_name.clone(),
        config: stream_config,
        sample_format,
        meter: Arc::clone(&meter),
        sink,
    };

    std::thread::Builder::new()
        .name(format!("audis-capture-{source:?}"))
        .spawn(move || {
            capture_thread(setup, ready_tx, stop_rx);
        })
        .map_err(|error| AudioError::StreamStart {
            device: device_name.clone(),
            detail: error.to_string(),
        })?;

    ready_rx.recv().map_err(|_| AudioError::StreamStart {
        device: device_name.clone(),
        detail: "the capture thread stopped before the stream opened".to_owned(),
    })??;

    tracing::info!(
        ?source,
        device = %device_name,
        sample_rate,
        channels,
        "capture started"
    );

    Ok(CaptureHandle {
        source,
        meter,
        device_name,
        sample_rate,
        channels,
        _stop: stop_tx,
    })
}

/// Everything needed to open one stream, moved to the capture thread as a unit.
struct StreamSetup {
    device: cpal::Device,
    device_name: String,
    config: cpal::StreamConfig,
    sample_format: cpal::SampleFormat,
    meter: Arc<LevelMeter>,
    sink: Option<FrameSink>,
}

/// Owns the cpal stream for its lifetime.
fn capture_thread(setup: StreamSetup, ready: Sender<Result<()>>, stop: Receiver<()>) {
    let device_name = setup.device_name.clone();

    let stream = match build_stream(setup) {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };

    if let Err(error) = stream.play() {
        let _ = ready.send(Err(AudioError::StreamStart {
            device: device_name,
            detail: error.to_string(),
        }));
        return;
    }

    let _ = ready.send(Ok(()));

    let _ = stop.recv();
    tracing::debug!(device = %device_name, "capture stopped");
}

fn build_stream(setup: StreamSetup) -> Result<cpal::Stream> {
    let StreamSetup {
        device,
        device_name,
        config,
        sample_format,
        meter,
        sink,
    } = setup;

    let error_name = device_name.clone();
    let on_error = move |error: cpal::Error| {
        tracing::warn!(device = %error_name, %error, "audio stream error");
    };

    let map_err = |error: cpal::Error| AudioError::StreamStart {
        device: device_name.clone(),
        detail: error.to_string(),
    };

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| {
                    meter.observe(data);
                    if let Some(sink) = &sink {
                        sink(data);
                    }
                },
                on_error,
                None,
            )
            .map_err(map_err)?,

        cpal::SampleFormat::I16 => device
            .build_input_stream(
                config,
                move |data: &[i16], _| {
                    let mut scratch = [0.0f32; 2048];
                    for chunk in data.chunks(scratch.len()) {
                        for (slot, &sample) in scratch.iter_mut().zip(chunk) {
                            *slot = f32::from(sample) / f32::from(i16::MAX);
                        }
                        meter.observe(&scratch[..chunk.len()]);
                        if let Some(sink) = &sink {
                            sink(&scratch[..chunk.len()]);
                        }
                    }
                },
                on_error,
                None,
            )
            .map_err(map_err)?,

        cpal::SampleFormat::U16 => device
            .build_input_stream(
                config,
                move |data: &[u16], _| {
                    let mut scratch = [0.0f32; 2048];
                    for chunk in data.chunks(scratch.len()) {
                        for (slot, &sample) in scratch.iter_mut().zip(chunk) {
                            *slot = (f32::from(sample) / f32::from(u16::MAX)) * 2.0 - 1.0;
                        }
                        meter.observe(&scratch[..chunk.len()]);
                        if let Some(sink) = &sink {
                            sink(&scratch[..chunk.len()]);
                        }
                    }
                },
                on_error,
                None,
            )
            .map_err(map_err)?,

        other => {
            return Err(AudioError::Format {
                device: device_name,
                detail: format!("unsupported sample format {other:?}"),
            });
        }
    };

    Ok(stream)
}
