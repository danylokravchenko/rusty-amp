pub mod external;
pub mod ir;
pub mod marshall;
pub mod mesa;
pub mod orange;

use crate::dsp::biquad::Biquad;
use crate::dsp::conv::FftConvolver;

pub use external::{ExternalIrCab, LoadedIr, MAX_IR_LEN, load_ir};
pub use marshall::MarshallCab;
pub use mesa::MesaCab;
pub use orange::OrangeCab;

pub trait Cabinet {
    /// Convolve a mono amp sample with the cab IR, returning a stereo (L, R) pair.
    ///
    /// `mic_pos` moves the close mic edge→centre, `blend` crossfades the close
    /// dynamic (SM57) into a ribbon (R121), and `room` adds an ambient room mic.
    fn process(&mut self, sample: f32, mic_pos: f32, blend: f32, room: f32) -> (f32, f32);
}

// ── Speaker nonlinearities (shared by every cab) ───────────────────────────────

// The blended convolution above is the cab's *linear* response (cone+cab+mic+room
// magnitude and the reflection/modal time structure). Two things a fixed IR can
// never capture happen at the speaker itself, *before* the mic picks the sound up,
// so they are modelled on the mono drive signal feeding the convolver:
//
//   • cone-breakup saturation — a real cone is not a perfectly rigid piston; pushed
//     hard it flexes and the radiated waveform soft-saturates, adding low-order
//     harmonic "thickness" that grows with how hard the cone is driven;
//   • power compression — the voice coil heats under sustained high power, its
//     resistance rises, and acoustic output compresses. It is a *thermal* effect:
//     slow to engage and slow to release, so transients punch through while
//     sustained loud passages "push back". This is what makes a loud cab feel alive
//     rather than a flat playback of an IR.
//
// Both are deliberately gentle. The goal is the weight of a real driver, not an
// audible distortion/limiter effect, so at normal levels the signal passes almost
// untouched and the shaping only emerges when the cab is genuinely driven hard.

/// Cone-breakup soft saturation. `1 + DRIVE` small-signal gain ≈ unity (a clean
/// cone), with a gentle, slightly asymmetric saturation on peaks. The asymmetry
/// adds the even-harmonic warmth a real cone produces; any DC it introduces is
/// removed downstream by the cab IR (whose 0 Hz gain is ~0). Bounded via `tanh`,
/// so it also can't blow up on a hot amp input.
const BREAKUP_DRIVE: f32 = 0.16;
const BREAKUP_ASYM: f32 = 0.07;

#[inline]
fn cone_breakup(x: f32) -> f32 {
    let s = x * (1.0 + BREAKUP_DRIVE);
    ((s + BREAKUP_ASYM).tanh() - BREAKUP_ASYM.tanh()) / (1.0 + BREAKUP_DRIVE)
}

/// Power-compression knee: below the threshold the drive is untouched; above it the
/// gain falls smoothly. `RATIO_K` sets how hard it leans in — kept mild so the cab
/// rounds and thickens rather than pumps.
const PC_THRESHOLD: f32 = 0.35;
const PC_RATIO_K: f32 = 0.6;
const PC_ATK_MS: f32 = 20.0; // fast enough to pass transients, slow enough to be thermal
const PC_REL_MS: f32 = 350.0; // slow recovery: sustained loud passages stay compressed

/// Off-axis comb: path-length differences across the cone toward the edge sum a
/// short-delayed copy of the signal back in, carving the comb notches that make an
/// edge mic sound hollow/dark. ~0.28 ms puts the first notch near 1.8 kHz.
const COMB_MS: f32 = 0.28;
const COMB_MAX_DEPTH: f32 = 0.5;

/// Proximity: moving on-axis/closer (toward centre) lifts the lows. A low shelf
/// centred low, swinging ±(RANGE/2) dB across the knob, neutral at the centre detent.
const PROX_FREQ: f32 = 150.0;
const PROX_RANGE_DB: f32 = 6.0;

/// Axis brightness (the original behaviour): centre is on-axis/bright, edge dark.
const SHELF_FREQ: f32 = 5000.0;
const SHELF_RANGE_DB: f32 = 12.0;

/// Short feed-forward comb (`y = x + g·x[n−d]`) for the off-axis mic colouration.
struct Comb {
    buf: Vec<f32>,
    pos: usize,
}

