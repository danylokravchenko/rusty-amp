//! CLAP plugin hosting — load third-party CLAP effect plugins as a stereo insert.
//!
//! This is intentionally a *minimal effect host*: it discovers CLAP bundles in the
//! standard install locations, loads one, and bridges its main stereo (or mono)
//! audio ports to the chain's [`StereoInsert`] slot. It is headless — plugin GUIs
//! are not opened; parameters are meant to be driven from the TUI (a later step).
//!
//! Threading model: the [`PluginInstance`] main-thread handle is `!Send` and stays
//! on whichever (UI/control) thread called [`load`], kept alive inside
//! [`LoadedPlugin`]. The audio processor is `Send` and `Arc`-backed, so the
//! [`ClapInsert`] that wraps it is handed to the audio thread independently.

// Loading a plugin binary is inherently unsafe FFI; confine the lint allowance to
// this module rather than sprinkling per-call attributes.
#![allow(unsafe_code)]
// clack's prelude is the intended one-stop import surface for host code.
#![allow(clippy::wildcard_imports)]

use std::ffi::CString;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use clack_extensions::audio_ports::{AudioPortInfoBuffer, AudioPortType, PluginAudioPorts};
use clack_extensions::params::{
    HostParams, HostParamsImplMainThread, HostParamsImplShared, ParamClearFlags, ParamInfoBuffer,
    ParamRescanFlags, PluginParams,
};
use clack_host::events::event_types::ParamValueEvent;
use clack_host::prelude::*;
use clack_host::process::StartedPluginAudioProcessor;
use clack_host::utils::{ClapId, Cookie};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::dsp::StereoInsert;

/// How many pending parameter changes the UI→audio ring can hold before it drops
/// the oldest (the next push always reflects the latest knob position anyway).
const PARAM_QUEUE_CAP: usize = 256;

// ── Host handlers ─────────────────────────────────────────────────────────────
//
// We need almost nothing from the host side for a headless effect: no GUI, no
// timers, no parameter callbacks. These are the minimal trait impls clack requires.

struct RaShared;

struct RaMainThread<'a> {
    _shared: &'a RaShared,
}

impl<'a> SharedHandler<'a> for RaShared {
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {}
}

impl<'a> MainThreadHandler<'a> for RaMainThread<'a> {
    fn initialized(&mut self, _instance: InitializedPluginHandle<'a>) {}
}

// Params extension host callbacks. We drive parameters one-way (UI → plugin) and
// don't track the plugin's own value changes, so these are intentionally no-ops —
// but the plugin needs the extension declared to accept our automation events.
impl HostParamsImplShared for RaShared {
    fn request_flush(&self) {}
}

impl HostParamsImplMainThread for RaMainThread<'_> {
    fn rescan(&mut self, _flags: ParamRescanFlags) {}
    fn clear(&mut self, _param_id: ClapId, _flags: ParamClearFlags) {}
}

struct RaHost;

impl HostHandlers for RaHost {
    type Shared<'a> = RaShared;
    type MainThread<'a> = RaMainThread<'a>;
    type AudioProcessor<'a> = ();

    fn declare_extensions(builder: &mut HostExtensions<Self>, _shared: &Self::Shared<'_>) {
        builder.register::<HostParams>();
    }
}

/// A single automatable parameter the UI sends to the plugin.
#[derive(Clone, Copy)]
struct ParamChange {
    id: ClapId,
    value: f64,
}

/// A plugin parameter, as surfaced to the UI for display and editing.
#[derive(Clone)]
pub struct PluginParam {
    /// CLAP parameter id (used to address it when sending changes).
    id: ClapId,
    /// Display name.
    pub name: String,
    /// Minimum plain value.
    pub min: f64,
    /// Maximum plain value.
    pub max: f64,
    /// Current value (cached on the UI side; updated as the user edits).
    pub value: f64,
}

fn host_info() -> Result<HostInfo> {
    HostInfo::new(
        "rusty-amp",
        "rusty-amp",
        "https://github.com/danylokravchenko/rusty-amp",
        env!("CARGO_PKG_VERSION"),
    )
    .map_err(|e| anyhow!("invalid host info: {e}"))
}

/// A CLAP plugin found on disk, before it is loaded.
#[derive(Clone, Debug)]
pub struct DiscoveredPlugin {
    /// Path to the `.clap` bundle/file it lives in.
    pub path: PathBuf,
    /// The plugin's unique CLAP id.
    pub id: String,
    /// Human-friendly display name (falls back to the id).
    pub name: String,
}

