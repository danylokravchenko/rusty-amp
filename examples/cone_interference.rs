//! Analyse the multi-speaker (neighbour-cone) interference against a reference
//! capture — the measurements that show whether `ConeSpread`'s geometry-derived
//! arrivals look like the real thing:
//!
//!   • early-time envelope (0–2.5 ms, 0.1 ms bins): where the arrivals land and
//!     how strong they are relative to the direct sound;
//!   • echo-delay scan (cepstrum-style): correlate the fine dB spectrum with
//!     cos(2πfτ) over τ = 0.2–1.6 ms — a comb from an echo at delay τ shows up
//!     as a peak, giving the *dominant* early-echo delay and its strength;
//!   • low-mid ripple (dB std-dev per octave, 250 Hz–2.5 kHz): the audible
//!     footprint of the interference texture.
//!
//! The reference was recorded on a real 4×12, so its numbers *contain* the true
//! cone interference; the builtins are measured through the same effective-IR
//! capture path. The isolated `ConeSpread` comb is printed last so its share of
//! the texture is visible next to the total.
//!
//!     cargo run --release --example cone_interference -- <reference.wav>

use rusty_amp::dsp::cab::{
    Cabinet, ExternalIrCab, MAX_IR_LEN, MarshallCab, MesaCab, OrangeCab, cone_spread_response,
    load_ir,
};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;
const CAP_LEN: usize = 8192;

/// Effective mono IR of a cab through its real process path (low drive level so
/// the speaker nonlinearities are ~transparent), matching `cab_analysis`.
fn capture(cab: &mut dyn Cabinet) -> Vec<f32> {
    const A: f32 = 0.02;
    (0..CAP_LEN)
        .map(|i| {
            let (l, r) = cab.process(if i == 0 { A } else { 0.0 }, 0.5, 0.15, 0.15);
            0.5 * (l + r) / A
        })
        .collect()
}

fn goertzel(s: &[f32], f: f32) -> f32 {
    let w = 2.0 * PI * f / SR;
    let c = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for &x in s {
        let s0 = x + c * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let re = s1 - s2 * w.cos();
    let im = s2 * w.sin();
    (re * re + im * im).sqrt()
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-12).log10()
}

/// Early-time envelope: RMS per 0.1 ms bin over the 2.5 ms following the IR's
/// direct arrival (the capture path carries the FFT convolver's ~2.7 ms fixed
/// latency, so the window is anchored to the peak, not t = 0), in dB relative to
/// the loudest bin. Neighbour-cone arrivals (and the mics' own early
/// reflections) show up as ledges after the direct peak.
fn early_envelope(ir: &[f32]) -> Vec<f32> {
    let peak_i = ir
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map_or(0, |(i, _)| i);
    let start = peak_i.saturating_sub((SR * 0.0001) as usize);
    let bin = (SR * 0.0001) as usize; // 0.1 ms
    let bins = 25;
    let e: Vec<f32> = (0..bins)
        .map(|b| {
            let (a, z) = (
                (start + b * bin).min(ir.len()),
                (start + (b + 1) * bin).min(ir.len()),
            );
            (ir[a..z].iter().map(|x| x * x).sum::<f32>() / bin.max(1) as f32).sqrt()
        })
        .collect();
    let peak = e.iter().cloned().fold(1e-12, f32::max);
    e.iter().map(|&v| db(v / peak)).collect()
}

/// Fine dB spectrum on a linear grid (for the echo scan: a comb from an echo at
/// delay τ is periodic in *linear* frequency with period 1/τ).
fn fine_spectrum(ir: &[f32], f_lo: f32, f_hi: f32, step: f32) -> Vec<(f32, f32)> {
    let mut out = vec![];
    let mut f = f_lo;
    while f <= f_hi {
        out.push((f, db(goertzel(ir, f))));
        f += step;
    }
    out
}

/// Cepstrum-style echo scan: correlation of the (mean-removed) dB spectrum with
/// cos(2πfτ). Returns (τ_ms, strength_db-ish) for each probed delay.
fn echo_scan(spec: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mean = spec.iter().map(|s| s.1).sum::<f32>() / spec.len() as f32;
    let mut out = vec![];
    let mut tau = 0.0002f32; // 0.2 ms
    while tau <= 0.0016 {
        let (mut c, mut s) = (0.0f32, 0.0f32);
        for &(f, d) in spec {
            c += (d - mean) * (2.0 * PI * f * tau).cos();
            s += (d - mean) * (2.0 * PI * f * tau).sin();
        }
        let n = spec.len() as f32;
        out.push((tau * 1000.0, (c * c + s * s).sqrt() * 2.0 / n));
        tau += 0.00005; // 0.05 ms grid
    }
    out
}

/// Ripple per octave: std-dev of 24 log-spaced dB points.
fn ripple(ir: &[f32], fc: f32) -> f32 {
    let pts: Vec<f32> = (0..24)
        .map(|i| db(goertzel(ir, fc * 2.0_f32.powf((i as f32 / 24.0) - 0.5))))
        .collect();
    let mean = pts.iter().sum::<f32>() / pts.len() as f32;
    (pts.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / pts.len() as f32).sqrt()
}

fn report(name: &str, ir: &[f32]) {
    println!("── {name} ──");
    let env = early_envelope(ir);
    print!("  early envelope (0–2.5 ms, dB re peak):\n   ");
    for (i, v) in env.iter().enumerate() {
        print!("{v:>6.1}");
        if i % 10 == 9 {
            print!("\n   ");
        }
    }
    println!();

    // The comb lives in the band the arrivals actually cover (LP'd ~1.6 kHz).
    let spec = fine_spectrum(ir, 250.0, 2200.0, 20.0);
    let scan = echo_scan(&spec);
    let top = scan
        .iter()
        .cloned()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    println!(
        "  echo scan (0.2–1.6 ms): dominant τ = {:.2} ms, strength {:.2} dB",
        top.0, top.1
    );
    print!("   ");
    for (t, v) in &scan {
        print!("{:.2}:{v:>5.2} ", t);
    }
    println!();

    println!(
        "  low-mid ripple (dB std-dev): 250-500 {:.1} | 0.5-1k {:.1} | 1-2k {:.1} | 2-4k {:.1}",
        ripple(ir, 355.0),
        ripple(ir, 710.0),
        ripple(ir, 1400.0),
        ripple(ir, 2800.0),
    );
    println!();
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: cone_interference <reference.wav>");
    let ir = load_ir(&path, SR, MAX_IR_LEN).expect("load IR");
    let name = std::path::Path::new(&path)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    report(
        &format!("ref:{name}"),
        &capture(&mut ExternalIrCab::new(SR, ir)),
    );
    report("mesa", &capture(&mut MesaCab::new(SR)));
    report("marshall", &capture(&mut MarshallCab::new(SR)));
    report("orange", &capture(&mut OrangeCab::new(SR)));

    // The isolated neighbour-cone stage: its own comb, no mic IR underneath.
    let cs = cone_spread_response(SR, 1024);
    println!("── ConeSpread (isolated) ──");
    println!("  magnitude (dB) across the comb band:");
    print!("   ");
    let mut f = 200.0f32;
    while f <= 3200.0 {
        print!("{f:>5.0}:{:>5.1} ", db(goertzel(&cs, f)));
        f *= 2.0_f32.powf(0.25);
    }
    println!();
    let spec = fine_spectrum(&cs, 250.0, 2200.0, 20.0);
    let top = echo_scan(&spec)
        .into_iter()
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    println!(
        "  echo scan: dominant τ = {:.2} ms, strength {:.2} dB",
        top.0, top.1
    );
}