impl Comb {
    fn new(sr: f32, max_ms: f32) -> Self {
        let n = (sr * max_ms / 1000.0) as usize + 2;
        Self {
            buf: vec![0.0; n.max(2)],
            pos: 0,
        }
    }

    #[inline]
    fn process(&mut self, x: f32, g: f32, delay: usize) -> f32 {
        let len = self.buf.len();
        let d = delay.clamp(1, len - 1);
        let read = (self.pos + len - d) % len;
        let delayed = self.buf[read];
        self.buf[self.pos] = x;
        self.pos = (self.pos + 1) % len;
        x + g * delayed
    }
}

// ── Stage 1: speaker drive (mono, pre-mic) ──────────────────────────────────────

/// The two driver nonlinearities a fixed IR can't hold, applied to the mono drive
/// before the mic picks the sound up: stateless [`cone_breakup`] saturation
/// followed by stateful voice-coil thermal power compression.
struct SpeakerDrive {
    env: f32,
    atk: f32,
    rel: f32,
}

impl SpeakerDrive {
    fn new(sr: f32) -> Self {
        let coeff = |ms: f32| 1.0 - (-1.0 / (sr * ms / 1000.0)).exp();
        Self {
            env: 0.0,
            atk: coeff(PC_ATK_MS),
            rel: coeff(PC_REL_MS),
        }
    }

    /// Cone breakup, then voice-coil thermal power compression. The envelope tracks
    /// the signal with a fast-ish attack and slow release (so transients pass and
    /// only sustained level compresses), and the gain rolls off smoothly above the
    /// threshold.
    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let x = cone_breakup(x);
        let a = x.abs();
        let coeff = if a > self.env { self.atk } else { self.rel };
        self.env += (a - self.env) * coeff;
        let over = (self.env - PC_THRESHOLD).max(0.0);
        x / (1.0 + PC_RATIO_K * over)
    }
}

// ── Stage 2: multi-mic blend convolution ────────────────────────────────────────

/// The three-mic blend rendered as a single pair of convolvers. Each capture is a
/// full impulse response (its own voicing + reflection texture, with the room mic
/// carrying extra pre-delay and denser late reflections). Because convolution is
/// linear, blending the mics is just a weighted **sum of their IRs**: the three IRs
/// are precomputed once and, whenever a blend knob moves, recombined into the live
/// convolver taps. The per-sample cost is therefore exactly two convolutions
/// regardless of how many mics are in the blend, and swapping the taps preserves the
/// delay-line history so it never clicks.
struct MicBlend {
    conv_l: FftConvolver,
    conv_r: FftConvolver,
    // Per-mic impulse responses (close / ribbon / room), per channel.
    close_l: Vec<f32>,
    close_r: Vec<f32>,
    ribbon_l: Vec<f32>,
    ribbon_r: Vec<f32>,
    room_l: Vec<f32>,
    room_r: Vec<f32>,
    // Preallocated combine buffers so the hot path never allocates.
    scratch_l: Vec<f32>,
    scratch_r: Vec<f32>,
    last_blend: f32,
    last_room: f32,
}

impl MicBlend {
    fn new(irs: [Vec<f32>; 6]) -> Self {
        let [close_l, close_r, ribbon_l, ribbon_r, room_l, room_r] = irs;
        let cap = close_l.len() + 1;
        let mut blend = Self {
            conv_l: FftConvolver::new(cap),
            conv_r: FftConvolver::new(cap),
            scratch_l: vec![0.0; close_l.len()],
            scratch_r: vec![0.0; close_r.len()],
            close_l,
            close_r,
            ribbon_l,
            ribbon_r,
            room_l,
            room_r,
            last_blend: -1.0,
            last_room: -1.0,
        };
        blend.recombine(0.0, 0.0);
        blend
    }

    /// Reload the convolver taps if the blend changed. `blend` 0 = close dynamic …
    /// 1 = ribbon; `room` 0–1 = ambient room amount.
    fn set(&mut self, blend: f32, room: f32) {
        if (blend - self.last_blend).abs() > 0.001 || (room - self.last_room).abs() > 0.001 {
            self.recombine(blend, room);
        }
    }

