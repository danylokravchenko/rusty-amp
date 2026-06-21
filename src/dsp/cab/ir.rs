//! Synthesised cabinet impulse responses.
//!
//! Rather than ship external .wav files, we generate a realistic cab IR in code.
//! The recipe deliberately reuses each cabinet's existing, carefully voiced
//! biquad EQ as the **magnitude skeleton** (so the tonal balance that was already
//! dialled in is preserved), then layers on the **time-domain** structure that a
//! static EQ fundamentally cannot produce and that makes a real miked cab sound
//! three-dimensional:
//!
//!   • early reflections (cone-to-grille, cabinet panels, mic bounce) → the dense
//!     comb filtering that gives a cab its "in a room" depth and movement;
//!   • speaker modal resonances (low cone "thump" + cone-breakup ring) → the
//!     decaying tail that a minimum-phase EQ has no way to express.
//!
//! Two slightly different textures are used for the left/right speakers, which
//! decorrelates the stereo image for natural width.

use std::f32::consts::PI;

/// Time-domain texture applied on top of the magnitude skeleton.
pub struct Texture {
    /// Mic-distance pre-delay, in samples (small for close-mic).
    pub predelay: usize,
    /// Early reflections as (time_ms, signed_gain). Signs alternate to create
    /// the characteristic comb peaks and notches.
    pub reflections: &'static [(f32, f32)],
    /// Speaker resonant modes as (freq_hz, t60_ms, gain).
    pub modes: &'static [(f32, f32, f32)],
}

/// Build an impulse response of `len` taps.
///
/// `voicing` is the cabinet's EQ chain fed one sample at a time; we drive it with
/// a unit impulse to capture its impulse response as the magnitude skeleton.
pub fn synth(sr: f32, len: usize, voicing: &mut dyn FnMut(f32) -> f32, tex: &Texture) -> Vec<f32> {
    // 1. Magnitude skeleton = impulse response of the voiced biquad EQ chain.
    let mut skel = vec![0.0f32; len];
    for (n, s) in skel.iter_mut().enumerate() {
        *s = voicing(if n == 0 { 1.0 } else { 0.0 });
    }

    // 2. Early-reflection comb kernel (direct path + reflections).
    let max_ms = tex.reflections.iter().fold(0.0f32, |m, r| m.max(r.0));
    let comb_len = (max_ms * sr / 1000.0) as usize + tex.predelay + 2;
    let mut comb = vec![0.0f32; comb_len.max(1)];
    let direct = tex.predelay.min(comb.len() - 1);
    comb[direct] = 1.0;
    for &(ms, g) in tex.reflections {
        let idx = tex.predelay + (ms * sr / 1000.0) as usize;
        if idx < comb.len() {
            comb[idx] += g;
        }
    }

    // 3. Convolve skeleton with the comb kernel → body with comb coloration.
    let mut ir = vec![0.0f32; len];
    for (i, out) in ir.iter_mut().enumerate() {
        let mut acc = 0.0f32;
        for (k, &c) in comb.iter().enumerate() {
            if i >= k {
                acc += c * skel[i - k];
            }
        }
        *out = acc;
    }

    // 4. Add decaying modal resonances (cone + breakup ring).
    for &(f, t60_ms, g) in tex.modes {
        // t60 (−60 dB) → exponential time constant: ln(1000) ≈ 6.908.
        let tau = (t60_ms / 1000.0) * sr / 6.908;
        let w = 2.0 * PI * f / sr;
        for (n, out) in ir.iter_mut().enumerate() {
            let nn = n.saturating_sub(tex.predelay) as f32;
            *out += g * (-nn / tau).exp() * (w * nn).sin();
        }
    }

    // 5. Raised-cosine fade over the final quarter to avoid a truncation click.
    let fade_start = len * 3 / 4;
    for (n, out) in ir.iter_mut().enumerate().skip(fade_start) {
        let p = (n - fade_start) as f32 / (len - fade_start).max(1) as f32;
        *out *= 0.5 * (1.0 + (PI * p).cos());
    }

    // 6. Normalise to the skeleton's energy so overall loudness matches the
    //    original EQ voicing (the rest of the chain was gain-staged around it).
    let e_skel = skel.iter().map(|v| v * v).sum::<f32>().sqrt();
    let e_ir = ir.iter().map(|v| v * v).sum::<f32>().sqrt();
    if e_ir > 1e-9 {
        let g = e_skel / e_ir;
        for v in &mut ir {
            *v *= g;
        }
    }

    ir
}

/// IR length in taps for a given sample rate (~11.6 ms, plenty for a cab body).
pub fn ir_len(sr: f32) -> usize {
    ((sr / 44100.0) * 512.0) as usize
}
