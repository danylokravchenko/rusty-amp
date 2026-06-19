use anyhow::{Result, anyhow};
use cpal::{
    Device, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::RingBuffer;
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{DspChain, Levels, Params};

pub struct AudioEngine {
    _input_stream: Stream,
    _output_stream: Stream,
}

pub fn start(params: Arc<Params>, levels: Arc<Levels>) -> Result<AudioEngine> {
    let host = cpal::default_host();

    // ── 1. Select input device ────────────────────────────────────────────────
    let (input_device, chosen_name) = select_input_device(&host)?;

    // ── 2. Select output device ───────────────────────────────────────────────
    let (output_device, _out_name) = select_output_device(&host)?;

    // ── 3. Negotiate matching sample rates ────────────────────────────────────
    let (input_cfg, output_cfg, sr, in_fmt) = negotiate_configs(&input_device, &output_device)?;

    let in_channels = input_cfg.channels as usize;
    let out_channels = output_cfg.channels as usize;

    println!("Input  : {chosen_name}");
    println!(
        "         {} Hz  {} ch  {:?}",
        sr as u32, in_channels, in_fmt
    );
    println!("Output : {}", output_device.description()?.name());
    println!(
        "         {} Hz  {} ch",
        output_cfg.sample_rate, out_channels
    );

    // ── 4. Ask input/output channel mapping ──────────────────────────────────
    let guitar_ch = ask_channel("guitar input", in_channels)?;
    println!("  Reading channel {} of {}", guitar_ch + 1, in_channels);
    println!("  Writing  all {} output channels\n", out_channels);

    // ── 5. Build streams ──────────────────────────────────────────────────────
    build_engine(
        input_device,
        input_cfg,
        in_channels,
        guitar_ch,
        output_device,
        output_cfg,
        out_channels,
        sr,
        params,
        levels,
    )
}

// ── Device / config helpers ───────────────────────────────────────────────────

fn select_input_device(host: &cpal::Host) -> Result<(Device, String)> {
    let inputs: Vec<(usize, String, Device)> = host
        .input_devices()?
        .enumerate()
        .map(|(i, d)| {
            let name = d
                .description()
                .map(|desc| desc.name().to_owned())
                .unwrap_or_else(|_| format!("device-{i}"));
            (i, name, d)
        })
        .collect();

    if inputs.is_empty() {
        return Err(anyhow!("No audio input devices found"));
    }

    println!("\nAvailable input devices:");
    for (i, name, _) in &inputs {
        println!("  [{i}] {name}");
    }
    print!("Select input [0]: ");

    use std::io::{self, Write};
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let idx: usize = line.trim().parse().unwrap_or(0);

    inputs
        .into_iter()
        .nth(idx)
        .map(|(_, name, dev)| (dev, name))
        .ok_or_else(|| anyhow!("Invalid device selection"))
}

fn select_output_device(host: &cpal::Host) -> Result<(Device, String)> {
    let outputs: Vec<(usize, String, Device)> = host
        .output_devices()?
        .enumerate()
        .map(|(i, d)| {
            let name = d
                .description()
                .map(|desc| desc.name().to_owned())
                .unwrap_or_else(|_| format!("device-{i}"));
            (i, name, d)
        })
        .collect();

    if outputs.is_empty() {
        return Err(anyhow!("No audio output devices found"));
    }

    println!("\nAvailable output devices:");
    for (i, name, _) in &outputs {
        println!("  [{i}] {name}");
    }
    print!("Select output [0]: ");

    use std::io::{self, Write};
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let idx: usize = line.trim().parse().unwrap_or(0);

    outputs
        .into_iter()
        .nth(idx)
        .map(|(_, name, dev)| (dev, name))
        .ok_or_else(|| anyhow!("Invalid device selection"))
}

/// Returns `(input_cfg, output_cfg, sample_rate_hz, input_format)`.
/// Forces the output to the same sample rate as the input so the ring buffer
/// never overflows or underflows.
fn negotiate_configs(
    input: &Device,
    output: &Device,
) -> Result<(StreamConfig, StreamConfig, f32, cpal::SampleFormat)> {
    let in_sup = input.default_input_config()?;
    let in_sr = in_sup.sample_rate();
    let in_fmt = in_sup.sample_format();

    let out_sup = output
        .supported_output_configs()?
        .find(|r| r.min_sample_rate() <= in_sr && r.max_sample_rate() >= in_sr)
        .map(|r| r.with_sample_rate(in_sr))
        .unwrap_or_else(|| {
            eprintln!(
                "Warning: output does not support {} Hz; falling back to its default \
                 — audio may stutter.",
                in_sr
            );
            output.default_output_config().unwrap()
        });

    Ok((in_sup.into(), out_sup.into(), in_sr as f32, in_fmt))
}

fn ask_channel(label: &str, n_channels: usize) -> Result<usize> {
    use std::io::{self, Write};
    print!("Select {label} channel (1-{n_channels}) [1]: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let ch = line
        .trim()
        .parse::<usize>()
        .map(|n| n.saturating_sub(1)) // user types 1-based
        .unwrap_or(0)
        .min(n_channels - 1);
    Ok(ch)
}

// ── Stream construction ───────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_engine(
    input_device: Device,
    input_cfg: StreamConfig,
    in_channels: usize,
    guitar_ch: usize,
    output_device: Device,
    output_cfg: StreamConfig,
    out_channels: usize,
    sr: f32,
    params: Arc<Params>,
    levels: Arc<Levels>,
) -> Result<AudioEngine> {
    let buf_samples = (sr as usize) / 5 * out_channels * 2;
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(buf_samples);

    let mut chain = DspChain::new(sr, Arc::clone(&params));

    // Envelope: fast attack (~1 ms), slow release (~300 ms)
    let attack = 1.0 - (-1.0 / (0.001 * sr)).exp();
    let release = 1.0 - (-1.0 / (0.300 * sr)).exp();
    let mut in_env = 0.0f32;
    let mut out_env = 0.0f32;

    let input_stream = input_device.build_input_stream(
        input_cfg,
        move |data: &[f32], _| {
            for frame in data.chunks(in_channels) {
                let sample = frame.get(guitar_ch).copied().unwrap_or(0.0);

                let a = sample.abs();
                in_env += if a > in_env { attack } else { release } * (a - in_env);

                let processed = chain.process(sample);

                let a = processed.abs();
                out_env += if a > out_env { attack } else { release } * (a - out_env);

                for _ in 0..out_channels {
                    let _ = producer.push(processed);
                }
            }
            levels.input.store(in_env, Relaxed);
            levels.output.store(out_env, Relaxed);
        },
        |e| eprintln!("input error: {e}"),
        None,
    )?;

    let output_stream = output_device.build_output_stream(
        output_cfg,
        move |data: &mut [f32], _| {
            for s in data.iter_mut() {
                *s = consumer.pop().unwrap_or(0.0);
            }
        },
        |e| eprintln!("output error: {e}"),
        None,
    )?;

    input_stream.play()?;
    output_stream.play()?;

    Ok(AudioEngine {
        _input_stream: input_stream,
        _output_stream: output_stream,
    })
}
