//! Audio Unit hosting (macOS) — load a third-party AU effect and drive it headless
//! as an *amp-position override* in the signal chain.
//!
//! Like the CLAP [`super`] host this is a minimal, GUI-less effect host: it
//! enumerates installed Audio Units, instantiates one, bridges its main stereo
//! audio to the chain's [`StereoInsert`] slot, and exposes its parameters for the
//! TUI to drive. It talks to the AudioComponent v2 C API directly (via
//! `coreaudio-sys`) — a pure-C surface, so no Objective-C runtime is needed on the
//! render path.
//!
//! Threading model: instantiation, activation and parameter discovery all happen on
//! the (UI/control) thread inside [`load`]. The resulting [`AuInsert`] owns the
//! `AudioUnit` instance and is the only thing handed to the audio thread — it is
//! `Send` because the instance is touched from exactly one thread once installed.
//! The `AudioUnit` is torn down in `AuInsert`'s `Drop`; the engine disposes displaced
//! inserts on the UI thread, so teardown never runs in the realtime callback.

// Hosting a plugin is inherently unsafe FFI; confine the allowance to this module.
#![allow(unsafe_code)]
// `coreaudio-sys` is a bindgen surface; the flat glob is the intended import.
#![allow(clippy::wildcard_imports)]

use std::ffi::c_void;
use std::os::raw::c_char;
use std::ptr;

use anyhow::{Result, anyhow};
use coreaudio_sys::*;
use rtrb::{Consumer, Producer, RingBuffer};

use crate::dsp::StereoInsert;

/// How many pending parameter changes the UI→audio ring can hold before it drops the
/// oldest (the next push always carries the latest knob position anyway).
const PARAM_QUEUE_CAP: usize = 256;

/// We always bridge a stereo (2-channel) non-interleaved float stream to the AU.
const CHANNELS: usize = 2;

/// A single automatable parameter change the UI sends to the plugin.
#[derive(Clone, Copy)]
struct AuParamChange {
    id: AudioUnitParameterID,
    value: f32,
}

/// A plugin parameter, as surfaced to the UI for display and editing.
#[derive(Clone)]
pub struct AuParam {
    /// AU parameter id (global scope).
    id: AudioUnitParameterID,
    /// Display name.
    pub name: String,
    /// Minimum plain value.
    pub min: f64,
    /// Maximum plain value.
    pub max: f64,
    /// Current value (cached UI-side; updated as the user edits).
    pub value: f64,
}

/// An Audio Unit found on disk, before it is loaded. The `(type, subtype,
/// manufacturer)` 4CC triple uniquely re-identifies the component for [`load`].
#[derive(Clone, Debug)]
pub struct DiscoveredAu {
    /// Human-friendly display name (usually "Manufacturer: Effect").
    pub name: String,
    type_: u32,
    subtype: u32,
    manufacturer: u32,
}

/// A loaded AU's UI-side handle: display name, parameters, and the channel used to
/// push parameter edits to the audio-thread insert. Unlike the CLAP host this does
/// *not* own the plugin instance (the [`AuInsert`] does), so dropping it does not
/// unload the plugin.
pub struct LoadedAu {
    /// Display name of the loaded plugin.
    pub name: String,
    params: Vec<AuParam>,
    param_tx: Producer<AuParamChange>,
}

impl LoadedAu {
    /// The plugin's parameters, in discovery order.
    pub fn params(&self) -> &[AuParam] {
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
        let _ = self.param_tx.push(AuParamChange {
            id: param.id,
            value: value as f32,
        });
    }
}

/// Convert a `CFStringRef` to an owned `String` (UTF-8). Returns `None` on null or a
/// conversion failure. Does **not** release the string (ownership stays with caller).
fn cfstring_to_string(s: CFStringRef) -> Option<String> {
    if s.is_null() {
        return None;
    }
    unsafe {
        // Fast path: a direct UTF-8 pointer if CoreFoundation has one.
        let ptr = CFStringGetCStringPtr(s, kCFStringEncodingUTF8);
        if !ptr.is_null() {
            return std::ffi::CStr::from_ptr(ptr).to_str().ok().map(str::to_owned);
        }
        // Slow path: copy into a buffer sized from the length (× 4 for worst-case UTF-8).
        let len = CFStringGetLength(s);
        if len <= 0 {
            return Some(String::new());
        }
        let cap = (len as usize) * 4 + 1;
        let mut buf = vec![0_i8; cap];
        let ok = CFStringGetCString(
            s,
            buf.as_mut_ptr().cast::<c_char>(),
            cap as CFIndex,
            kCFStringEncodingUTF8,
        );
        if ok == 0 {
            return None;
        }
        let bytes = buf.as_ptr().cast::<c_char>();
        std::ffi::CStr::from_ptr(bytes)
            .to_str()
            .ok()
            .map(str::to_owned)
    }
}