    fn recombine(&mut self, blend: f32, room: f32) {
        let wc = 1.0 - blend; // close dynamic weight
        let wr = blend; // ribbon weight
        let wroom = room * 0.9; // room ambience: real captures carry ~25% late energy
        for (i, s) in self.scratch_l.iter_mut().enumerate() {
            *s = wc * self.close_l[i] + wr * self.ribbon_l[i] + wroom * self.room_l[i];
        }
        for (i, s) in self.scratch_r.iter_mut().enumerate() {
            *s = wc * self.close_r[i] + wr * self.ribbon_r[i] + wroom * self.room_r[i];
        }
        self.conv_l.load(&self.scratch_l);
        self.conv_r.load(&self.scratch_r);
        self.last_blend = blend;
        self.last_room = room;
    }

    #[inline]
    fn process(&mut self, drive: f32) -> (f32, f32) {
        (self.conv_l.process(drive), self.conv_r.process(drive))
    }
}

// ── Stage 3: physical mic-position colouration (per channel) ────────────────────

/// One captured channel's mic-position colouration, applied in physical order:
/// proximity low-shelf → axis-brightness high-shelf → off-axis comb.
struct MicChannel {
    prox: Biquad,
    shelf: Biquad,
    comb: Comb,
}

impl MicChannel {
    fn new(sr: f32) -> Self {
        Self {
            prox: Biquad::low_shelf(sr, PROX_FREQ, 0.0),
            shelf: Biquad::high_shelf(sr, SHELF_FREQ, 0.0),
            comb: Comb::new(sr, COMB_MS),
        }
    }

    /// Re-dial the two shelves for a new mic position. The comb gain/delay are shared
    /// across channels and passed into [`MicChannel::process`].
    fn retune(&mut self, sr: f32, prox_db: f32, bright_db: f32) {
        self.prox = Biquad::low_shelf(sr, PROX_FREQ, prox_db);
        self.shelf = Biquad::high_shelf(sr, SHELF_FREQ, bright_db);
    }

    #[inline]
    fn process(&mut self, x: f32, comb_g: f32, comb_d: usize) -> f32 {
        let x = self.prox.process(x);
        let x = self.shelf.process(x);
        self.comb.process(x, comb_g, comb_d)
    }
}

/// The mic-position model across both channels: maps the edge↔centre knob to
/// proximity lows, axis brightness, and an off-axis comb, so the knob feels like
/// sliding a mic across the cone rather than tilting an EQ.
struct MicPosition {
    sr: f32,
    l: MicChannel,
    r: MicChannel,
    comb_g: f32,
    comb_d: usize,
    last_pos: f32,
}

impl MicPosition {
    fn new(sr: f32) -> Self {
        Self {
            sr,
            l: MicChannel::new(sr),
            r: MicChannel::new(sr),
            comb_g: 0.0,
            comb_d: ((sr * COMB_MS / 1000.0) as usize).max(1),
            last_pos: -1.0,
        }
    }

    /// Re-dial the per-channel filters and comb if the position changed.
    fn set(&mut self, pos: f32) {
        if (pos - self.last_pos).abs() <= 0.001 {
            return;
        }
        // 0 = edge (off-axis, dark), 1 = centre (on-axis, bright); 0.5 = neutral.
        let bright = (pos - 0.5) * SHELF_RANGE_DB;
        // Proximity: lows rise on-axis/closer (toward centre), fall toward edge.
        let prox = (pos - 0.5) * PROX_RANGE_DB;
        self.l.retune(self.sr, prox, bright);
        self.r.retune(self.sr, prox, bright);
        // Off-axis comb: only when moving past centre toward the edge.
        let off_axis = (0.5 - pos).max(0.0) * 2.0; // 0 at centre, 1 at the edge
        self.comb_g = -off_axis * COMB_MAX_DEPTH;
        self.last_pos = pos;
    }

    #[inline]
    fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        (
            self.l.process(l, self.comb_g, self.comb_d),
            self.r.process(r, self.comb_g, self.comb_d),
        )
    }
}

// ── Composed cabinet ────────────────────────────────────────────────────────────

/// A studio "mic'd cab" assembled from three stages, in signal order:
///   1. [`SpeakerDrive`] — the speaker's cone-breakup saturation and thermal power
///      compression on the mono drive (the parts of a real cab a fixed IR can't hold);
///   2. [`MicBlend`] — the linear capture: a blend of three mic IRs (close SM57
///      dynamic, close R121 ribbon, ambient room) convolved per channel;
///   3. [`MicPosition`] — the physical edge↔centre mic-position colouration on each
///      captured channel (proximity low-shelf + axis brightness + off-axis comb).
///
/// Each stage owns its own state, parameter-change caching, and coefficient updates;
/// `process` just threads a sample through them.
pub struct BlendedCab {
    speaker: SpeakerDrive,
    blend: MicBlend,
    mic: MicPosition,
}

