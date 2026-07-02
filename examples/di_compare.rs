//! DI-render comparison: run a real (or synthesized) guitar DI through the
//! built-in JCM800 rig and through a reference Audio Unit, then compare the
//! renders with playing-level metrics that sine probes can't see:
//!
//!   • long-term average spectrum (third-octave, level-normalized)
//!   • crest factor (transient preservation through the compression chain)
//!   • envelope punch (P95/median of the broadband envelope)
//!   • treble modulation depth (how much the 1.5–4 kHz band "breathes" with
//!     the low strings — intermodulation/growl under real playing)
//!
//! Both renders are written to the scratch dir as .wav for listening A/B.
//!
//!     cargo run --release --example di_compare -- --au jubilee [di.wav]
//!
//! With no DI file, a deterministic Karplus-Strong performance is synthesized
//! (palm-muted low-E chugs, power chords, a single-note lick) so runs are
//! reproducible and machines without a DI library still get the comparison.

use rusty_amp::dsp::amp::AmpBank;
use rusty_amp::dsp::cab::CabBank;
use rusty_amp::dsp::{AmpModel, CabModel};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;

// ── DI synthesis: Karplus-Strong strings ───────────────────────────────────────

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0
    }
}

/// Pluck one string into `out` at `t0` seconds. `mute` raises loop damping for
/// the palm-muted chug sound.
fn pluck(out: &mut [f32], t0: f32, f: f32, dur: f32, amp: f32, mute: bool, seed: u32) {
    let period = (SR / f) as usize;
    let mut buf: Vec<f32> = Vec::with_capacity(period);
    let mut rng = Lcg(seed);
    for _ in 0..period {
        buf.push(rng.next());
    }
    let damp = if mute { 0.955 } else { 0.9965 };
    let start = (t0 * SR) as usize;
    let n = (dur * SR) as usize;
    let mut idx = 0usize;
    let mut prev = 0.0f32;
    for i in 0..n {
        let cur = buf[idx];
        // KS loop: averaging lowpass + decay.
        buf[idx] = damp * 0.5 * (cur + prev);
        prev = cur;
        idx = (idx + 1) % period;
        if start + i < out.len() {
            // Short attack ramp avoids a click; body is the raw string.
            let env = (i as f32 / 48.0).min(1.0);
            out[start + i] += amp * env * cur;
        }
    }
}

/// A ~11 s deterministic performance: chugs, power chords, a lick.
fn synth_di() -> Vec<f32> {
    let mut out = vec![0.0f32; (SR * 11.0) as usize];
    let e2 = 82.41;
    let chord = [82.41, 123.47, 164.81]; // E5 power chord
    // Bar 1: four palm-muted chugs.
    for (k, &t) in [0.0f32, 0.35, 0.7, 1.05].iter().enumerate() {
        pluck(&mut out, t, e2, 0.30, 0.9, true, 7 + k as u32);
    }
    // Bar 2: open power chord, let ring.
    for (k, &f) in chord.iter().enumerate() {
        pluck(&mut out, 1.6, f, 2.4, 0.55, false, 40 + k as u32);
    }
    // Bar 3: chugs with an accent, chord stab on top.
    for (k, &(t, a)) in [(4.2f32, 0.9f32), (4.55, 0.6), (4.9, 0.9), (5.25, 1.0)]
        .iter()
        .enumerate()
    {
        pluck(&mut out, t, e2, 0.30, a, true, 70 + k as u32);
    }
    for (k, &f) in chord.iter().enumerate() {
        pluck(&mut out, 5.8, f, 1.2, 0.5, false, 100 + k as u32);
    }
    // Bar 4: single-note lick up the neck.
    for (k, &(t, f)) in [
        (7.3f32, 164.81f32),
        (7.65, 196.0),
        (8.0, 220.0),
        (8.35, 261.63),
    ]
    .iter()
    .enumerate()
    {
        pluck(&mut out, t, f, 0.5, 0.7, false, 130 + k as u32);
    }
    // Final ring-out chord.
    for (k, &f) in chord.iter().enumerate() {
        pluck(&mut out, 9.0, f, 1.9, 0.6, false, 160 + k as u32);
    }
    // Normalize to a hot DI peak.
    let peak = out.iter().fold(0.0f32, |m, &x| m.max(x.abs())).max(1e-9);
    for v in &mut out {
        *v *= 0.7 / peak;
    }
    out
}