/// Scan the standard CLAP install locations and return every plugin found.
///
/// Best-effort: unreadable directories and files that fail to load are skipped
/// rather than erroring the whole scan.
pub fn scan() -> Vec<DiscoveredPlugin> {
    let mut files = Vec::new();
    for dir in standard_clap_paths() {
        find_clap_files(&dir, &mut files);
    }
    files.sort();
    files.dedup();

    let mut plugins = Vec::new();
    for path in &files {
        if let Ok(found) = descriptors_in(path) {
            plugins.extend(found);
        }
    }
    plugins
}

/// Reads the plugin descriptors exposed by a single `.clap` bundle.
fn descriptors_in(path: &Path) -> Result<Vec<DiscoveredPlugin>> {
    let entry =
        unsafe { PluginEntry::load(path) }.map_err(|e| anyhow!("load {}: {e}", path.display()))?;
    let factory = entry
        .get_plugin_factory()
        .ok_or_else(|| anyhow!("{} has no plugin factory", path.display()))?;

    let mut out = Vec::new();
    for descriptor in factory.plugin_descriptors() {
        let Some(id) = descriptor.id().and_then(|s| s.to_str().ok()) else {
            continue;
        };
        let name = descriptor
            .name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| id.to_owned());
        out.push(DiscoveredPlugin {
            path: path.to_path_buf(),
            id: id.to_owned(),
            name,
        });
    }
    Ok(out)
}

/// The directories CLAP hosts are expected to search, plus anything in `CLAP_PATH`.
fn standard_clap_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(env_path) = std::env::var("CLAP_PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        paths.extend(
            env_path
                .split(sep)
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
        );
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Library/Audio/Plug-Ins/CLAP"));
        }
        paths.push(PathBuf::from("/Library/Audio/Plug-Ins/CLAP"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(common) = std::env::var("CommonProgramFiles") {
            paths.push(PathBuf::from(common).join("CLAP"));
        }
        if let Some(local) = dirs::data_local_dir() {
            paths.push(local.join("Programs/Common/CLAP"));
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".clap"));
        }
        paths.push(PathBuf::from("/usr/lib/clap"));
        paths.push(PathBuf::from("/usr/local/lib/clap"));
    }

    paths
}

/// Recursively collect `*.clap` entries under `dir`. A `.clap` is added as-is (on
/// macOS it is a bundle directory, which [`PluginEntry::load`] resolves); other
/// directories are descended into.
fn find_clap_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("clap") {
            out.push(path);
        } else if path.is_dir() {
            find_clap_files(&path, out);
        }
    }
}

/// A loaded plugin's main-thread handle, kept alive for as long as the plugin is
/// in use. Holding this keeps the underlying entry/instance loaded; dropping it
/// unloads the plugin (drop the [`StereoInsert`] on the audio side first).
pub struct LoadedPlugin {
    /// Display name of the loaded plugin.
    pub name: String,
    /// CLAP id of the loaded plugin.
    pub id: String,
    /// The plugin's automatable parameters (for display and editing).
    params: Vec<PluginParam>,
    /// Sends parameter changes to the audio-thread insert.
    param_tx: Producer<ParamChange>,
    // Kept purely to own the plugin's lifetime on this (non-audio) thread.
    #[allow(dead_code)]
    entry: PluginEntry,
    #[allow(dead_code)]
    instance: PluginInstance<RaHost>,
}

impl LoadedPlugin {
    /// The plugin's parameters, in discovery order.
    pub fn params(&self) -> &[PluginParam] {
        &self.params
    }

    /// Set parameter `index` to `value` (clamped to its range), updating the cached
    /// value and queueing the change for the audio thread. A full queue is ignored:
    /// the next change carries the latest value anyway.
    pub fn set_param(&mut self, index: usize, value: f64) {
        let Some(param) = self.params.get_mut(index) else {
            return;
        };
        let value = value.clamp(param.min, param.max);
        param.value = value;
        let _ = self.param_tx.push(ParamChange {
            id: param.id,
            value,
        });
    }
}