/// Enumerate installed effect Audio Units (`aufx` + `aumf`), returning what we can
/// name and re-identify. Best-effort: components we can't read a name for are skipped.
pub fn scan() -> Vec<DiscoveredAu> {
    let mut out = Vec::new();
    for type_ in [kAudioUnitType_Effect, kAudioUnitType_MusicEffect] {
        let desc = AudioComponentDescription {
            componentType: type_,
            componentSubType: 0,
            componentManufacturer: 0,
            componentFlags: 0,
            componentFlagsMask: 0,
        };
        let mut comp: AudioComponent = ptr::null_mut();
        loop {
            comp = unsafe { AudioComponentFindNext(comp, &desc) };
            if comp.is_null() {
                break;
            }
            let mut found = AudioComponentDescription {
                componentType: 0,
                componentSubType: 0,
                componentManufacturer: 0,
                componentFlags: 0,
                componentFlagsMask: 0,
            };
            if unsafe { AudioComponentGetDescription(comp, &mut found) } != 0 {
                continue;
            }
            let mut name_ref: CFStringRef = ptr::null();
            let name = if unsafe { AudioComponentCopyName(comp, &mut name_ref) } == 0 {
                let n = cfstring_to_string(name_ref);
                if !name_ref.is_null() {
                    unsafe { CFRelease(name_ref.cast()) };
                }
                n
            } else {
                None
            };
            let Some(name) = name else { continue };
            out.push(DiscoveredAu {
                name,
                type_: found.componentType,
                subtype: found.componentSubType,
                manufacturer: found.componentManufacturer,
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Non-interleaved 32-bit float stream description for `sample_rate`, `CHANNELS` wide.
fn stream_format(sample_rate: f32) -> AudioStreamBasicDescription {
    AudioStreamBasicDescription {
        mSampleRate: f64::from(sample_rate),
        mFormatID: kAudioFormatLinearPCM,
        // Float | Packed | NonInterleaved.
        mFormatFlags: kAudioFormatFlagIsFloat
            | kAudioFormatFlagIsPacked
            | kAudioFormatFlagIsNonInterleaved,
        // For non-interleaved audio these count a *single* channel's frame.
        mBytesPerPacket: 4,
        mFramesPerPacket: 1,
        mBytesPerFrame: 4,
        mChannelsPerFrame: CHANNELS as u32,
        mBitsPerChannel: 32,
        mReserved: 0,
    }
}

/// Load `au`, activate it for the given audio config, and return both the UI-side
/// handle ([`LoadedAu`]) and the audio-thread [`StereoInsert`].
///
/// `max_block` is the largest block (in frames) the audio thread will ever ask the
/// insert to process; the AU is configured with this as its maximum slice.
pub fn load(au: &DiscoveredAu, sample_rate: f32, max_block: u32) -> Result<(LoadedAu, Box<dyn StereoInsert>)> {
    let desc = AudioComponentDescription {
        componentType: au.type_,
        componentSubType: au.subtype,
        componentManufacturer: au.manufacturer,
        componentFlags: 0,
        componentFlagsMask: 0,
    };
    let comp = unsafe { AudioComponentFindNext(ptr::null_mut(), &desc) };
    if comp.is_null() {
        return Err(anyhow!("audio unit '{}' not found", au.name));
    }

    let mut unit: AudioUnit = ptr::null_mut();
    check(unsafe { AudioComponentInstanceNew(comp, &mut unit) }, "instantiate")?;
    // From here on, any early return must dispose the instance.
    let guard = InstanceGuard(unit);

    // Maximum slice, then the stream format on both scopes, then the input callback —
    // all before AudioUnitInitialize, as the AU API requires.
    let max = max_block;
    set_prop(unit, kAudioUnitProperty_MaximumFramesPerSlice, kAudioUnitScope_Global, 0, &max)?;

    let fmt = stream_format(sample_rate);
    set_prop(unit, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Input, 0, &fmt)?;
    set_prop(unit, kAudioUnitProperty_StreamFormat, kAudioUnitScope_Output, 0, &fmt)?;

    // The render context lives on the heap so its address is stable once captured by
    // the AU as the callback's refcon, regardless of where the AuInsert is moved.
    let mut ctx = Box::new(RenderCtx {
        in_ptr: ptr::null(),
        frames: 0,
        stride: max_block as usize,
    });
    let cb = AURenderCallbackStruct {
        inputProc: Some(input_render_cb),
        inputProcRefCon: (&mut *ctx as *mut RenderCtx).cast::<c_void>(),
    };
    set_prop(unit, kAudioUnitProperty_SetRenderCallback, kAudioUnitScope_Input, 0, &cb)?;

    check(unsafe { AudioUnitInitialize(unit) }, "initialize")?;

    let params = query_params(unit);

    // Everything succeeded: hand the live instance to the insert and defuse the guard.
    guard.defuse();
    let insert = AuInsert::new(unit, ctx, max_block as usize);

    let (param_tx, param_rx) = RingBuffer::<AuParamChange>::new(PARAM_QUEUE_CAP);
    let insert = insert.with_params(param_rx);

    let loaded = LoadedAu {
        name: au.name.clone(),
        params,
        param_tx,
    };
    Ok((loaded, Box::new(insert)))
}

/// Read the AU's global-scope parameter list (id, name, range, current value).
/// Returns empty if the AU exposes none. Best-effort per parameter.
fn query_params(unit: AudioUnit) -> Vec<AuParam> {
    let mut size: u32 = 0;
    // First call with a null buffer just reports the byte size of the id array.
    let st = unsafe {
        AudioUnitGetProperty(
            unit,
            kAudioUnitProperty_ParameterList,
            kAudioUnitScope_Global,
            0,
            ptr::null_mut(),
            &mut size,
        )
    };
    if st != 0 || size == 0 {
        return Vec::new();
    }
    let count = size as usize / std::mem::size_of::<AudioUnitParameterID>();
    let mut ids = vec![0_u32; count];
    let st = unsafe {
        AudioUnitGetProperty(
            unit,
            kAudioUnitProperty_ParameterList,
            kAudioUnitScope_Global,
            0,
            ids.as_mut_ptr().cast::<c_void>(),
            &mut size,
        )
    };
    if st != 0 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(count);
    for id in ids {
        let mut info: AudioUnitParameterInfo = unsafe { std::mem::zeroed() };
        let mut isize = std::mem::size_of::<AudioUnitParameterInfo>() as u32;
        let st = unsafe {
            AudioUnitGetProperty(
                unit,
                kAudioUnitProperty_ParameterInfo,
                kAudioUnitScope_Global,
                id,
                (&mut info as *mut AudioUnitParameterInfo).cast::<c_void>(),
                &mut isize,
            )
        };
        if st != 0 {
            continue;
        }

        // Prefer the CFString name; fall back to the fixed-size C name field.
        let name = cfstring_to_string(info.cfNameString)
            .filter(|s| !s.is_empty())
            .or_else(|| c_name(&info.name))
            .unwrap_or_else(|| format!("param {id}"));
        // If the AU asked us to, release the CFString it handed back.
        if !info.cfNameString.is_null()
            && info.flags & kAudioUnitParameterFlag_CFNameRelease != 0
        {
            unsafe { CFRelease(info.cfNameString.cast()) };
        }

        let mut value: AudioUnitParameterValue = info.defaultValue;
        unsafe {
            AudioUnitGetParameter(unit, id, kAudioUnitScope_Global, 0, &mut value);
        }

        out.push(AuParam {
            id,
            name,
            min: f64::from(info.minValue),
            max: f64::from(info.maxValue),
            value: f64::from(value),
        });
    }
    out
}

/// Read the fixed 52-byte C `name` field of an `AudioUnitParameterInfo`.
fn c_name(name: &[c_char]) -> Option<String> {
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(name.as_ptr().cast::<u8>(), name.len()) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    if end == 0 {
        return None;
    }
    std::str::from_utf8(&bytes[..end]).ok().map(str::to_owned)
}

/// Turn a non-zero `OSStatus` into an error tagged with the failing step.
fn check(status: OSStatus, step: &str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(anyhow!("audio unit {step} failed (OSStatus {status})"))
    }
}

/// Set an AU property from a `&T`, sizing the payload from `T`.
fn set_prop<T>(
    unit: AudioUnit,
    id: AudioUnitPropertyID,
    scope: AudioUnitScope,
    elem: AudioUnitElement,
    value: &T,
) -> Result<()> {
    let status = unsafe {
        AudioUnitSetProperty(
            unit,
            id,
            scope,
            elem,
            (value as *const T).cast::<c_void>(),
            std::mem::size_of::<T>() as u32,
        )
    };
    check(status, "set property")
}

/// Disposes an `AudioUnit` on drop unless defused. Used so an error partway through
/// [`load`] can't leak a half-initialised instance.
struct InstanceGuard(AudioUnit);

impl InstanceGuard {
    fn defuse(self) {
        std::mem::forget(self);
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            AudioComponentInstanceDispose(self.0);
        }
    }
}

/// Input the render callback hands to the AU for the current chunk. Held behind a
/// stable heap address (a `Box`) so the pointer captured as the callback refcon
/// stays valid for the life of the [`AuInsert`].
struct RenderCtx {
    /// Channel-major input for the current chunk (`in_ptr[c*stride..][..frames]`).
    in_ptr: *const f32,
    frames: usize,
    stride: usize,
}

/// A two-buffer `AudioBufferList`. The bindgen `AudioBufferList` has a
/// single-element flexible array, so we lay out our own with room for both channels
/// and cast the pointer when calling `AudioUnitRender`.
#[repr(C)]
struct BufferList2 {
    number_buffers: u32,
    buffers: [AudioBuffer; CHANNELS],
}

/// The AU's input pull callback: copy our stashed channel-major input into the
/// buffers the unit provides. Non-interleaved stereo in ⇒ one buffer per channel.
unsafe extern "C" fn input_render_cb(
    in_ref_con: *mut c_void,
    _flags: *mut AudioUnitRenderActionFlags,
    _ts: *const AudioTimeStamp,
    _bus: u32,
    frames: u32,
    io_data: *mut AudioBufferList,
) -> OSStatus {
    if in_ref_con.is_null() || io_data.is_null() {
        return 0;
    }
    // SAFETY: `in_ref_con` is the `Box<RenderCtx>` pointer we installed at load time
    // (stable heap address, alive for the AuInsert's life); `io_data` is the AU's own
    // buffer list. We only read `ctx` and write into the buffers the AU provided.
    unsafe {
        let ctx = &*in_ref_con.cast::<RenderCtx>();
        let list = &mut *io_data;
        let want = frames as usize;
        let have = want.min(ctx.frames);
        // The bindgen struct declares `mBuffers: [_; 1]`, so trust the runtime count
        // field rather than the array's static length (non-interleaved stereo has 2).
        let nbuf = list.mNumberBuffers as usize;
        for b in 0..nbuf {
            let buf = &mut *list.mBuffers.as_mut_ptr().add(b);
            let dst = buf.mData.cast::<f32>();
            if dst.is_null() {
                continue;
            }
            if ctx.in_ptr.is_null() || have == 0 {
                ptr::write_bytes(dst, 0, want);
                continue;
            }
            // Our input is always stereo; if the AU somehow wants more buffers, reuse
            // the last channel rather than reading out of bounds.
            let src_ch = b.min(CHANNELS - 1);
            let src = ctx.in_ptr.add(src_ch * ctx.stride);
            ptr::copy_nonoverlapping(src, dst, have);
            if want > have {
                ptr::write_bytes(dst.add(have), 0, want - have);
            }
        }
    }
    0
}

/// Bridges a live `AudioUnit` to the chain's [`StereoInsert`] slot. Owns the instance
/// and tears it down on drop.
struct AuInsert {
    unit: AudioUnit,
    /// Kept alive (and at a stable address) for the AU's callback refcon.
    ctx: Box<RenderCtx>,
    /// `CHANNELS * max_block`, channel-major (stride `max_block`).
    in_buf: Vec<f32>,
    /// `CHANNELS * max_block`, channel-major (stride `max_block`).
    out_buf: Vec<f32>,
    out_list: BufferList2,
    ts: AudioTimeStamp,
    max_block: usize,
    /// Steady sample-time counter handed to `AudioUnitRender`.
    steady: u64,
    /// Parameter changes pushed by the UI thread (installed via `with_params`).
    param_rx: Option<Consumer<AuParamChange>>,
}

// The instance is only ever touched from the audio thread once the insert is
// installed there, and disposed on the UI thread after it is displaced — never
// concurrently. That single-owner discipline is what makes this `Send`.
unsafe impl Send for AuInsert {}

impl AuInsert {
    fn new(unit: AudioUnit, ctx: Box<RenderCtx>, max_block: usize) -> Self {
        let zero_buf = AudioBuffer {
            mNumberChannels: 1,
            mDataByteSize: 0,
            mData: ptr::null_mut(),
        };
        Self {
            unit,
            ctx,
            in_buf: vec![0.0; CHANNELS * max_block],
            out_buf: vec![0.0; CHANNELS * max_block],
            out_list: BufferList2 {
                number_buffers: CHANNELS as u32,
                buffers: [zero_buf; CHANNELS],
            },
            ts: unsafe { std::mem::zeroed() },
            max_block,
            steady: 0,
            param_rx: None,
        }
    }

    fn with_params(mut self, rx: Consumer<AuParamChange>) -> Self {
        self.param_rx = Some(rx);
        self
    }

    /// Process one chunk no larger than `max_block`. `left`/`right` are equal length,
    /// read for input and overwritten with the AU's output.
    fn process_chunk(&mut self, left: &mut [f32], right: &mut [f32]) {
        let m = left.len();
        let stride = self.max_block;

        // Deinterleave our stereo pair into the channel-major input buffer, then point
        // the render context at it for this chunk.
        self.in_buf[..m].copy_from_slice(left);
        self.in_buf[stride..stride + m].copy_from_slice(right);
        self.ctx.in_ptr = self.in_buf.as_ptr();
        self.ctx.frames = m;
        self.ctx.stride = stride;

        // Apply queued parameter changes (AudioUnitSetParameter is realtime-safe).
        if let Some(rx) = self.param_rx.as_mut() {
            while let Ok(change) = rx.pop() {
                unsafe {
                    AudioUnitSetParameter(
                        self.unit,
                        change.id,
                        kAudioUnitScope_Global,
                        0,
                        change.value,
                        0,
                    );
                }
            }
        }

        // Point the output buffer list at our channel-major output scratch.
        for c in 0..CHANNELS {
            let buf = &mut self.out_list.buffers[c];
            buf.mNumberChannels = 1;
            buf.mDataByteSize = std::mem::size_of_val(left) as u32;
            buf.mData = unsafe { self.out_buf.as_mut_ptr().add(c * stride) }.cast::<c_void>();
        }
        self.out_list.number_buffers = CHANNELS as u32;

        self.ts.mSampleTime = self.steady as f64;
        self.ts.mFlags = kAudioTimeStampSampleTimeValid as AudioTimeStampFlags;

        let mut flags: AudioUnitRenderActionFlags = 0;
        let status = unsafe {
            AudioUnitRender(
                self.unit,
                &mut flags,
                &self.ts,
                0,
                m as u32,
                (&mut self.out_list as *mut BufferList2).cast::<AudioBufferList>(),
            )
        };
        self.steady += m as u64;

        if status == 0 {
            left.copy_from_slice(&self.out_buf[..m]);
            right.copy_from_slice(&self.out_buf[stride..stride + m]);
        }
        // On error, leave left/right untouched (the dry pre-amp signal passes through).
    }
}

impl StereoInsert for AuInsert {
    fn process_block(&mut self, left: &mut [f32], right: &mut [f32]) {
        // The host block may exceed the AU's configured maximum slice; split it.
        let len = left.len();
        let mut start = 0;
        while start < len {
            let end = (start + self.max_block).min(len);
            self.process_chunk(&mut left[start..end], &mut right[start..end]);
            start = end;
        }
    }
}

impl Drop for AuInsert {
    fn drop(&mut self) {
        unsafe {
            AudioUnitUninitialize(self.unit);
            AudioComponentInstanceDispose(self.unit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The handoff design depends on the insert being `Send` so it can be moved to
    /// the audio thread. This won't compile if that ever stops holding.
    #[test]
    fn au_insert_is_send() {
        const fn assert_send<T: Send>() {}
        assert_send::<AuInsert>();
        assert_send::<Box<dyn StereoInsert>>();
    }

    /// Scanning must not crash and should return named components on a normal macOS
    /// box. We don't assert a count (CI runners may have none installed).
    #[test]
    fn scan_returns_named_components() {
        for au in scan() {
            assert!(!au.name.is_empty(), "discovered AU with empty name");
        }
    }
}
