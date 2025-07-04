use clap::Parser;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use ctrlc;
use hound;
use std::error::Error;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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

    let unique_file = make_unique_filename(&file);

    println!("Selected device: {}", device.name()?);
    println!("Output file: {}", unique_file);

    record_audio(device, unique_file)?;

    Ok(())
}

fn record_audio(device: cpal::Device, filename: String) -> Result<(), Box<dyn Error>> {
    let config = device.default_input_config()?;
    let stream_config: cpal::StreamConfig = config.clone().into();

    if config.sample_format() != SampleFormat::F32 {
        return Err("This recorder supports only f32 sample format.".into());
    }

    let spec = hound::WavSpec {
        channels: config.channels(),
        sample_rate: config.sample_rate().0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let writer = Arc::new(std::sync::Mutex::new(Some(hound::WavWriter::create(
        &filename, spec,
    )?)));

    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
        })?;
    }

    println!(
        "Recording... (Ctrl+C to stop)\nchannels: {}, sample_rate: {}, format: {:?}",
        config.channels(),
        config.sample_rate().0,
        config.sample_format()
    );

    let writer_clone = writer.clone();

    let stream = device.build_input_stream(
        &stream_config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            if let Some(ref mut w) = *writer_clone.lock().unwrap() {
                for &sample in data {
                    let clamped = sample.max(-1.0).min(1.0);
                    let i16_sample = (clamped * i16::MAX as f32) as i16;
                    let _ = w.write_sample(i16_sample);
                }
            }
        },
        |err| eprintln!("Stream error: {:?}", err),
        None,
    )?;

    stream.play()?;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Stopping and saving...");
    let mut guard = writer.lock().unwrap();
    if let Some(wav_writer) = guard.take() {
        wav_writer.finalize()?;
    }
    println!("Saved: {}", filename);

    Ok(())
}

fn make_unique_filename(filename: &str) -> String {
    let path = Path::new(filename);
    if !path.exists() {
        return filename.to_string();
    }
    let stem = path.file_stem().unwrap().to_string_lossy();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("wav");
    let mut i = 1;
    loop {
        let candidate = format!("{}_{}.{}", stem, i, ext);
        if !Path::new(&candidate).exists() {
            return candidate;
        }
        i += 1;
    }
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