/// Load `plugin`, activate it for the given audio config, and return both the
/// main-thread handle ([`LoadedPlugin`]) and the audio-thread [`StereoInsert`].
///
/// `max_block` is the largest block (in frames) the audio thread will ever ask the
/// insert to process; the plugin is activated with this as its maximum.
pub fn load(
    plugin: &DiscoveredPlugin,
    sample_rate: f32,
    max_block: u32,
) -> Result<(LoadedPlugin, Box<dyn StereoInsert>)> {
    let entry = unsafe { PluginEntry::load(&plugin.path) }
        .map_err(|e| anyhow!("load {}: {e}", plugin.path.display()))?;

    let plugin_id = CString::new(plugin.id.as_str())?;
    let mut instance = PluginInstance::<RaHost>::new(
        |_| RaShared,
        |shared| RaMainThread { _shared: shared },
        &entry,
        &plugin_id,
        &host_info()?,
    )
    .map_err(|e| anyhow!("instantiate {}: {e}", plugin.id))?;

    let in_ch = main_port_channels(&mut instance, true);
    let out_ch = main_port_channels(&mut instance, false);
    let params = query_params(&mut instance);

    let config = PluginAudioConfiguration {
        sample_rate: f64::from(sample_rate),
        min_frames_count: 1,
        max_frames_count: max_block,
    };

    let processor = instance
        .activate(|_, _| (), config)
        .map_err(|e| anyhow!("activate {}: {e}", plugin.id))?
        .start_processing()
        .map_err(|e| anyhow!("start processing {}: {e}", plugin.id))?;

    let (param_tx, param_rx) = RingBuffer::<ParamChange>::new(PARAM_QUEUE_CAP);

    let insert = ClapInsert::new(
        processor,
        in_ch,
        out_ch,
        max_block as usize,
        param_rx,
        params.len(),
    );

    let loaded = LoadedPlugin {
        name: plugin.name.clone(),
        id: plugin.id.clone(),
        params,
        param_tx,
        entry,
        instance,
    };

    Ok((loaded, Box::new(insert)))
}

/// Read the plugin's parameter list (id, name, range, current value) via the
/// params extension. Returns empty if the plugin doesn't implement params.
fn query_params(instance: &mut PluginInstance<RaHost>) -> Vec<PluginParam> {
    let mut handle = instance.plugin_handle();
    let Some(params) = handle.get_extension::<PluginParams>() else {
        return Vec::new();
    };

    let mut info_buf = ParamInfoBuffer::new();
    let mut out = Vec::new();
    for i in 0..params.count(&mut handle) {
        let Some(info) = params.get_info(&mut handle, i, &mut info_buf) else {
            continue;
        };
        let value = params
            .get_value(&mut handle, info.id)
            .unwrap_or(info.default_value);
        out.push(PluginParam {
            id: info.id,
            name: String::from_utf8_lossy(info.name).into_owned(),
            min: info.min_value,
            max: info.max_value,
            value,
        });
    }
    out
}

/// Returns the channel count (clamped to 1 or 2) of the plugin's first/main audio
/// port on the requested side, defaulting to stereo if the extension is absent.
fn main_port_channels(instance: &mut PluginInstance<RaHost>, is_input: bool) -> usize {
    let mut handle = instance.plugin_handle();
    let Some(ports) = handle.get_extension::<PluginAudioPorts>() else {
        return 2;
    };

    let mut buffer = AudioPortInfoBuffer::new();
    for i in 0..ports.count(&mut handle, is_input) {
        let Some(info) = ports.get(&mut handle, i, is_input, &mut buffer) else {
            continue;
        };
        let port_type = info
            .port_type
            .or_else(|| AudioPortType::from_channel_count(info.channel_count));
        return match port_type {
            Some(t) if t == AudioPortType::MONO => 1,
            Some(t) if t == AudioPortType::STEREO => 2,
            _ => (info.channel_count.clamp(1, 2)) as usize,
        };
    }
    2
}

/// Bridges a started CLAP audio processor to the chain's [`StereoInsert`] slot.
///
/// Channel buffers are laid out channel-major with a fixed stride of `max_block`,
/// allocated once up front so processing never allocates on the audio thread.
struct ClapInsert {
    processor: StartedPluginAudioProcessor<RaHost>,
    in_ports: AudioPorts,
    out_ports: AudioPorts,
    /// `in_ch * max_block` samples, channel-major (stride `max_block`).
    in_buf: Vec<f32>,
    /// `out_ch * max_block` samples, channel-major (stride `max_block`).
    out_buf: Vec<f32>,
    in_ch: usize,
    out_ch: usize,
    max_block: usize,
    /// Steady-time frame counter handed to the plugin's `process`.
    steady: u64,
    /// Parameter changes pushed by the UI thread.
    param_rx: Consumer<ParamChange>,
    /// Reused buffer the drained param changes are turned into for `process`.
    events: EventBuffer,
    /// Scratch used to coalesce queued changes to one (latest) value per param,
    /// pre-sized to the parameter count so it never allocates on the audio thread.
    coalesced: Vec<ParamChange>,
}

