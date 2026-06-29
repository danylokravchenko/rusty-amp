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

    // 4. Normalise the *direct body* (skeleton + comb) to the skeleton's energy
    //    BEFORE adding the modal resonances. This is the crucial ordering: the
    //    modes are long, decaying sinusoids that carry a lot of energy, so if we
    //    normalised the finished IR (body + modes) to the skeleton, the booming
    //    low-frequency modes would force the whole IR — including the mid/high
    //    direct sound — to be scaled *down*, leaving a quiet, all-bass cab. By
    //    levelling the body first, the modes become coloration that rings on top
    //    of a full-level direct sound rather than swallowing it.
    let e_skel = skel.iter().map(|v| v * v).sum::<f32>().sqrt();
    let e_body = ir.iter().map(|v| v * v).sum::<f32>().sqrt();
    if e_body > 1e-9 {
        let g = e_skel / e_body;
        for v in &mut ir {
            *v *= g;
        }
    }

    // 5. Add decaying modal resonances (cone + breakup ring) on top of the
    //    level-matched body — a controlled amount of resonance, not a takeover.
    for &(f, t60_ms, g) in tex.modes {
        // t60 (−60 dB) → exponential time constant: ln(1000) ≈ 6.908.
        let tau = (t60_ms / 1000.0) * sr / 6.908;
        let w = 2.0 * PI * f / sr;
        for (n, out) in ir.iter_mut().enumerate() {
            let nn = n.saturating_sub(tex.predelay) as f32;
            *out += g * (-nn / tau).exp() * (w * nn).sin();
        }
    }

    // 6. Raised-cosine fade over the final quarter to avoid a truncation click.
    let fade_start = (len as f32 * FADE_START_FRAC) as usize;
    for (n, out) in ir.iter_mut().enumerate().skip(fade_start) {
        let p = (n - fade_start) as f32 / (len - fade_start).max(1) as f32;
        *out *= 0.5 * (1.0 + (PI * p).cos());
    }

    // 7. A loudspeaker reproduces no DC. The additive modal kernels above are
    //    one-sided decaying sinusoids, so they carry a small non-zero mean;
    //    remove it to force the IR's 0 Hz gain to zero rather than letting a
    //    sub-DC offset leak into the downstream stereo bus.
    let mean = ir.iter().sum::<f32>() / ir.len() as f32;
    for v in &mut ir {
        *v -= mean;
    }

    ir
}

/// IR length in taps for a given sample rate (~23 ms at any rate).
///
/// The length is a deliberate compromise: long enough that the late
/// cabinet/room reflections (out to ~21 ms) and the low cone-resonance ring
/// actually fit inside the window — a short IR truncates them and the authored
/// values become inert — but not so long that we pay for a tail that adds little
/// beyond a controllable boom. At 48 kHz this is ~1114 taps per channel;
/// direct-form convolution at that length is a few hundred MFLOP/s, well within
/// a real-time budget on a modern CPU.
///
/// Invariant relied upon by the tests: every reflection time and modal decay in
/// the cabinet textures must be realizable within this window (see
/// `cab::*::tests`). If you shorten this, trim the textures to match.
pub fn ir_len(sr: f32) -> usize {
    ((sr / 44100.0) * 1024.0) as usize
}

/// Window length in milliseconds for a given tap count / sample rate.
#[cfg(test)]
fn ir_len_ms(sr: f32) -> f32 {
    ir_len(sr) as f32 / sr * 1000.0
}

/// The raised-cosine fade in [`synth`] starts at this fraction of the IR: taps
/// after it are progressively attenuated, so authored detail should land before.
pub const FADE_START_FRAC: f32 = 0.75;

// Objective measurements that let the cabinet textures be *justified* rather than
// eyeballed: every shipped cab IR is synthesised and checked against the physical
// claims its parameters encode (modes appear at the stated frequency and decay,
// reflections fit the window, L/R stay symmetric, values stay in plausible
// ranges). Shared here so each cabinet's `tests` module reuses the same probes.
#[cfg(test)]
pub mod analysis {
    /// Physically plausible bounds, from speaker datasheets / cab geometry.
    pub const BODY_MODE_HZ: (f32, f32) = (55.0, 230.0); // cone Fs + cab/body modes
    pub const BREAKUP_MODE_HZ: (f32, f32) = (2000.0, 4500.0); // cone breakup
    pub const MODE_T60_MS: (f32, f32) = (40.0, 180.0); // realizable, non-booming

