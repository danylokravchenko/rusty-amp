use anyhow::{Result, anyhow};
use cpal::{
    Device, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::RingBuffer;
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::{DspChain, Levels, Params};
use crate::recording::RecordingState;

pub struct AudioEngine {
    _input_stream: Stream,
    _output_stream: Stream,
}

// ── Device discovery ──────────────────────────────────────────────────────────

pub struct InputInfo {
    pub name: String,
    pub channels: usize,
}

pub struct DeviceInfo {
    pub inputs: Vec<InputInfo>,
    pub outputs: Vec<String>,
}

pub fn list_devices() -> Result<DeviceInfo> {
    let host = cpal::default_host();

    let inputs = host
        .input_devices()?
        .enumerate()
        .map(|(i, d)| {
            let name = d
                .description()
                .map(|desc| desc.name().to_owned())
                .unwrap_or_else(|_| format!("device-{i}"));
            let channels = d
                .default_input_config()
                .map(|c| c.channels() as usize)
                .unwrap_or(1);
            InputInfo { name, channels }
        })
        .collect();

    let outputs = host
        .output_devices()?
        .enumerate()
        .map(|(i, d)| {
            d.description()
                .map(|desc| desc.name().to_owned())
                .unwrap_or_else(|_| format!("device-{i}"))
        })
        .collect();

    Ok(DeviceInfo { inputs, outputs })
}

pub fn start(
    input_idx: usize,
    guitar_ch: usize,
    output_idx: usize,
    params: Arc<Params>,
    levels: Arc<Levels>,
    recording: Arc<RecordingState>,
) -> Result<AudioEngine> {
    let host = cpal::default_host();

    let input_device: Device = host
        .input_devices()?
        .nth(input_idx)
        .ok_or_else(|| anyhow!("Input device index {input_idx} not found"))?;

    let output_device: Device = host
        .output_devices()?
        .nth(output_idx)
        .ok_or_else(|| anyhow!("Output device index {output_idx} not found"))?;

    let (input_cfg, output_cfg, sr, _in_fmt) = negotiate_configs(&input_device, &output_device)?;

    let in_channels = input_cfg.channels as usize;
    let out_channels = output_cfg.channels as usize;

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
        recording,
    )
}

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
                "Warning: output does not support {} Hz; falling back to its default.",
                in_sr
            );
            output.default_output_config().unwrap()
        });

    Ok((in_sup.into(), out_sup.into(), in_sr as f32, in_fmt))
}

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
    recording: Arc<RecordingState>,
) -> Result<AudioEngine> {
    recording.sample_rate.store(sr as u32, Relaxed);

    let buf_samples = (sr as usize) / 5 * out_channels * 2;
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(buf_samples);

    let mut chain = DspChain::new(sr, Arc::clone(&params));

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

                let (l, r) = chain.process(sample);
                let mono = 0.5 * (l + r);

                let a = mono.abs();
                out_env += if a > out_env { attack } else { release } * (a - out_env);

                if recording.active.load(Relaxed)
                    && let Ok(mut buf) = recording.buffer.try_lock()
                {
                    // Interleaved stereo (L, R).
                    buf.push(l);
                    buf.push(r);
                }

                // Fan the stereo pair out to the device channels: L→0, R→1,
                // any extra channels get the mono sum; a mono device gets the sum.
                for ch in 0..out_channels {
                    let s = if out_channels == 1 {
                        mono
                    } else {
                        match ch {
                            0 => l,
                            1 => r,
                            _ => mono,
                        }
                    };
                    let _ = producer.push(s);
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