impl ClapInsert {
    fn new(
        processor: StartedPluginAudioProcessor<RaHost>,
        in_ch: usize,
        out_ch: usize,
        max_block: usize,
        param_rx: Consumer<ParamChange>,
        n_params: usize,
    ) -> Self {
        Self {
            processor,
            in_ports: AudioPorts::with_capacity(in_ch, 1),
            out_ports: AudioPorts::with_capacity(out_ch, 1),
            in_buf: vec![0.0; in_ch * max_block],
            out_buf: vec![0.0; out_ch * max_block],
            in_ch,
            out_ch,
            max_block,
            steady: 0,
            param_rx,
            events: EventBuffer::with_capacity(PARAM_QUEUE_CAP),
            coalesced: Vec::with_capacity(n_params),
        }
    }

    /// Process a single chunk no larger than `max_block`. `left`/`right` are equal
    /// length and are read for input and overwritten with the plugin's output.
    fn process_chunk(&mut self, left: &mut [f32], right: &mut [f32]) {
        let m = left.len();
        let stride = self.max_block;

        // Deinterleave our stereo pair into the plugin's input channel buffers.
        if self.in_ch >= 2 {
            self.in_buf[..m].copy_from_slice(left);
            self.in_buf[stride..stride + m].copy_from_slice(right);
        } else {
            for (dst, (l, r)) in self.in_buf[..m]
                .iter_mut()
                .zip(left.iter().zip(right.iter()))
            {
                *dst = 0.5 * (*l + *r);
            }
        }

        // Drain pending parameter changes, coalescing to the latest value per param
        // so the plugin recomputes each parameter at most once this block (a fast
        // knob sweep that queued many changes can't pile up N recomputes here).
        self.coalesced.clear();
        while let Ok(change) = self.param_rx.pop() {
            if let Some(slot) = self.coalesced.iter_mut().find(|c| c.id == change.id) {
                slot.value = change.value;
            } else {
                self.coalesced.push(change);
            }
        }
        self.events.clear();
        for change in &self.coalesced {
            self.events.push(&ParamValueEvent::new(
                0,
                change.id,
                Pckn::match_all(),
                change.value,
                Cookie::empty(),
            ));
        }

        {
            let ins = self.in_ports.with_input_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_input_only(
                    self.in_buf.chunks_exact_mut(stride).map(|c| InputChannel {
                        buffer: &mut c[..m],
                        is_constant: false,
                    }),
                ),
            }]);
            let mut outs = self.out_ports.with_output_buffers([AudioPortBuffer {
                latency: 0,
                channels: AudioPortBufferType::f32_output_only(
                    self.out_buf.chunks_exact_mut(stride).map(|c| &mut c[..m]),
                ),
            }]);

            // If the plugin errors, leave out_buf as-is (it was zero-initialised /
            // holds the previous block); we still advance steady time below.
            let _ = self.processor.process(
                &ins,
                &mut outs,
                &self.events.as_input(),
                &mut OutputEvents::void(),
                Some(self.steady),
                None,
            );
        }

        self.steady += m as u64;

        // Re-interleave the plugin's output back into our stereo pair.
        if self.out_ch >= 2 {
            left.copy_from_slice(&self.out_buf[..m]);
            right.copy_from_slice(&self.out_buf[stride..stride + m]);
        } else {
            left.copy_from_slice(&self.out_buf[..m]);
            right.copy_from_slice(&self.out_buf[..m]);
        }
    }
}

impl StereoInsert for ClapInsert {
    fn process_block(&mut self, left: &mut [f32], right: &mut [f32]) {
        // The host block may exceed the plugin's activated maximum; split it.
        let len = left.len();
        let mut start = 0;
        while start < len {
            let end = (start + self.max_block).min(len);
            self.process_chunk(&mut left[start..end], &mut right[start..end]);
            start = end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole handoff design depends on the insert being `Send` so it can be
    /// moved to the audio thread. This won't compile if that ever stops holding.
    #[test]
    fn clap_insert_is_send() {
        const fn assert_send<T: Send>() {}
        assert_send::<ClapInsert>();
        assert_send::<Box<dyn StereoInsert>>();
    }
}