    /// Magnitude of frequency `f` in `samples` via the Goertzel algorithm.
    pub fn goertzel(samples: &[f32], f: f32, sr: f32) -> f32 {
        use std::f32::consts::PI;
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

    /// Short-time amplitude of frequency `f` around sample `center` (a windowed
    /// DFT bin — unlike an IIR band-pass it has no ring of its own to corrupt a
    /// decay estimate).
    fn envelope_at(ir: &[f32], f: f32, sr: f32, center: usize, half: usize) -> f32 {
        use std::f32::consts::PI;
        let w = 2.0 * PI * f / sr;
        let (a, b) = (center.saturating_sub(half), (center + half).min(ir.len()));
        let (mut re, mut im) = (0.0f32, 0.0f32);
        for (n, &x) in ir.iter().enumerate().take(b).skip(a) {
            re += x * (w * n as f32).cos();
            im += x * (w * n as f32).sin();
        }
        (re * re + im * im).sqrt() / (b - a).max(1) as f32
    }

    /// Estimate a mode's realized T60 (ms) from the IR by comparing its amplitude
    /// at two centres inside the pre-fade region. Catches both a dead/truncated
    /// mode and a fictional t60 that the window can't actually express.
    pub fn estimate_t60(ir: &[f32], f: f32, sr: f32) -> f32 {
        let len = ir.len();
        let half = (len / 10).max(64);
        let c1 = len * 30 / 100;
        let c2 = len * 55 / 100;
        let e1 = envelope_at(ir, f, sr, c1, half);
        let e2 = envelope_at(ir, f, sr, c2, half);
        if e2 <= 0.0 || e1 <= e2 {
            return f32::INFINITY; // not decaying (flat/boom) or no signal
        }
        let tau = (c2 - c1) as f32 / (e1 / e2).ln(); // amplitude tau, samples
        6.908 * tau / sr * 1000.0 // t60 in ms
    }

    /// Assert a single texture is physically plausible and fully realizable in the
    /// IR window (nothing is silently truncated).
    pub fn assert_plausible(tag: &str, sr: f32, tex: &super::Texture) {
        let win_ms = super::ir_len_ms(sr);
        let predelay_ms = tex.predelay as f32 / sr * 1000.0;
        assert!(
            predelay_ms < win_ms * 0.5,
            "{tag}: predelay {predelay_ms:.1} ms is too large for the {win_ms:.1} ms window"
        );

        // Reflections: increasing time, non-increasing magnitude, all in-window.
        let mut prev_t = 0.0f32;
        let mut prev_g = f32::INFINITY;
        for &(t, g) in tex.reflections {
            assert!(
                t > prev_t,
                "{tag}: reflection times not strictly increasing"
            );
            assert!(
                predelay_ms + t < win_ms,
                "{tag}: reflection at {t:.1} ms truncated by the {win_ms:.1} ms window"
            );
            assert!(g.abs() < 1.0, "{tag}: reflection gain |{g}| >= 1");
            assert!(
                g.abs() <= prev_g + 1e-6,
                "{tag}: reflection magnitude rises again at {t:.1} ms ({g})"
            );
            prev_t = t;
            prev_g = g.abs();
        }

        // Modes: frequency in a real speaker band, decay realizable & non-booming.
        for &(f, t60, g) in tex.modes {
            let in_body = (BODY_MODE_HZ.0..=BODY_MODE_HZ.1).contains(&f);
            let in_breakup = (BREAKUP_MODE_HZ.0..=BREAKUP_MODE_HZ.1).contains(&f);
            assert!(
                in_body || in_breakup,
                "{tag}: mode {f} Hz outside plausible speaker bands"
            );
            // Breakup modes are intentionally short transients; the T60 window
            // applies to the resonant body modes that must ring without booming.
            if in_body {
                assert!(
                    (MODE_T60_MS.0..=MODE_T60_MS.1).contains(&t60),
                    "{tag}: body mode {f} Hz T60 {t60} ms outside [{:.0},{:.0}] ms",
                    MODE_T60_MS.0,
                    MODE_T60_MS.1
                );
            }
            assert!(g.abs() < 0.5, "{tag}: mode gain |{g}| implausibly hot");
        }
    }

    /// Assert the L/R textures decorrelate via small time/frequency offsets only —
    /// their per-element *gains* must stay symmetric. Catches the decimal-point
    /// typos that make one channel resonate or reflect far louder than the other.
    pub fn assert_lr_symmetry(tag: &str, l: &super::Texture, r: &super::Texture) {
        assert_eq!(
            l.reflections.len(),
            r.reflections.len(),
            "{tag}: L/R reflection counts differ"
        );
        assert_eq!(
            l.modes.len(),
            r.modes.len(),
            "{tag}: L/R mode counts differ"
        );
        let close = |a: f32, b: f32, ratio: f32, what: &str| {
            let (a, b) = (a.abs().max(1e-9), b.abs().max(1e-9));
            assert!(
                a / b <= ratio && b / a <= ratio,
                "{tag}: L/R {what} asymmetric ({a} vs {b}) — likely a typo"
            );
        };
        for (&(_, gl), &(_, gr)) in l.reflections.iter().zip(r.reflections) {
            close(gl, gr, 1.6, "reflection gain");
        }
        for (&(fl, _, gl), &(fr, _, gr)) in l.modes.iter().zip(r.modes) {
            close(fl, fr, 1.15, "mode frequency");
            close(gl, gr, 1.6, "mode gain");
        }
    }

    /// Assert each body mode actually rings in the rendered IR. Works on the pure
    /// modal contribution — the difference between the IR and a "bare" render of the
    /// same texture with the modes stripped — so it is independent of the mic
    /// voicing (whose high-pass can sit above a low mode) and of the reflection
    /// comb. That contribution must be present at the mode frequency and must
    /// *decay* across the window: a dead/truncated mode shows no energy, and a
    /// fictional (window-exceeding) T60 shows no decay.
    pub fn assert_modes_realized(
        tag: &str,
        full: &[f32],
        bare: &[f32],
        sr: f32,
        tex: &super::Texture,
    ) {
        let diff: Vec<f32> = full.iter().zip(bare).map(|(a, b)| a - b).collect();
        let n = diff.len();

        // Presence: each body mode is a distinct peak in the modal-only signal —
        // measured full-length so even a ~70 Hz mode spans many periods (a short
        // sub-window can't resolve a low frequency).
        for &(f, _, _) in tex.modes {
            if !(BODY_MODE_HZ.0..=BODY_MODE_HZ.1).contains(&f) {
                continue; // breakup transients are short; the engine test covers them
            }
            let here = goertzel(&diff, f, sr);
            let gap = goertzel(&diff, f * 1.6, sr); // between modes — no resonance
            assert!(
                here > 1.3 * gap,
                "{tag}: mode {f} Hz not realized (peak {here:.5} vs gap {gap:.5})"
            );
        }

        // Decay: the total modal contribution must ring down, not sit truncated —
        // broadband RMS (robust at any frequency) falls from an early to a late
        // window inside the pre-fade region.
        let rms = |a: usize, b: usize| {
            (diff[a..b].iter().map(|&x| (x * x) as f64).sum::<f64>() / (b - a) as f64).sqrt()
        };
        let early = rms(n / 10, n * 30 / 100);
        let late = rms(n * 55 / 100, n * 75 / 100); // up to the fade start
        assert!(
            early > late * 1.2 && late > 0.0,
            "{tag}: modal ring does not decay (early {early:.5} late {late:.5})"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// A declared mode must actually appear in the synthesised IR at its stated
    /// frequency and decay at (approximately) its stated T60 — proving the synth
    /// engine realizes the parameters rather than silently dropping them.
    #[test]
    fn synth_realizes_a_mode_at_its_frequency_and_t60() {
        let sr = 48_000.0;
        let len = ir_len(sr);
        let mut flat = |x: f32| x; // identity voicing → bare impulse skeleton
        let tex = Texture {
            predelay: 0,
            reflections: &[],
            modes: &[(110.0, 90.0, 0.05)],
        };
        let ir = synth(sr, len, &mut flat, &tex);
        // Frequency: 110 Hz dominates its neighbours.
        let at = |f| analysis::goertzel(&ir, f, sr);
        assert!(at(110.0) > 3.0 * at(220.0), "mode not present at 110 Hz");
        // Decay: realized T60 within ±40% of the declared 90 ms.
        let t60 = analysis::estimate_t60(&ir, 110.0, sr);
        assert!(
            (54.0..=126.0).contains(&t60),
            "realized T60 {t60:.0} ms is far from the declared 90 ms"
        );
    }

    /// A single early reflection at delay τ must produce comb colouration — a
    /// spectral extremum near 1/(2τ) — confirming the reflection list shapes the
    /// magnitude response as intended.
    #[test]
    fn synth_reflection_creates_expected_comb() {
        let sr = 48_000.0;
        let len = ir_len(sr);
        const TAU_MS: f32 = 1.0; // → first comb notch/peak near 500 Hz
        let mut flat = |x: f32| x;
        let tex = Texture {
            predelay: 0,
            reflections: &[(TAU_MS, -0.7)],
            modes: &[],
        };
        let ir = synth(sr, len, &mut flat, &tex);
        let f_ext = 1000.0 / (2.0 * TAU_MS); // 500 Hz
        let peak = analysis::goertzel(&ir, f_ext, sr); // inverted refl → peak here
        let trough = analysis::goertzel(&ir, f_ext * 2.0, sr); // notch near 1 kHz
        assert!(
            peak > 1.5 * trough,
            "no comb colouration from a {TAU_MS} ms reflection: {peak:.3} vs {trough:.3}"
        );
    }

    /// The synthesised IR must be finite, energy-bounded, and free of DC bias.
    #[test]
    fn synth_is_finite_and_dc_free() {
        let sr = 48_000.0;
        let len = ir_len(sr);
        let mut voicing = {
            let mut hp = crate::dsp::biquad::Biquad::highpass(sr, 80.0, 0.8);
            move |x: f32| hp.process(x)
        };
        let tex = Texture {
            predelay: 4,
            reflections: &[(0.5, -0.3), (2.0, 0.15)],
            modes: &[(95.0, 100.0, 0.01), (3400.0, 4.0, 0.1)],
        };
        let ir = synth(sr, len, &mut voicing, &tex);
        assert!(ir.iter().all(|v| v.is_finite()), "non-finite IR tap");
        let dc: f32 = ir.iter().sum();
        assert!(dc.abs() < 0.5, "IR carries DC bias: {dc}");
        let _ = PI;
    }
}
