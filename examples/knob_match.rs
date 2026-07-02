//! Find the Mic / Blend / Room knob settings that make each built-in cab best
//! match a reference IR's tonal balance (level-independent, third-octave grid).
//!
//!     cargo run --release --example knob_match -- <reference.wav>

use rusty_amp::dsp::cab::{
    Cabinet, ExternalIrCab, MAX_IR_LEN, MarshallCab, MesaCab, OrangeCab, load_ir,
};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;
const CAP_LEN: usize = 8192;

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

fn probe_freqs() -> Vec<f32> {
    let mut f = 80.0f32;
    let mut v = vec![];
    while f < 8000.0 {
        v.push(f);
        f *= 2.0_f32.powf(1.0 / 3.0);
    }
    v
}

/// Band magnitudes (dB) of a cab's effective IR at the given knobs.
fn bands(cab: &mut dyn Cabinet, mic: f32, blend: f32, room: f32) -> Vec<f32> {
    const A: f32 = 0.02;
    let mut mono = Vec::with_capacity(CAP_LEN);
    for i in 0..CAP_LEN {
        let x = if i == 0 { A } else { 0.0 };
        let (l, r) = cab.process(x, mic, blend, room);
        mono.push(0.5 * (l + r) / A);
    }
    probe_freqs()
        .iter()
        .map(|&f| 20.0 * goertzel(&mono, f).max(1e-9).log10())
        .collect()
}

/// Mean-removed RMS distance between two dB curves (level-independent).
fn dist(a: &[f32], b: &[f32]) -> f32 {
    let off = a.iter().zip(b).map(|(x, y)| x - y).sum::<f32>() / a.len() as f32;
    (a.iter()
        .zip(b)
        .map(|(x, y)| (x - y - off).powi(2))
        .sum::<f32>()
        / a.len() as f32)
        .sqrt()
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: knob_match <ir.wav>");
    let ir = load_ir(&path, SR, MAX_IR_LEN).expect("load IR");
    let mut ext = ExternalIrCab::new(SR, ir);
    let target = bands(&mut ext, 0.5, 0.0, 0.0);

    let grid: Vec<f32> = (0..=8).map(|i| i as f32 / 8.0).collect();
    let cabs: [(&str, Box<dyn Cabinet>); 3] = [
        ("mesa", Box::new(MesaCab::new(SR))),
        ("marshall", Box::new(MarshallCab::new(SR))),
        ("orange", Box::new(OrangeCab::new(SR))),
    ];
    println!("best knob match vs {path}");
    println!("  (knob units 0–10, as shown in the UI)\n");
    for (name, mut cab) in cabs {
        let mut best = (f32::INFINITY, 0.0, 0.0, 0.0);
        for &mic in &grid {
            for &blend in &grid {
                for &room in &grid {
                    let d = dist(&bands(cab.as_mut(), mic, blend, room), &target);
                    if d < best.0 {
                        best = (d, mic, blend, room);
                    }
                }
            }
        }
        println!(
            "  {name:<9} MIC {:>4.1}  BLEND {:>4.1}  ROOM {:>4.1}   (residual {:.1} dB rms)",
            best.1 * 10.0,
            best.2 * 10.0,
            best.3 * 10.0,
            best.0
        );
    }
}
