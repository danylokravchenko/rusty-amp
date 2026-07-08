//! Nonlinear characterisation of the drive pedals (DS-1 Distortion, ML-2 Metal
//! Core) against the TS-808 as a "clean, musical" reference — the measurements
//! that reveal *why* a distortion sounds "dirty" or "unmusical" rather than just
//! its magnitude curve:
//!
//!   • harmonic fingerprint (h2..h10 rel. h1) — the voice, and how far up the
//!     overtone ladder energy extends (a long, hot ladder reads as harsh/fizzy)
//!   • single-note INHARMONIC fraction — the share of output energy that is NOT
//!     on a harmonic of the played note: aliasing + noise = the "dirt/grit"
//!   • INTERMOD (IMD) on real intervals — a power fifth and a major third played
//!     together. A nonlinearity makes sum/difference tones; on a just power fifth
//!     they mostly land on a sub-harmonic (musical), but on a third they scatter
//!     onto inharmonic bins — the "mud/beehive" that makes chords unmusical. This
//!     is the single best "melodic vs dirty" discriminator.
//!   • spectral centroid — where the energy's weight sits (brightness/harshness)
//!   • note shape — attack crest, choke, sustain, and whether the overtones sing
//!     through the decay or collapse into thump
//!   • decay-tail fizz — as a plucked note dies, does the residue stay harmonic
//!     or turn into a bed of inharmonic hiss?
//!
//!     cargo run --release --example drive_analysis
//!
//! Knobs are fixed at musically-typical settings so runs compare over time.

use rusty_amp::dsp::effects::{Distortion, MetalCore, TubeScreamer};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;

/// A mono drive pedal under test at a fixed knob setting.
trait Dut {
    fn name(&self) -> String;
    fn process(&mut self, x: f32) -> f32;
    fn reset(&mut self);
}

struct Ds1 {
    d: Distortion,
    drive: f32,
    tone: f32,
    level: f32,
}
impl Dut for Ds1 {
    fn name(&self) -> String {
        format!("DS-1 (drive {:.2}, tone {:.2})", self.drive, self.tone)
    }
    fn process(&mut self, x: f32) -> f32 {
        self.d.process(x, self.drive, self.tone, self.level)
    }
    fn reset(&mut self) {
        self.d = Distortion::new(SR);
    }
}

struct Ml2 {
    m: MetalCore,
    dist: f32,
    low: f32,
    high: f32,
    level: f32,
}
impl Dut for Ml2 {
    fn name(&self) -> String {
        format!(
            "ML-2 (dist {:.2}, L {:.2}, H {:.2})",
            self.dist, self.low, self.high
        )
    }
    fn process(&mut self, x: f32) -> f32 {
        self.m
            .process(x, self.dist, self.low, self.high, self.level)
    }
    fn reset(&mut self) {
        self.m = MetalCore::new(SR);
    }
}

struct Ts808 {
    t: TubeScreamer,
    drive: f32,
    tone: f32,
    level: f32,
}
impl Dut for Ts808 {
    fn name(&self) -> String {
        format!("TS-808 (drive {:.2}, tone {:.2})", self.drive, self.tone)
    }
    fn process(&mut self, x: f32) -> f32 {
        self.t.process(x, self.drive, self.tone, self.level)
    }
    fn reset(&mut self) {
        self.t = TubeScreamer::new(SR);
    }
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-12).log10()
}

