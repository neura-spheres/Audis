//! Manual hardware check: enumerate devices, then capture from the default
//! microphone and the default output (loopback) for a few seconds and print
//! live levels.
//!
//! Run with `cargo run -p audis-audio --example probe`. Play some audio while
//! it runs and the computer-audio meter should move.

fn main() {
    let devices = match audis_audio::enumerate() {
        Ok(devices) => devices,
        Err(error) => {
            eprintln!("could not enumerate: {error}");
            std::process::exit(1);
        }
    };

    println!("INPUTS ({})", devices.inputs.len());
    for device in &devices.inputs {
        let marker = if device.is_default { "*" } else { " " };
        println!(
            "  {marker} {} [{} Hz, {} ch]",
            device.name, device.sample_rate, device.channels
        );
    }

    println!("OUTPUTS ({})", devices.outputs.len());
    for device in &devices.outputs {
        let marker = if device.is_default { "*" } else { " " };
        println!(
            "  {marker} {} [{} Hz, {} ch]",
            device.name, device.sample_rate, device.channels
        );
    }

    let microphone = audis_audio::start_microphone(None);
    let computer = audis_audio::start_computer_audio(None);

    match &microphone {
        Ok(handle) => println!(
            "\nMIC OK: {} @ {} Hz",
            handle.device_name(),
            handle.sample_rate()
        ),
        Err(error) => println!("\nMIC FAILED: {error}"),
    }
    match &computer {
        Ok(handle) => println!(
            "LOOPBACK OK: {} @ {} Hz",
            handle.device_name(),
            handle.sample_rate()
        ),
        Err(error) => println!("LOOPBACK FAILED: {error}"),
    }

    println!("\nsampling levels for 5s (play audio to move the loopback meter)");
    let mut mic_peak = 0.0f32;
    let mut loopback_peak = 0.0f32;

    for _ in 0..25 {
        std::thread::sleep(std::time::Duration::from_millis(200));

        if let Ok(handle) = &microphone {
            let level = handle.level();
            mic_peak = mic_peak.max(level.peak);
        }
        if let Ok(handle) = &computer {
            let level = handle.level();
            loopback_peak = loopback_peak.max(level.peak);
        }
    }

    println!("mic peak over 5s:      {mic_peak:.4}");
    println!("loopback peak over 5s: {loopback_peak:.4}");
    println!(
        "\nloopback capture: {}",
        if loopback_peak > 0.0 {
            "RECEIVING AUDIO"
        } else {
            "silent (nothing was playing, or loopback is not working)"
        }
    );
}