impl BlendedCab {
    /// Build from the six prebuilt IRs: `[close_l, close_r, ribbon_l, ribbon_r,
    /// room_l, room_r]`.
    pub fn new(sr: f32, irs: [Vec<f32>; 6]) -> Self {
        Self {
            speaker: SpeakerDrive::new(sr),
            blend: MicBlend::new(irs),
            mic: MicPosition::new(sr),
        }
    }

    #[inline]
    pub fn process(&mut self, sample: f32, mic_pos: f32, blend: f32, room: f32) -> (f32, f32) {
        self.blend.set(blend, room);
        self.mic.set(mic_pos);

        let drive = self.speaker.process(sample);
        let (l, r) = self.blend.process(drive);
        self.mic.process(l, r)
    }
}

/// Owns all cabinet instances simultaneously so filter state survives model switches.
pub struct CabBank {
    mesa: MesaCab,
    marshall: MarshallCab,
    orange: OrangeCab,
}

impl CabBank {
    pub fn new(sr: f32) -> Self {
        Self {
            mesa: MesaCab::new(sr),
            marshall: MarshallCab::new(sr),
            orange: OrangeCab::new(sr),
        }
    }

    #[inline]
    pub fn process(
        &mut self,
        model: super::CabModel,
        sample: f32,
        mic_pos: f32,
        blend: f32,
        room: f32,
    ) -> (f32, f32) {
        match model {
            super::CabModel::Mesa => self.mesa.process(sample, mic_pos, blend, room),
            super::CabModel::Marshall => self.marshall.process(sample, mic_pos, blend, room),
            super::CabModel::Orange => self.orange.process(sample, mic_pos, blend, room),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::CabModel;
    use std::f32::consts::PI;

    const SR: f32 = 48_000.0;

    fn goertzel(samples: &[f32], f: f32, sr: f32) -> f32 {
        let w = 2.0 * PI * f / sr;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f32, 0.0f32);
        for &x in samples {
            let s0 = x + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let real = s1 - s2 * w.cos();
        let imag = s2 * w.sin();
        (real * real + imag * imag).sqrt() / (samples.len() as f32 / 2.0)
    }

    /// Render a sustained sine of amplitude `amp` at `freq` through a fresh cab at
    /// the given `mic_pos`, returning the mono (L+R) steady-state tail (warm-up
    /// thirds dropped so filter/compression transients are excluded).
    fn render(freq: f32, amp: f32, mic_pos: f32) -> Vec<f32> {
        let mut cab = MarshallCab::new(SR);
        let n = SR as usize;
        let warm = n / 3;
        let mut out = Vec::with_capacity(n - warm);
        for i in 0..n {
            let x = (2.0 * PI * freq * i as f32 / SR).sin() * amp;
            let (l, r) = cab.process(x, mic_pos, 0.15, 0.15);
            assert!(l.is_finite() && r.is_finite(), "non-finite cab output");
            if i >= warm {
                out.push(l + r);
            }
        }
        out
    }

    /// Steady-state RMS of a single tone through the cab at `mic_pos`.
    fn tone_rms(freq: f32, amp: f32, mic_pos: f32) -> f32 {
        let out = render(freq, amp, mic_pos);
        (out.iter().map(|&x| x * x).sum::<f32>() / out.len() as f32).sqrt()
    }

    // ── 1. Richer mic-position model (proximity + comb) ────────────────────────

    /// Proximity: moving on-axis/closer (toward the centre) must lift the lows. A
    /// low note is measurably stronger at `mic_pos` 0.85 than at 0.15 — the knob
    /// physically moves the mic, it is not a top-end-only tilt.
    #[test]
    fn proximity_lifts_lows_on_axis() {
        let low_centre = goertzel(&render(110.0, 0.25, 0.85), 110.0, SR);
        let low_edge = goertzel(&render(110.0, 0.25, 0.15), 110.0, SR);
        assert!(
            low_centre > low_edge * 1.15,
            "proximity dead: 110 Hz centre {low_centre:.5} vs edge {low_edge:.5}"
        );
    }

    /// Axis brightness: the centre (on-axis) must be brighter up top than the edge.
    /// Guards the high-shelf half of the mic-position move.
    #[test]
    fn centre_is_brighter_than_edge() {
        let hi_centre = tone_rms(6000.0, 0.25, 0.85);
        let hi_edge = tone_rms(6000.0, 0.25, 0.15);
        assert!(
            hi_centre > hi_edge * 1.3,
            "axis brightness dead: 6 kHz centre {hi_centre:.5} vs edge {hi_edge:.5}"
        );
    }

    /// Off-axis comb: at the edge, path-length differences must carve a comb — the
    /// notch frequency is attenuated relative to the comb peak far more than the
    /// IR alone (centre, comb bypassed) shapes them. This is the "moving a mic"
    /// hollowing a fixed EQ tilt cannot produce.
    #[test]
    fn off_axis_introduces_comb_notch() {
        // For the negative comb gain, the crest sits at 1/(2d) and the null at 1/d.
        let peak = 500.0 / COMB_MS; // comb crest (~1.8 kHz)
        let notch = 1000.0 / COMB_MS; // first comb null (~3.6 kHz)
        // Peak/notch contrast from the cab at the edge vs at the centre (no comb).
        let edge = tone_rms(peak, 0.25, 0.0) / tone_rms(notch, 0.25, 0.0).max(1e-9);
        let centre = tone_rms(peak, 0.25, 0.5) / tone_rms(notch, 0.25, 0.5).max(1e-9);
        assert!(
            edge > centre * 1.5,
            "off-axis comb absent: edge peak/notch {edge:.3} vs centre {centre:.3}"
        );
    }

    /// The centre detent (0.5) must be the neutral reference: the comb is fully
    /// bypassed there, so the notch frequency is not attenuated relative to a
    /// neighbour the way it is at the edge. Guards against the comb leaking into the
    /// shipped default `mic_pos`.
    #[test]
    fn centre_detent_is_comb_neutral() {
        let notch = 1000.0 / COMB_MS; // the comb null (~3.6 kHz)
        let near = notch * 0.85;
        let centre_ratio = tone_rms(notch, 0.25, 0.5) / tone_rms(near, 0.25, 0.5).max(1e-9);
        let edge_ratio = tone_rms(notch, 0.25, 0.0) / tone_rms(near, 0.25, 0.0).max(1e-9);
        assert!(
            edge_ratio < centre_ratio * 0.85,
            "comb present at the centre detent: centre {centre_ratio:.3} vs edge {edge_ratio:.3}"
        );
    }

    // ── 2. Power compression + cone-breakup nonlinearity ───────────────────────

    /// Power compression: at high SPL the cab must compress (sub-linear gain), so a
    /// loud tone's output/input gain is lower than a quiet tone's. The "push back"
    /// of a driven cab.
    #[test]
    fn loud_signal_is_power_compressed() {
        let quiet_gain = tone_rms(110.0, 0.05, 0.5) / 0.05;
        let loud_gain = tone_rms(110.0, 1.0, 0.5) / 1.0;
        let ratio = loud_gain / quiet_gain;
        assert!(
            ratio < 0.9,
            "no power compression: loud gain is {ratio:.2}× the quiet gain"
        );
        // …but it must stay gentle (thickening, not a brickwall limiter).
        assert!(
            ratio > 0.3,
            "power compression too aggressive (squashes the cab): {ratio:.2}×"
        );
    }

    /// Power compression is *thermal*: a fast transient must punch through before
    /// the slow envelope engages. The first few milliseconds of a cold loud burst
    /// must be louder than the settled steady state of the same tone.
    ///
    /// Probed at 1.9 kHz, away from the cab's low resonant hump and body modes
    /// (at a resonance the linear ring-up would mask the compression under test),
    /// with an attack window long enough (12 ms) for most of the ~46 ms IR's
    /// reflections to contribute to the "uncompressed" peak, yet well inside the
    /// 20 ms thermal attack.
    #[test]
    fn transient_punches_through_thermal_compression() {
        let mut cab = MarshallCab::new(SR);
        let mut peak_attack = 0.0f32;
        let attack_n = (SR * 0.012) as usize; // first 12 ms
        let mut settled = 0.0f32;
        let total = (SR * 0.5) as usize;
        for i in 0..total {
            let x = (2.0 * PI * 1900.0 * i as f32 / SR).sin();
            let (l, r) = cab.process(x, 0.5, 0.15, 0.15);
            let m = (l + r).abs();
            if i < attack_n {
                peak_attack = peak_attack.max(m);
            }
            if i >= total - attack_n {
                settled = settled.max(m);
            }
        }
        assert!(
            peak_attack > settled * 1.05,
            "transient ducked instantly (not thermal): attack {peak_attack:.3} vs settled {settled:.3}"
        );
    }

    /// Cone breakup must be level-dependent: a loud tone picks up more harmonic
    /// "thickness" (2nd+3rd) relative to its fundamental than a quiet one. The cab
    /// reacts to how hard it is driven instead of being a static playback of an IR.
    #[test]
    fn cone_breakup_thickens_with_level() {
        let f = 220.0;
        let thd = |amp: f32| {
            let out = render(f, amp, 0.5);
            let fund = goertzel(&out, f, SR).max(1e-9);
            let harm = goertzel(&out, 2.0 * f, SR) + goertzel(&out, 3.0 * f, SR);
            harm / fund
        };
        let quiet = thd(0.05);
        let loud = thd(0.95);
        assert!(
            loud > quiet * 1.5,
            "cone breakup not level-dependent: quiet THD {quiet:.4} loud {loud:.4}"
        );
    }

    /// …and it must stay a *deep, real* breakup, not an "acid" digital fuzz: clean,
    /// low-level playing passes nearly untouched (tiny THD), and even when driven
    /// hard the harmonics stay well below the fundamental (thickening, not a fuzz
    /// box). Plus no aliased fizz survives above the cab's rolloff.
    #[test]
    fn breakup_stays_clean_and_musical() {
        let f = 220.0;
        let clean = {
            let out = render(f, 0.05, 0.5);
            let fund = goertzel(&out, f, SR).max(1e-9);
            (goertzel(&out, 2.0 * f, SR) + goertzel(&out, 3.0 * f, SR)) / fund
        };
        assert!(clean < 0.1, "breakup dirties clean playing: THD {clean:.4}");

        let out = render(f, 0.95, 0.5);
        let fund = goertzel(&out, f, SR).max(1e-9);
        let harm = goertzel(&out, 2.0 * f, SR) + goertzel(&out, 3.0 * f, SR);
        assert!(
            harm / fund < 0.5,
            "driven breakup is fuzzy/artificial: harm/fund {:.3}",
            harm / fund
        );
        // Aliasing/fizz above the cab rolloff must stay negligible.
        let mut fizz = 0.0f32;
        let mut g = 6500.0;
        while g < 12_000.0 {
            fizz += goertzel(&out, g, SR).powi(2);
            g *= 2.0_f32.powf(1.0 / 12.0);
        }
        assert!(
            fizz.sqrt() / fund < 0.05,
            "fizz above rolloff: {:.4}",
            fizz.sqrt() / fund
        );
    }

    /// Across every cab model, the full mic-position sweep at a hot drive must stay
    /// finite, bounded, and free of DC offset — the new feedback-free nonlinearities
    /// and comb must never blow up or leak a sub-DC bias into the stereo bus.
    #[test]
    fn stable_bounded_and_dc_free_across_the_sweep() {
        for model in [CabModel::Mesa, CabModel::Marshall, CabModel::Orange] {
            for &pos in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
                let mut bank = CabBank::new(SR);
                let n = SR as usize / 2;
                let mut max_abs = 0.0f32;
                let mut sum = 0.0f64;
                let mut count = 0u32;
                for i in 0..n {
                    // 120 Hz divides the 48 kHz rate exactly (400 samples/period),
                    // so the DC average below spans whole periods and carries no
                    // truncation residue — it measures true DC only.
                    let x = (2.0 * PI * 120.0 * i as f32 / SR).sin() * 1.2;
                    let (l, r) = bank.process(model, x, pos, 0.2, 0.2);
                    assert!(l.is_finite() && r.is_finite(), "non-finite at pos {pos}");
                    max_abs = max_abs.max(l.abs()).max(r.abs());
                    if i >= n / 3 {
                        sum += (l + r) as f64;
                        count += 1;
                    }
                }
                // Bound sized to the voiced low-end: the cabs peak ~+9 dB near
                // 100–140 Hz (commercial captures like God's Cab measure +12 dB
                // there), so a 1.2-amplitude 110 Hz tone legitimately leaves the
                // cab near 3×. The guard is against instability/runaway, not the
                // voicing itself.
                assert!(max_abs < 4.0, "cab runaway at pos {pos}: {max_abs}");
                let dc = (sum / count as f64).abs();
                assert!(dc < 0.02, "cab leaks DC at pos {pos}: {dc:.4}");
            }
        }
    }
}
