use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait};
use std::error::Error;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    device: Option<String>,

    #[arg(long)]
    file: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let (device, file) = match parse_args_and_select_device()? {
        Some(result) => result,
        None => return Ok(()),
    };

    println!("Selected device: {}", device.name()?);
    println!("Output file: {}", file);

    Ok(())
}

fn parse_args_and_select_device() -> Result<Option<(cpal::Device, String)>, Box<dyn Error>> {
    let args = Args::parse();

    if args.device.is_none() && args.file.is_none() {
        println!("Usage: Run with --device <name> --file <filename.wav>\n");
        print_input_device_list()?;
        return Ok(None);
    }

    if args.device.is_none() || args.file.is_none() {
        eprintln!("Error: Both --device <name> and --file <filename.wav> options are required.");
        std::process::exit(1);
    }

    let file = args.file.as_ref().unwrap().clone();
    let device_name = args.device.as_ref().unwrap().to_lowercase();

    let host = cpal::default_host();
    let devices = host.input_devices()?;

    let device = devices
        .filter(|dev| {
            dev.name()
                .map(|n| n.to_lowercase().contains(&device_name))
                .unwrap_or(false)
        })
        .next()
        .ok_or("No matching device found")?;

    Ok(Some((device, file)))
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