/// Single-bin magnitude (peak amplitude of a sine reads ~its amplitude).
fn goertzel(samples: &[f32], f: f32) -> f32 {
    let w = 2.0 * std::f64::consts::PI * f as f64 / SR as f64;
    let coeff = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for &x in samples {
        let s0 = x as f64 + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let real = s1 - s2 * w.cos();
    let imag = s2 * w.sin();
    ((real * real + imag * imag).sqrt() / (samples.len() as f64 / 2.0)) as f32
}

fn rms(s: &[f32]) -> f32 {
    (s.iter().map(|&x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
}

/// Render a steady sine at `f0`, `amp` peak; warm up 0.25 s, keep 0.5 s.
fn render_sine(dut: &mut dyn Dut, f0: f32, amp: f32) -> Vec<f32> {
    dut.reset();
    let warm = (SR * 0.25) as usize;
    let keep = (SR * 0.5) as usize;
    let mut out = Vec::with_capacity(keep);
    for i in 0..warm + keep {
        let x = (2.0 * PI * f0 * i as f32 / SR).sin() * amp;
        let y = dut.process(x);
        if i >= warm {
            out.push(y);
        }
    }
    out
}

/// Harmonic fingerprint: h2..h10 relative to the fundamental, at two drives set
/// by the input level (the pedals' own drive knob is fixed in the DUT).
fn harmonics(dut: &mut dyn Dut, f0: f32) {
    println!(
        "  harmonics rel. h1 (dB) @ {f0:.0} Hz:   h2    h3    h4    h5    h6    h7    h8    h9   h10"
    );
    for (label, amp) in [("light (0.15)", 0.15f32), ("hot (0.5)", 0.5)] {
        let out = render_sine(dut, f0, amp);
        let h1 = goertzel(&out, f0).max(1e-9);
        let hs: Vec<String> = (2..=10)
            .map(|k| format!("{:>5.0}", db(goertzel(&out, f0 * k as f32) / h1)))
            .collect();
        println!("    {label:<14} {}", hs.join(" "));
    }
}

/// Single-note inharmonic fraction: 1 − (energy on the note's harmonics / total
/// energy). What's left is aliasing + noise — the "dirt/grit". Also reports the
/// spectral centroid (Hz) as a brightness/harshness proxy.
fn inharmonic_and_centroid(dut: &mut dyn Dut, f0: f32, amp: f32) -> (f32, f32) {
    let out = render_sine(dut, f0, amp);
    let total = rms(&out).powi(2);
    let harm_pow: f32 = (1..=40)
        .map(|k| f0 * k as f32)
        .take_while(|&f| f < SR * 0.48)
        .map(|f| goertzel(&out, f).powi(2) / 2.0)
        .sum();
    let inharm = (1.0 - harm_pow / total.max(1e-12)).clamp(0.0, 1.0);

    // Spectral centroid over a fine harmonic comb (weighted by |H|).
    let (mut num, mut den) = (0.0f32, 0.0f32);
    for k in 1..=40 {
        let f = f0 * k as f32;
        if f >= SR * 0.48 {
            break;
        }
        let m = goertzel(&out, f);
        num += f * m;
        den += m;
    }
    (inharm, num / den.max(1e-9))
}

/// Intermodulation on a two-note interval. Feed `f_lo` + `f_hi` together and
/// split the output spectrum into:
///   • musical  = energy on integer multiples of the interval's greatest common
///     divisor grid (the harmonics of both notes *and* the shared sub-harmonic
///     the ear fuses into a "chord")
///   • intermod = everything else on a fine bin grid: the sum/diff mud that
///     makes distortion sound unmusical on anything but a single note.
/// Returns intermod / (musical + intermod) as a percentage.
fn imd(dut: &mut dyn Dut, f_lo: f32, f_hi: f32, amp: f32) -> f32 {
    dut.reset();
    let warm = (SR * 0.25) as usize;
    let keep = (SR * 0.5) as usize;
    let mut out = Vec::with_capacity(keep);
    for i in 0..warm + keep {
        let t = i as f32 / SR;
        let x = amp * (0.5 * (2.0 * PI * f_lo * t).sin() + 0.5 * (2.0 * PI * f_hi * t).sin());
        let y = dut.process(x);
        if i >= warm {
            out.push(y);
        }
    }
    // Grid = harmonics of the notes' shared sub-harmonic (their approx GCD).
    // Round to the nearest ~1 Hz so equal-tempered ratios still share a grid.
    let grid = gcd_hz(f_lo, f_hi);
    let mut musical = 0.0f32;
    let mut total = 0.0f32;
    let mut f = grid;
    while f < SR * 0.45 {
        let m = goertzel(&out, f);
        let p = m * m;
        total += p;
        // A bin is "musical" if it is (near) a harmonic of either played note.
        let on_note = |fn0: f32| {
            let r = f / fn0;
            (r - r.round()).abs() < 0.03 && r >= 0.5
        };
        if on_note(f_lo) || on_note(f_hi) {
            musical += p;
        }
        f += grid;
    }
    let intermod = (total - musical).max(0.0);
    100.0 * intermod / total.max(1e-12)
}

/// Approx greatest-common-divisor frequency of two pitches, in Hz, for the IMD
/// bin grid. Uses the integer ratio of rounded frequencies.
fn gcd_hz(a: f32, b: f32) -> f32 {
    let (mut x, mut y) = (a.round() as u32, b.round() as u32);
    while y != 0 {
        let t = y;
        y = x % y;
        x = t;
    }
    (x.max(1)) as f32
}

// ── Note shape ──────────────────────────────────────────────────────────────

const NOTE_SECS: f32 = 0.6;

/// Deterministic plucked-string DI: damped harmonic series (higher partials die
/// faster) + a short pick click. Same generator the amp analysis uses.
fn pluck_di(f0: f32, peak: f32) -> Vec<f32> {
    let n = (SR * NOTE_SECS) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            let mut s = 0.0f32;
            for k in 1..=6u32 {
                let kf = k as f32;
                let tau = 0.5 / kf;
                s += (-t / tau).exp() / kf * (2.0 * PI * f0 * kf * t).sin();
            }
            let click = (-t / 0.004).exp() * (2.0 * PI * 2500.0 * t).sin() * 0.3;
            peak * (s + 0.3 * click)
        })
        .collect()
}

fn env_db(sig: &[f32], hop_ms: f32) -> Vec<f32> {
    let hop = (SR * hop_ms / 1000.0) as usize;
    sig.chunks(hop.max(1)).map(|c| db(rms(c))).collect()
}

/// Pluck a note through the DUT and report its shape + how clean the decay tail
/// stays (harmonic residue vs fizz) deep in the note.
fn note_shape(dut: &mut dyn Dut, f0: f32, name: &str) {
    let di = pluck_di(f0, 0.35);
    dut.reset();
    let out: Vec<f32> = di.iter().map(|&x| dut.process(x)).collect();

    let env = env_db(&out, 10.0);
    let attack = env[..5].iter().cloned().fold(f32::MIN, f32::max);
    let at = |ms: f32| env[((ms / 10.0) as usize).min(env.len() - 1)];

    let n120 = (SR * 0.12) as usize;
    let head = &out[..n120.min(out.len())];
    let crest = head.iter().fold(0.0f32, |m, &x| m.max(x.abs())) / rms(head).max(1e-9);

    // Inharmonic fraction of the late decay tail (300–500 ms): is the note
    // dying into clean overtones or into a fizz bed?
    let a = (SR * 0.30) as usize;
    let b = ((SR * 0.50) as usize).min(out.len());
    let tail = &out[a.min(b)..b];
    let harm: f32 = (1..=30)
        .map(|k| f0 * k as f32)
        .take_while(|&f| f < SR * 0.48)
        .map(|f| goertzel(tail, f).powi(2) / 2.0)
        .sum();
    let tail_fizz = (1.0 - harm / rms(tail).powi(2).max(1e-12)).clamp(0.0, 1.0);

    println!(
        "  pluck {name}: choke {:+5.1} dB @60ms · sustain {:+5.1} dB @300ms · crest {:.1}× · tail inharmonic {:>4.0}%",
        at(60.0) - attack,
        at(300.0) - attack,
        crest,
        100.0 * tail_fizz,
    );
}

fn analyse(dut: &mut dyn Dut) {
    println!("── {} ──", dut.name());
    harmonics(dut, 196.0); // G3, a common riff/lead note

    // Dirt: single-note inharmonic fraction + centroid at a musical level.
    println!("  single-note dirt (inharmonic %) and brightness (centroid Hz):");
    for &(f0, nm) in &[(82.41f32, "E2"), (196.0, "G3"), (440.0, "A4")] {
        let (ih, cen) = inharmonic_and_centroid(dut, f0, 0.4);
        println!(
            "    {nm:<4} {f0:>6.1} Hz → inharmonic {:>4.1}%   centroid {:>6.0} Hz",
            ih, cen
        );
    }

    // Melodic: intermod on a power fifth (should stay low) and a major third
    // (the real chord-mud stress test).
    println!("  intermod (% inharmonic of dyad output; lower = more melodic):");
    let fifth = imd(dut, 82.41, 123.47, 0.35); // E2 + B2 power chord
    let third = imd(dut, 196.0, 246.94, 0.35); // G3 + B3 major third
    println!("    power 5th (E2+B2) {fifth:>5.1}%      major 3rd (G3+B3) {third:>5.1}%");

    // Note shape + decay-tail fizz.
    note_shape(dut, 82.41, "E2");
    note_shape(dut, 196.0, "G3");
    println!();
}

fn main() {
    // DS-1 at a classic gritty rhythm setting.
    let mut ds1 = Ds1 {
        d: Distortion::new(SR),
        drive: 0.6,
        tone: 0.5,
        level: 0.6,
    };
    // ML-2 at a high-gain metal rhythm setting.
    let mut ml2 = Ml2 {
        m: MetalCore::new(SR),
        dist: 0.7,
        low: 0.5,
        high: 0.5,
        level: 0.6,
    };
    // TS-808 as the "musical/clean overdrive" reference.
    let mut ts = Ts808 {
        t: TubeScreamer::new(SR),
        drive: 0.5,
        tone: 0.6,
        level: 0.6,
    };

    analyse(&mut ts);
    analyse(&mut ds1);
    analyse(&mut ml2);
}
