//! Audio device enumeration.

use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};

use crate::error::{AudioError, Result};

/// What a device is used for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    /// A microphone or other capture endpoint.
    Input,
    /// A speaker or headphone endpoint. Audis captures these with WASAPI
    /// loopback to hear what the computer is playing.
    Output,
}

/// One audio endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    /// Stable identifier, from cpal's `DeviceId`.
    ///
    /// Safe to persist in settings: it survives restarts and is matched back
    /// with `FromStr`, unlike a friendly name which changes when Windows
    /// renames an endpoint.
    pub id: String,
    /// Name shown to the user.
    pub name: String,
    /// Input or output.
    pub kind: DeviceKind,
    /// Whether Windows considers this the default endpoint.
    pub is_default: bool,
    /// Sample rate of the device's shared-mode format.
    pub sample_rate: u32,
    /// Channel count of the device's shared-mode format.
    pub channels: u16,
}

/// Everything Audis can capture from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevices {
    /// Microphones.
    pub inputs: Vec<AudioDevice>,
    /// Outputs, capturable via loopback.
    pub outputs: Vec<AudioDevice>,
}

/// Enumerate every input and output endpoint.
///
/// A device that fails to report its id or format is skipped rather than
/// failing the whole enumeration: one broken driver should not empty the picker.
pub fn enumerate() -> Result<AudioDevices> {
    let host = cpal::default_host();

    let default_input = host
        .default_input_device()
        .and_then(|device| device.id().ok());
    let default_output = host
        .default_output_device()
        .and_then(|device| device.id().ok());

    let inputs = host
        .input_devices()
        .map_err(|error| AudioError::Enumeration {
            detail: error.to_string(),
        })?
        .filter_map(|device| describe(&device, DeviceKind::Input, default_input.as_ref()))
        .collect();

    let outputs = host
        .output_devices()
        .map_err(|error| AudioError::Enumeration {
            detail: error.to_string(),
        })?
        .filter_map(|device| describe(&device, DeviceKind::Output, default_output.as_ref()))
        .collect();

    Ok(AudioDevices { inputs, outputs })
}

fn describe(
    device: &cpal::Device,
    kind: DeviceKind,
    default_id: Option<&cpal::DeviceId>,
) -> Option<AudioDevice> {
    let id = device.id().ok()?;
    let name = device
        .description()
        .ok()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|| id.id().to_owned());

    // An output endpoint's output format is what loopback delivers, so each
    // kind asks for the config it will actually run at.
    let config = match kind {
        DeviceKind::Input => device.default_input_config().ok()?,
        DeviceKind::Output => device.default_output_config().ok()?,
    };

    Some(AudioDevice {
        is_default: default_id == Some(&id),
        id: id.to_string(),
        name,
        kind,
        sample_rate: config.sample_rate(),
        channels: config.channels(),
    })
}

/// Find a device by id, or fall back to the system default.
///
/// Falling back matters: a saved device may have been unplugged since, and
/// refusing to start would be worse than using the default and saying so.
pub fn find(kind: DeviceKind, id: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();

    if let Some(wanted) = id {
        let mut devices = match kind {
            DeviceKind::Input => host.input_devices(),
            DeviceKind::Output => host.output_devices(),
        }
        .map_err(|error| AudioError::Enumeration {
            detail: error.to_string(),
        })?;

        let found =
            devices.find(|device| device.id().ok().is_some_and(|id| id.to_string() == wanted));

        if let Some(device) = found {
            return Ok(device);
        }

        tracing::warn!(
            requested = wanted,
            "device not found; using the system default"
        );
    }

    match kind {
        DeviceKind::Input => host.default_input_device(),
        DeviceKind::Output => host.default_output_device(),
    }
    .ok_or(AudioError::NoDevice { kind })
}

/// A device's display name, or a fallback that is never empty.
pub fn display_name(device: &cpal::Device) -> String {
    device
        .description()
        .ok()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|| "Unknown device".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serde naming is load-bearing: the frontend reads these keys.
    #[test]
    fn devices_serialise_as_camel_case() {
        let device = AudioDevice {
            id: "wasapi:{0.0.1.00000000}".to_owned(),
            name: "Microphone (Realtek)".to_owned(),
            kind: DeviceKind::Input,
            is_default: true,
            sample_rate: 48_000,
            channels: 2,
        };

        let json = serde_json::to_value(&device).expect("serialise");

        assert_eq!(json["isDefault"], true);
        assert_eq!(json["sampleRate"], 48_000);
        assert_eq!(json["kind"], "input");
    }

    /// Runs against whatever hardware this machine has, so it must not panic on
    /// a machine with no audio devices at all, such as a CI runner.
    #[test]
    fn enumeration_does_not_panic_without_hardware() {
        if let Ok(devices) = enumerate() {
            for device in devices.inputs.iter().chain(&devices.outputs) {
                assert!(!device.name.is_empty(), "a device reported an empty name");
                assert!(!device.id.is_empty(), "a device reported an empty id");
                assert!(device.sample_rate > 0);
            }
        }
    }

    /// At most one device per kind may claim to be the default.
    #[test]
    fn at_most_one_default_per_kind() {
        if let Ok(devices) = enumerate() {
            assert!(devices.inputs.iter().filter(|d| d.is_default).count() <= 1);
            assert!(devices.outputs.iter().filter(|d| d.is_default).count() <= 1);
        }
    }
}
