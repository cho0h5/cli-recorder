use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{SampleFormat, SupportedStreamConfig};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    print_input_device_list()?;
    Ok(())
}

fn print_input_device_list() -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();
    let devices = host.input_devices()?;

    println!("Input audio device list:\n");

    for (i, device) in devices.enumerate() {
        let name = device.name().unwrap_or_else(|_| "<Cannot get name>".into());
        println!("{}. {}", i + 1, name);

        let default_cfg = device.default_input_config().ok();

        match device.supported_input_configs() {
            Ok(configs) => {
                for config in configs {
                    let is_default = default_cfg.as_ref().map_or(false, |def| {
                        config.channels() == def.channels()
                            && config.sample_format() == def.sample_format()
                            && def.sample_rate().0 >= config.min_sample_rate().0
                            && def.sample_rate().0 <= config.max_sample_rate().0
                    });
                    print!(
                        "    channels: {}, sample_rate: {} ~ {}, sample_format: {:?}",
                        config.channels(),
                        config.min_sample_rate().0,
                        config.max_sample_rate().0,
                        config.sample_format()
                    );
                    if is_default {
                        print!(" (default)");
                    }
                    println!();
                }
            }
            Err(e) => {
                println!("    Could not get supported configs: {:?}", e);
            }
        }
        println!();
    }
    Ok(())
}