fn load_di(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).expect("open DI wav");
    let spec = reader.spec();
    let ch = spec.channels as usize;
    let mono: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(Result::unwrap)
            .collect::<Vec<_>>()
            .chunks(ch)
            .map(|fr| fr.iter().sum::<f32>() / ch as f32)
            .collect(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 * scale)
                .collect::<Vec<_>>()
                .chunks(ch)
                .map(|fr| fr.iter().sum::<f32>() / ch as f32)
                .collect()
        }
    };
    if spec.sample_rate == SR as u32 {
        return mono;
    }
    // Linear resample to 48 kHz — fine for a comparison source.
    let ratio = spec.sample_rate as f32 / SR;
    let n = (mono.len() as f32 / ratio) as usize;
    (0..n)
        .map(|i| {
            let p = i as f32 * ratio;
            let (a, f) = (p as usize, p.fract());
            let b = (a + 1).min(mono.len() - 1);
            mono[a] * (1.0 - f) + mono[b] * f
        })
        .collect()
}

// ── Renders ─────────────────────────────────────────────────────────────────────

fn render_builtin(di: &[f32]) -> Vec<f32> {
    let mut amp = AmpBank::new(SR);
    let mut cab = CabBank::new(SR);
    di.iter()
        .map(|&x| {
            let a = amp.process(AmpModel::Marshall, x, 0.65, 0.50, 0.45, 0.65, 0.50, 0.50);
            let (l, r) = cab.process(CabModel::Marshall, a, 0.5, 0.15, 0.15);
            0.5 * (l + r)
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn render_au(di: &[f32], pat: &str) -> (String, Vec<f32>) {
    let found = rusty_amp::host::au::scan()
        .into_iter()
        .find(|a| a.name.to_lowercase().contains(&pat.to_lowercase()))
        .expect("no AU matches");
    let (_, mut ins) = rusty_amp::host::au::load(&found, SR, 512).expect("AU load");
    let mut out = Vec::with_capacity(di.len());
    for chunk in di.chunks(512) {
        let mut l = chunk.to_vec();
        let mut r = chunk.to_vec();
        ins.process_block(&mut l, &mut r);
        out.extend(l.iter().zip(&r).map(|(a, b)| 0.5 * (a + b)));
    }
    (found.name, out)
}

// ── Metrics ─────────────────────────────────────────────────────────────────────

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
    (re * re + im * im).sqrt() / (s.len() as f32 / 2.0)
}

fn db(x: f32) -> f32 {
    20.0 * x.max(1e-12).log10()
}

fn rms(s: &[f32]) -> f32 {
    (s.iter().map(|&x| x * x).sum::<f32>() / s.len() as f32).sqrt()
}

/// Long-term average spectrum on a third-octave grid, normalized to its mean.
/// Each band integrates energy over a semitone grid of probe bins — a single
/// centre bin would land on or between note partials and swing ±10 dB with
/// bin alignment rather than tone.
fn ltas(s: &[f32]) -> Vec<(f32, f32)> {
    let mut f = 70.0f32;
    let mut rows = vec![];
    while f < 8000.0 {
        let hi = f * 2.0_f32.powf(1.0 / 3.0);
        let mut probe = f;
        let mut e = 0.0f32;
        while probe < hi {
            e += goertzel(s, probe).powi(2);
            probe *= 2.0_f32.powf(1.0 / 12.0);
        }
        rows.push((f, 10.0 * e.max(1e-18).log10()));
        f = hi;
    }
    let mean = rows.iter().map(|r| r.1).sum::<f32>() / rows.len() as f32;
    rows.iter().map(|&(f, v)| (f, v - mean)).collect()
}

/// Broadband envelope (abs → 5 ms one-pole), for crest/punch metrics.
fn envelope(s: &[f32], ms: f32) -> Vec<f32> {
    let c = 1.0 - (-1.0 / (SR * ms / 1000.0)).exp();
    let mut env = 0.0f32;
    s.iter()
        .map(|&x| {
            env += c * (x.abs() - env);
            env
        })
        .collect()
}

fn percentile(sorted: &[f32], p: f32) -> f32 {
    sorted[((sorted.len() - 1) as f32 * p) as usize]
}

/// Treble modulation depth: envelope std/mean of the 1.5–4 kHz band — how much
/// the top breathes with the playing (IMD/growl under a real performance).
fn treble_mod_depth(s: &[f32]) -> f32 {
    use rusty_amp::dsp::biquad::Biquad;
    let mut hp = Biquad::highpass(SR, 1500.0, 0.707);
    let mut lp = Biquad::lowpass(SR, 4000.0, 0.707);
    let band: Vec<f32> = s.iter().map(|&x| lp.process(hp.process(x))).collect();
    let env = envelope(&band, 8.0);
    let mean = env.iter().sum::<f32>() / env.len() as f32;
    let var = env.iter().map(|&e| (e - mean).powi(2)).sum::<f32>() / env.len() as f32;
    var.sqrt() / mean.max(1e-9)
}

fn report(name: &str, s: &[f32]) -> (Vec<(f32, f32)>, f32, f32, f32, f32) {
    let spec = ltas(s);
    let r = rms(s);
    let peak = s.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
    let crest = db(peak) - db(r);
    let mut env = envelope(s, 5.0);
    env.sort_by(f32::total_cmp);
    let punch = percentile(&env, 0.95) / percentile(&env, 0.5).max(1e-9);
    let md = treble_mod_depth(s);
    println!(
        "  {name:<28} rms {:>6.1} dB   crest {crest:>4.1} dB   punch(P95/med) {punch:>4.1}   treble-mod {md:>4.2}",
        db(r)
    );
    (spec, r, crest, punch, md)
}

fn save(path: &std::path::Path, s: &[f32]) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SR as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(path, spec).expect("create wav");
    for &x in s {
        w.write_sample(x).expect("write");
    }
    w.finalize().expect("finalize");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let au_pat = args
        .iter()
        .position(|a| a == "--au")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let di_path = args
        .iter().rfind(|a| !a.starts_with("--") && Some(*a) != au_pat.as_ref())
        .cloned();

    let di = match &di_path {
        Some(p) => {
            println!("DI: {p}");
            load_di(p)
        }
        None => {
            println!("DI: synthesized Karplus-Strong performance (11 s, deterministic)");
            synth_di()
        }
    };

    println!("\nrender metrics:");
    let ours = render_builtin(&di);
    let (spec_ours, ..) = report("Marshall + Marshall cab", &ours);

    let out_dir = std::env::temp_dir();
    save(&out_dir.join("di_ours.wav"), &ours);

    #[cfg(target_os = "macos")]
    if let Some(pat) = au_pat {
        let (name, theirs) = render_au(&di, &pat);
        let (spec_au, ..) = report(&format!("AU:{name}"), &theirs);
        save(&out_dir.join("di_reference.wav"), &theirs);

        println!("\nLTAS (third-octave, mean-normalized dB) and difference:");
        println!("  {:>6}  {:>7} {:>7} {:>7}", "Hz", "ours", "ref", "Δ");
        for (o, a) in spec_ours.iter().zip(&spec_au) {
            println!(
                "  {:>6.0}  {:>7.1} {:>7.1} {:>+7.1}",
                o.0,
                o.1,
                a.1,
                o.1 - a.1
            );
        }
        println!(
            "\nrenders written: {} and {}",
            out_dir.join("di_ours.wav").display(),
            out_dir.join("di_reference.wav").display()
        );
    }
}
