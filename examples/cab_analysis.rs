//! Full cabinet characterisation vs a reference IR — every comparable sound
//! characteristic, not just the magnitude curve:
//!
//!   • band magnitudes (tonal balance)
//!   • per-band decay: early→late energy drop (the "waterfall" — punch vs bloom)
//!   • spectral ripple (comb texture per octave — real captures are jagged)
//!   • stereo L/R correlation (image width)
//!   • time structure: peak delay, direct(<3 ms)/late energy ratio
//!
//!     cargo run --release --example cab_analysis -- <reference.wav> [more.wav…]
//!
//! Each cab's *effective* IR is captured through the real `Cabinet::process`
//! path at a low drive level (speaker nonlinearities ~transparent), default
//! mic knobs — the same path the reference .wav takes through `ExternalIrCab`.

use rusty_amp::dsp::cab::{
    Cabinet, ExternalIrCab, MAX_IR_LEN, MarshallCab, MesaCab, OrangeCab, load_ir,
};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;
const CAP_LEN: usize = 8192; // > any IR tail incl. room pre-delay

/// Capture the effective stereo IR of a cab through its real process path.
fn capture(cab: &mut dyn Cabinet) -> (Vec<f32>, Vec<f32>) {
    const A: f32 = 0.02; // small: cone breakup/power compression ~transparent
    let (mut l, mut r) = (Vec::with_capacity(CAP_LEN), Vec::with_capacity(CAP_LEN));
    for i in 0..CAP_LEN {
        let x = if i == 0 { A } else { 0.0 };
        let (yl, yr) = cab.process(x, 0.5, 0.15, 0.15);
        l.push(yl / A);
        r.push(yr / A);
    }
    (l, r)
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

/// Windowed band magnitude: Goertzel over ir[a..b].
fn band_seg(ir: &[f32], f: f32, a: usize, b: usize) -> f32 {
    goertzel(&ir[a.min(ir.len())..b.min(ir.len())], f)
}

struct Report {
    name: String,
    mags: Vec<f32>,    // dB per octave band
    decay: Vec<f32>,   // dB drop early(0–8ms) → late(15–40ms) per band
    ripple: Vec<f32>,  // dB std-dev per octave
    corr: f32,         // broadband L/R correlation
    peak_ms: f32,      // time of |h| peak
    direct_ratio: f32, // energy < 3 ms / total
}

const OCTS: [(f32, &str); 7] = [
    (90.0, "63-125"),
    (180.0, "125-250"),
    (355.0, "250-500"),
    (710.0, "0.5-1k"),
    (1400.0, "1-2k"),
    (2800.0, "2-4k"),
    (5600.0, "4-8k"),
];

fn analyse(name: &str, l: &[f32], r: &[f32]) -> Report {
    let mono: Vec<f32> = l.iter().zip(r).map(|(a, b)| 0.5 * (a + b)).collect();
    let n3 = (SR * 0.003) as usize;
    let e_direct: f32 = mono[..n3.min(mono.len())].iter().map(|x| x * x).sum();
    let e_total: f32 = mono.iter().map(|x| x * x).sum::<f32>().max(1e-12);
    let peak_i = mono
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map_or(0, |(i, _)| i);

    // Broadband L/R correlation (image width; 1.0 = mono).
    let (mut num, mut dl, mut dr) = (0.0f64, 0.0f64, 0.0f64);
    for (&a, &b) in l.iter().zip(r) {
        num += f64::from(a * b);
        dl += f64::from(a * a);
        dr += f64::from(b * b);
    }
    let corr = (num / (dl.sqrt() * dr.sqrt()).max(1e-12)) as f32;

    let early = (0, (SR * 0.008) as usize);
    let late = ((SR * 0.015) as usize, (SR * 0.040) as usize);
    let mut mags = vec![];
    let mut decay = vec![];
    let mut ripple = vec![];
    for &(fc, _) in &OCTS {
        mags.push(db(goertzel(&mono, fc)));
        let e = band_seg(&mono, fc, early.0, early.1);
        let lt = band_seg(&mono, fc, late.0, late.1);
        decay.push(db(e) - db(lt));
        // Ripple: std-dev of fine-grid magnitudes (24 pts) across the octave.
        let pts: Vec<f32> = (0..24)
            .map(|i| db(goertzel(&mono, fc * 2.0_f32.powf((i as f32 / 24.0) - 0.5))))
            .collect();
        let mean = pts.iter().sum::<f32>() / pts.len() as f32;
        let var = pts.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / pts.len() as f32;
        ripple.push(var.sqrt());
    }
    Report {
        name: name.to_string(),
        mags,
        decay,
        ripple,
        corr,
        peak_ms: peak_i as f32 / SR * 1000.0,
        direct_ratio: e_direct / e_total,
    }
}

fn print_table(title: &str, reports: &[&Report], row: impl Fn(&Report) -> Vec<String>) {
    println!("\n{title}");
    print!("  {:<26}", "");
    for (_, label) in OCTS {
        print!("{label:>9}");
    }
    println!();
    for rep in reports {
        print!("  {:<26}", rep.name);
        for v in row(rep) {
            print!("{v:>9}");
        }
        println!();
    }
}

fn main() {
    let refs: Vec<String> = std::env::args().skip(1).collect();
    assert!(
        !refs.is_empty(),
        "usage: cab_analysis <ref.wav> [more.wav…]"
    );

    let mut reports: Vec<Report> = vec![];
    for path in &refs {
        let ir = load_ir(path, SR, MAX_IR_LEN).expect("load IR");
        let name = std::path::Path::new(path)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut cab = ExternalIrCab::new(SR, ir);
        let (l, r) = capture(&mut cab);
        reports.push(analyse(&format!("ref:{name}"), &l, &r));
    }
    let (l, r) = capture(&mut MesaCab::new(SR));
    reports.push(analyse("mesa", &l, &r));
    let (l, r) = capture(&mut MarshallCab::new(SR));
    reports.push(analyse("marshall", &l, &r));
    let (l, r) = capture(&mut OrangeCab::new(SR));
    reports.push(analyse("orange", &l, &r));

    let refs: Vec<&Report> = reports.iter().collect();
    print_table("MAGNITUDE (dB, per octave band)", &refs, |r| {
        r.mags.iter().map(|v| format!("{v:>8.1}")).collect()
    });
    print_table(
        "DECAY early(0-8ms) → late(15-40ms) drop (dB; small = rings longer)",
        &refs,
        |r| r.decay.iter().map(|v| format!("{v:>8.1}")).collect(),
    );
    print_table(
        "RIPPLE (dB std-dev per octave; real captures are jagged)",
        &refs,
        |r| r.ripple.iter().map(|v| format!("{v:>8.1}")).collect(),
    );
    println!("\nTIME / STEREO");
    println!(
        "  {:<26}{:>10}{:>12}{:>14}",
        "", "L/R corr", "peak (ms)", "direct/total"
    );
    for r in &reports {
        println!(
            "  {:<26}{:>10.2}{:>12.2}{:>14.2}",
            r.name, r.corr, r.peak_ms, r.direct_ratio
        );
    }
}
