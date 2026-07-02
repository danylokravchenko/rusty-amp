//! Nonlinear characterisation of the amp models — the measurements that a
//! frequency sweep can't show: gain compression, harmonic fingerprint, and
//! sag/bloom dynamics. Optionally loads an external Audio Unit (macOS) and
//! runs the identical probes through it for a side-by-side reference.
//!
//!     cargo run --release --example amp_analysis                # built-ins
//!     cargo run --release --example amp_analysis -- --list-au   # show AUs
//!     cargo run --release --example amp_analysis -- --au marshall
//!
//! Knobs are fixed at the test-suite defaults (gain 0.65, bass 0.5, mid 0.45,
//! treble 0.65, presence 0.5, master 0.5) so runs are comparable over time.

use rusty_amp::dsp::amp::AmpBank;
use rusty_amp::dsp::cab::CabBank;
use rusty_amp::dsp::{AmpModel, CabModel};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;
const PROBE_HZ: f32 = 220.0;

/// A mono processor under test: either a built-in amp model or a loaded AU.
trait Dut {
    fn name(&self) -> String;
    fn process(&mut self, x: f32) -> f32;
    fn reset(&mut self);
}

/// A built-in amp, optionally through its matching cab (mono-summed, default
/// mic settings) so it compares like-for-like with an AU that embeds a cab.
struct BuiltIn {
    model: AmpModel,
    bank: AmpBank,
    cab: Option<(CabModel, CabBank)>,
}

impl Dut for BuiltIn {
    fn name(&self) -> String {
        match &self.cab {
            Some((cm, _)) => format!("{:?} + {cm:?} cab", self.model),
            None => format!("{:?}", self.model),
        }
    }
    fn process(&mut self, x: f32) -> f32 {
        let a = self
            .bank
            .process(self.model, x, 0.65, 0.50, 0.45, 0.65, 0.50, 0.50);
        match &mut self.cab {
            Some((cm, cab)) => {
                let (l, r) = cab.process(*cm, a, 0.5, 0.15, 0.15);
                0.5 * (l + r)
            }
            None => a,
        }
    }
    fn reset(&mut self) {
        self.bank = AmpBank::new(SR);
        if let Some((_, cab)) = &mut self.cab {
            *cab = CabBank::new(SR);
        }
    }
}

#[cfg(target_os = "macos")]
struct AuDut {
    name: String,
    au: Box<dyn rusty_amp::dsp::StereoInsert>,
    desc: rusty_amp::host::au::DiscoveredAu,
}

#[cfg(target_os = "macos")]
impl Dut for AuDut {
    fn name(&self) -> String {
        format!("AU:{}", self.name)
    }
    fn process(&mut self, x: f32) -> f32 {
        let mut l = [x];
        let mut r = [x];
        self.au.process_block(&mut l, &mut r);
        0.5 * (l[0] + r[0])
    }
    fn reset(&mut self) {
        if let Ok((_, ins)) = rusty_amp::host::au::load(&self.desc, SR, 512) {
            self.au = ins;
        }
    }
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-9).log10()
}

