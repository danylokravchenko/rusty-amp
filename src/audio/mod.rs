use anyhow::{Result, anyhow};
use cpal::{
    Device, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

use crate::dsp::tuner::{Tuner, TunerDetector};
use crate::dsp::{DspChain, Levels, Params, StereoInsert};
use crate::recording::RecordingState;

/// A swappable plugin insert handed to the audio thread (`Some` to install, `None`
/// to clear). Boxed so the audio thread only ever moves a pointer.
type InsertCommand = Option<Box<dyn StereoInsert>>;

/// How many pending insert swaps / disposals the lock-free rings can hold. Swaps
/// are rare (a user loading/clearing a plugin), so a small buffer is plenty.
const INSERT_QUEUE_CAP: usize = 8;

/// Largest block (in frames) the audio thread will ever process at once. Scratch
/// buffers are pre-sized to this, and plugin inserts are activated with it as
/// their maximum block size.
pub const MAX_BLOCK: usize = 4096;

pub struct AudioEngine {
    _input_stream: Stream,
    _output_stream: Stream,
    /// Negotiated sample rate of the running streams (Hz).
    sample_rate: f32,
    /// Sends insert swaps to the audio thread (consumed at the top of its callback).
    insert_tx: Producer<InsertCommand>,
    /// Receives inserts the audio thread displaced, so they are dropped here on a
    /// non-audio thread rather than freed in the realtime callback.
    dropped_rx: Consumer<Box<dyn StereoInsert>>,
}

impl AudioEngine {
    /// The sample rate (Hz) the engine negotiated and is running at.
    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Install (`Some`) or clear (`None`) the third-party plugin insert.
    ///
    /// Call this from the UI/control thread, never the audio thread. The actual
    /// swap happens lock-free inside the audio callback; any previously installed
    /// insert is disposed of here, on the caller's thread.
    pub fn set_plugin_insert(&mut self, insert: InsertCommand) -> Result<()> {
        // Dispose of anything the audio thread has handed back since last time.
        while let Ok(old) = self.dropped_rx.pop() {
            drop(old);
        }
        self.insert_tx
            .push(insert)
            .map_err(|_| anyhow!("plugin-insert command queue is full"))
    }
}

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
    tuner: Arc<Tuner>,
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
        tuner,
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
    tuner: Arc<Tuner>,
) -> Result<AudioEngine> {
    recording.sample_rate.store(sr as u32, Relaxed);

    let buf_samples = (sr as usize) / 5 * out_channels * 2;
    let (mut producer, mut consumer) = RingBuffer::<f32>::new(buf_samples);

    let mut chain = DspChain::new(sr, Arc::clone(&params));

    // Tuner: when engaged, the rig is bypassed and the dry guitar feeds both the
    // output (a clean signal to tune against) and the pitch/spectrum detector.
    let mut tuner_detector = TunerDetector::new(sr);

    // Lock-free handoff for swapping the plugin insert in/out without touching the
    // running stream: commands flow UI → audio, displaced inserts flow back to be
    // dropped off the audio thread.
    let (insert_tx, mut insert_rx) = RingBuffer::<InsertCommand>::new(INSERT_QUEUE_CAP);
    let (mut dropped_tx, dropped_rx) = RingBuffer::<Box<dyn StereoInsert>>::new(INSERT_QUEUE_CAP);

    let attack = 1.0 - (-1.0 / (0.001 * sr)).exp();
    let release = 1.0 - (-1.0 / (0.300 * sr)).exp();
    let mut in_env = 0.0f32;
    let mut out_env = 0.0f32;

    // Reusable scratch buffers for block processing. Pre-sized generously so the
    // audio thread never reallocates for normal device buffer sizes; the `resize`
    // below only grows them on the rare callback that asks for a larger block.
    let mut in_buf: Vec<f32> = Vec::with_capacity(MAX_BLOCK);
    let mut out_l: Vec<f32> = vec![0.0; MAX_BLOCK];
    let mut out_r: Vec<f32> = vec![0.0; MAX_BLOCK];

    let input_stream = input_device.build_input_stream(
        input_cfg,
        move |data: &[f32], _| {
            // Apply any pending insert swaps before processing this block. The old
            // insert is shipped back to the control thread for disposal; if that
            // queue is somehow full we drop it here as a last resort.
            while let Ok(cmd) = insert_rx.pop() {
                if let Some(old) = chain.replace_insert(cmd) {
                    let _ = dropped_tx.push(old);
                }
            }

            let frames = data.len() / in_channels;
            if out_l.len() < frames {
                out_l.resize(frames, 0.0);
                out_r.resize(frames, 0.0);
            }

            // Deinterleave the guitar channel into the mono input block.
            in_buf.clear();
            in_buf.extend(
                data.chunks(in_channels)
                    .map(|frame| frame.get(guitar_ch).copied().unwrap_or(0.0)),
            );

            if tuner.active.load(Relaxed) {
                // Bypass the whole rig: clean dry guitar to both channels, and
                // analyse the same signal for pitch and spectrum.
                tuner_detector.process(&in_buf, &tuner);
                for ((dst_l, dst_r), &x) in
                    out_l.iter_mut().zip(out_r.iter_mut()).zip(in_buf.iter())
                {
                    *dst_l = x;
                    *dst_r = x;
                }
            } else {
                chain.process_block(&in_buf, &mut out_l, &mut out_r);
            }

            for ((&sample, &l), &r) in in_buf.iter().zip(out_l.iter()).zip(out_r.iter()) {
                let a = sample.abs();
                in_env += if a > in_env { attack } else { release } * (a - in_env);

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
        sample_rate: sr,
        insert_tx,
        dropped_rx,
    })
}
