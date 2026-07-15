//! Audio device enumeration and capture for Audis.
//!
//! Microphone and system-playback (loopback) capture run as two independent
//! streams and are never mixed. Which device audio arrived on is free, perfect
//! attribution: the microphone is the local user, loopback is everyone else.
//! Mixing them would throw that away and no model could recover it.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

pub mod capture;
pub mod device;
pub mod error;
pub mod level;

pub use capture::{
    CaptureHandle, FrameSink, start_computer_audio, start_computer_audio_with_sink,
    start_microphone, start_microphone_with_sink,
};
pub use device::{AudioDevice, AudioDevices, DeviceKind, enumerate};
pub use error::{AudioError, Result};
pub use level::{AudioLevel, LevelMeter};