fn goertzel(samples: &[f32], f: f32) -> f32 {
    let w = 2.0 * PI * f / SR;
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

/// Steady-state output of a sine at `amp_in` peak: warm up 0.25 s, keep 0.5 s.
fn render(dut: &mut dyn Dut, amp_in: f32) -> Vec<f32> {
    dut.reset();
    let warm = (SR * 0.25) as usize;
    let keep = (SR * 0.5) as usize;
    let mut out = Vec::with_capacity(keep);
    for i in 0..warm + keep {
        let x = (2.0 * PI * PROBE_HZ * i as f32 / SR).sin() * amp_in;
        let y = dut.process(x);
        if i >= warm {
            out.push(y);
        }
    }
    out
}

fn rms(s: &[f32]) -> f32 {
    (s.iter().map(|&x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}

/// Gain curve: output RMS and incremental gain vs input level.
fn gain_curve(dut: &mut dyn Dut) {
    println!("  gain curve (220 Hz):  in dBFS → out dB(rms), gain dB");
    for step in 0..8 {
        let in_db = -42.0 + 6.0 * step as f32;
        let a = 10f32.powf(in_db / 20.0);
        let out = rms(&render(dut, a));
        println!(
            "    {in_db:>6.0} → {:>7.1}   {:>6.1}",
            db(out),
            db(out) - db(a / 2f32.sqrt())
        );
    }
}

/// Harmonic fingerprint: h2..h8 relative to the fundamental, at two drives.
fn harmonics(dut: &mut dyn Dut) {
    println!("  harmonics rel. h1 (dB):     h2     h3     h4     h5     h6     h7     h8");
    for (label, in_db) in [("edge of breakup (−24)", -24.0f32), ("driven (−6)", -6.0)] {
        let out = render(dut, 10f32.powf(in_db / 20.0));
        let h1 = goertzel(&out, PROBE_HZ).max(1e-9);
        let hs: Vec<String> = (2..=8)
            .map(|k| format!("{:>6.1}", db(goertzel(&out, PROBE_HZ * k as f32) / h1)))
            .collect();
        println!("    {label:<24} {}", hs.join(" "));
    }
}

/// Sag/bloom: full-scale burst from silence; peak envelope early vs settled.
fn sag(dut: &mut dyn Dut) {
    dut.reset();
    let total = (SR * 0.8) as usize;
    let seg = (SR * 0.010) as usize; // 10 ms envelope segments
    let mut peaks = vec![0.0f32; total / seg];
    for i in 0..total {
        let x = (2.0 * PI * PROBE_HZ * i as f32 / SR).sin();
        let y = dut.process(x).abs();
        let s = i / seg;
        if s < peaks.len() && y > peaks[s] {
            peaks[s] = y;
        }
    }
    let attack = peaks[..3].iter().cloned().fold(0.0f32, f32::max);
    let dip = peaks[2..20].iter().cloned().fold(f32::INFINITY, f32::min);
    let settled = peaks[peaks.len() - 10..]
        .iter()
        .cloned()
        .fold(0.0f32, f32::max);
    println!(
        "  sag: attack {:.2} → dip {:.2} → settled {:.2}  (sag depth {:>4.1} dB, recovery {:>+4.1} dB)",
        attack,
        dip,
        settled,
        db(dip / attack),
        db(settled / dip)
    );
}

/// Full-rig frequency response at a driven level (−12 dBFS): shows the voicing
/// the player actually hears, tone stack + saturation tilt + cab.
fn spectrum(dut: &mut dyn Dut) {
    println!("  response @ −12 dBFS in (dB out):");
    let mut freqs: Vec<f32> = vec![];
    let mut f = 70.0f32;
    while f < 8000.0 {
        freqs.push(f);
        f *= 2.0_f32.powf(1.0 / 3.0);
    }
    let a = 10f32.powf(-12.0 / 20.0);
    for chunk in freqs.chunks(6) {
        let row: Vec<String> = chunk
            .iter()
            .map(|&f| {
                let n = (SR * 0.4) as usize;
                let warm = n / 2;
                dut.reset();
                let mut out = Vec::with_capacity(n - warm);
                for i in 0..n {
                    let x = (2.0 * PI * f * i as f32 / SR).sin() * a;
                    let y = dut.process(x);
                    if i >= warm {
                        out.push(y);
                    }
                }
                format!("{f:>5.0}:{:>6.1}", db(rms(&out)))
            })
            .collect();
        println!("    {}", row.join("  "));
    }
}

fn analyse(dut: &mut dyn Dut) {
    println!("── {} ──", dut.name());
    gain_curve(dut);
    harmonics(dut);
    sag(dut);
    spectrum(dut);
    println!();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    #[cfg(target_os = "macos")]
    {
        if args.iter().any(|a| a == "--list-au") {
            for au in rusty_amp::host::au::scan() {
                println!("{}", au.name);
            }
            return;
        }
        if let Some(i) = args.iter().position(|a| a == "--au") {
            let pat = args
                .get(i + 1)
                .expect("usage: --au <name-substring>")
                .to_lowercase();
            let found = rusty_amp::host::au::scan()
                .into_iter()
                .find(|a| a.name.to_lowercase().contains(&pat))
                .expect("no AU matches");
            let (_, ins) = rusty_amp::host::au::load(&found, SR, 512).expect("AU load failed");
            let mut dut = AuDut {
                name: found.name.clone(),
                au: ins,
                desc: found,
            };
            analyse(&mut dut);
        }
    }

    let with_cab = args.iter().any(|a| a == "--cab");
    for (model, cm) in [
        (AmpModel::Marshall, CabModel::Marshall),
        (AmpModel::Mesa, CabModel::Mesa),
        (AmpModel::Randall, CabModel::Orange),
        (AmpModel::Vox, CabModel::Marshall),
    ] {
        let mut dut = BuiltIn {
            model,
            bank: AmpBank::new(SR),
            cab: with_cab.then(|| (cm, CabBank::new(SR))),
        };
        analyse(&mut dut);
    }
}
