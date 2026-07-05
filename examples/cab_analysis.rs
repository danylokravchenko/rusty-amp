//! Full cabinet characterisation vs a reference IR — every comparable sound
//! characteristic, not just the magnitude curve:
//!
//!   • band magnitudes (tonal balance)
//!   • body–pocket tilt: mean 400 Hz–1 kHz level over mean 1.25–2 kHz level —
//!     real captures hold their body *above* the mid pocket (positive tilt);
//!     a negative tilt is the "hollow at 700 Hz, poking at 1.5 kHz" inversion
//!     that reads as thin and boxy at once
//!   • per-band decay, early (punch vs bloom) *and* late (the 40–80 ms ring
//!     that reads as a cab still sounding after the note)
//!   • spectral ripple amount (dB std-dev) *and density* (extrema per octave —
//!     real captures carry dozens of fine comb features per octave; a few deep
//!     notches produce the same std-dev but sound hollow, not "real")
//!   • echo density over time (peak/RMS crest per window — discrete authored
//!     taps stay spiky where real reflections densify into a diffuse texture)
//!   • per-band group delay (dispersion — part of perceived depth)
//!   • stereo L/R correlation and mono-sum penalty (real captures are mono:
//!     corr 1.0, penalty 0 dB; heavy decorrelation is phasey width that smears
//!     the centre image)
//!   • time structure: peak delay, direct(<3 ms)/late energy ratio
//!   • worst ⅓-octave deviations from the reference mean (tuning targets)
//!   • nonlinear engagement: how much of the shared speaker-drive range each
//!     built-in amp actually exercises at plugin-default knobs — 0% engaged
//!     means the cab's "alive" stages never fire on real playing levels
//!
//!     cargo run --release --example cab_analysis -- [reference.wav …]
//!
//! Each cab's *effective* IR is captured through the real `Cabinet::process`
//! path at a low drive level (speaker nonlinearities ~transparent), default
//! mic knobs — the same path the reference .wav takes through `ExternalIrCab`.

use rusty_amp::dsp::AmpModel;
use rusty_amp::dsp::amp::AmpBank;
use rusty_amp::dsp::cab::{
    Cabinet, ExternalIrCab, MAX_IR_LEN, MarshallCab, MesaCab, OrangeCab, load_ir,
    speaker_drive_stats,
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

/// Complex DFT bin at `f` (for phase / group delay).
fn cdft(s: &[f32], f: f32) -> (f32, f32) {
    let w = 2.0 * PI * f / SR;
    let (mut re, mut im) = (0.0f32, 0.0f32);
    for (n, &x) in s.iter().enumerate() {
        re += x * (w * n as f32).cos();
        im -= x * (w * n as f32).sin();
    }
    (re, im)
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
    mags: Vec<f32>,      // dB per octave band
    tilt: f32,           // body (400–1k) − pocket (1.25–2k) mean dB
    decay: Vec<f32>,     // dB drop early(0–8ms) → late(15–40ms) per band
    late_ring: Vec<f32>, // dB drop 5–20ms → 40–80ms per band (small = rings on)
    ripple: Vec<f32>,    // dB std-dev per octave
    density: Vec<usize>, // prominent spectral extrema per octave
    crest: Vec<f32>,     // peak/RMS of |h| per time window (low = diffuse)
    gdelay: Vec<f32>,    // group delay (ms) per octave band
    third_oct: Vec<f32>, // ⅓-octave magnitudes (dB) for deviation targets
    corr: f32,           // broadband L/R correlation
    mono_db: f32,        // mono-sum energy penalty (0 dB when L/R identical)
    peak_ms: f32,        // time of |h| peak
    direct_ratio: f32,   // energy < 3 ms / total
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

/// Standard ⅓-octave centres, 80 Hz – 8 kHz.
const THIRD_OCT_HZ: [f32; 21] = [
    80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0, 500.0, 630.0, 800.0, 1000.0, 1250.0,
    1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0, 8000.0,
];
/// THIRD_OCT_HZ index ranges for the body plateau and the mid pocket.
const BODY_RANGE: std::ops::Range<usize> = 7..12; // 400 Hz – 1 kHz
const POCKET_RANGE: std::ops::Range<usize> = 12..15; // 1.25 – 2 kHz

/// Time windows (ms) for the echo-density crest profile.
const CREST_WINS: [(f32, f32, &str); 4] = [
    (0.0, 2.0, "0-2ms"),
    (2.0, 5.0, "2-5ms"),
    (5.0, 10.0, "5-10ms"),
    (10.0, 20.0, "10-20ms"),
];

/// Fine log-frequency grid resolution for ripple analysis.
const GRID_PER_OCT: usize = 48;
/// An extremum must deviate this far (dB) from the ⅓-oct-smoothed trend to count.
const EXTREMUM_DB: f32 = 0.5;

fn analyse(name: &str, l: &[f32], r: &[f32]) -> Report {
    let mono: Vec<f32> = l.iter().zip(r).map(|(a, b)| 0.5 * (a + b)).collect();
    // All time windows are taken relative to the first arrival, not t=0: the
    // FFT convolver adds a fixed block latency (~2.7 ms at 48 kHz) and the
    // room mics carry pre-delay, so absolute windows would start in silence.
    let maxabs = mono.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
    let onset = mono
        .iter()
        .position(|&x| x.abs() > 0.02 * maxabs)
        .unwrap_or(0);
    let onset_ms = onset as f32 / SR * 1000.0;
    let win = |a_ms: f32, b_ms: f32| {
        (
            onset + (SR * a_ms / 1000.0) as usize,
            (onset + (SR * b_ms / 1000.0) as usize).min(mono.len()),
        )
    };
    let n3 = win(0.0, 3.0);
    let e_direct: f32 = mono[n3.0.min(mono.len())..n3.1].iter().map(|x| x * x).sum();
    let e_total: f32 = mono.iter().map(|x| x * x).sum::<f32>().max(1e-12);
    let peak_i = mono
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .map_or(0, |(i, _)| i);

    // Broadband L/R correlation (image width; 1.0 = mono) and the energy the
    // mono sum loses to inter-channel cancellation (0 dB when identical).
    let (mut num, mut dl, mut dr) = (0.0f64, 0.0f64, 0.0f64);
    for (&a, &b) in l.iter().zip(r) {
        num += f64::from(a * b);
        dl += f64::from(a * a);
        dr += f64::from(b * b);
    }
    let corr = (num / (dl.sqrt() * dr.sqrt()).max(1e-12)) as f32;
    let mono_e: f64 = mono.iter().map(|&x| f64::from(x * x)).sum();
    let mono_db = (10.0 * (mono_e / (0.5 * (dl + dr)).max(1e-18)).log10()) as f32;

    // Fine log-frequency magnitude grid (63 Hz – 8 kHz, aligned with OCTS) and
    // its ⅓-octave-smoothed trend. Band magnitudes are averaged over the grid
    // rather than read from single DFT bins: on a jagged IR a single bin can
    // land in a comb notch and misreport the whole band by 10+ dB.
    let grid_db: Vec<f32> = (0..=(OCTS.len() * GRID_PER_OCT))
        .map(|k| {
            db(goertzel(
                &mono,
                63.0 * 2.0_f32.powf(k as f32 / GRID_PER_OCT as f32),
            ))
        })
        .collect();
    let half = GRID_PER_OCT / 6; // ±1/6 octave smoothing window
    let smooth: Vec<f32> = (0..grid_db.len())
        .map(|i| {
            let a = i.saturating_sub(half);
            let b = (i + half + 1).min(grid_db.len());
            grid_db[a..b].iter().sum::<f32>() / (b - a) as f32
        })
        .collect();

    let early = win(0.0, 8.0);
    let late = win(15.0, 40.0);
    let ring_a = win(5.0, 20.0);
    let ring_b = win(40.0, 80.0);
    let mut mags = vec![];
    let mut decay = vec![];
    let mut late_ring = vec![];
    let mut gdelay = vec![];
    for (o, &(fc, _)) in OCTS.iter().enumerate() {
        let seg = &grid_db[o * GRID_PER_OCT..(o + 1) * GRID_PER_OCT];
        mags.push(seg.iter().sum::<f32>() / seg.len() as f32);
        let e = band_seg(&mono, fc, early.0, early.1);
        let lt = band_seg(&mono, fc, late.0, late.1);
        decay.push(db(e) - db(lt));
        // Capped at 96 dB: a short/gated capture has a silent 40–80 ms window,
        // which would otherwise read as a nonsense few-hundred-dB drop.
        let ra = band_seg(&mono, fc, ring_a.0, ring_a.1);
        let rb = band_seg(&mono, fc, ring_b.0, ring_b.1);
        late_ring.push((db(ra) - db(rb)).min(96.0));
        // Group delay: −dφ/dω across a narrow interval around the band centre,
        // minus the arrival delay so only the IR's own dispersion remains.
        let df = (0.01 * fc).clamp(2.0, 20.0);
        let (r1, i1) = cdft(&mono, fc - df);
        let (r2, i2) = cdft(&mono, fc + df);
        let mut dphi = i2.atan2(r2) - i1.atan2(r1);
        while dphi > PI {
            dphi -= 2.0 * PI;
        }
        while dphi < -PI {
            dphi += 2.0 * PI;
        }
        gdelay.push(-dphi / (2.0 * PI * 2.0 * df) * 1000.0 - onset_ms);
    }

    // Ripple amount and density per octave: extrema of the deviation from the
    // ⅓-octave-smoothed trend — "how many comb features per octave",
    // independent of how deep they are.
    let dev: Vec<f32> = grid_db.iter().zip(&smooth).map(|(m, s)| m - s).collect();
    let mut ripple = vec![];
    let mut density = vec![];
    for o in 0..OCTS.len() {
        let seg = &dev[o * GRID_PER_OCT..(o + 1) * GRID_PER_OCT];
        let mean = seg.iter().sum::<f32>() / seg.len() as f32;
        let var = seg.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / seg.len() as f32;
        ripple.push(var.sqrt());
        let mut count = 0usize;
        for i in (o * GRID_PER_OCT).max(1)..((o + 1) * GRID_PER_OCT).min(dev.len() - 1) {
            let peak = dev[i] > dev[i - 1] && dev[i] >= dev[i + 1] && dev[i] > EXTREMUM_DB;
            let dip = dev[i] < dev[i - 1] && dev[i] <= dev[i + 1] && dev[i] < -EXTREMUM_DB;
            if peak || dip {
                count += 1;
            }
        }
        density.push(count);
    }

    // Echo density over time: crest (peak/RMS) of |h| per window. Discrete
    // authored taps stay spiky; real reflections densify toward a low crest.
    let crest = CREST_WINS
        .iter()
        .map(|&(a_ms, b_ms, _)| {
            let (a, b) = win(a_ms, b_ms);
            let seg = &mono[a.min(b)..b];
            let peak = seg.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
            let rms = (seg.iter().map(|&x| x * x).sum::<f32>() / seg.len().max(1) as f32).sqrt();
            peak / rms.max(1e-12)
        })
        .collect();

    // ⅓-octave magnitudes from the smoothed trend (bin-noise-free targets).
    let third_oct: Vec<f32> = THIRD_OCT_HZ
        .iter()
        .map(|&f| {
            let idx = ((f / 63.0).log2() * GRID_PER_OCT as f32).round() as usize;
            smooth[idx.min(smooth.len() - 1)]
        })
        .collect();
    let body = third_oct[BODY_RANGE].iter().sum::<f32>() / BODY_RANGE.len() as f32;
    let pocket = third_oct[POCKET_RANGE].iter().sum::<f32>() / POCKET_RANGE.len() as f32;

    Report {
        name: name.to_string(),
        mags,
        tilt: body - pocket,
        decay,
        late_ring,
        ripple,
        density,
        crest,
        gdelay,
        third_oct,
        corr,
        mono_db,
        peak_ms: peak_i as f32 / SR * 1000.0,
        direct_ratio: e_direct / e_total,
    }
}

fn print_table(
    title: &str,
    reports: &[&Report],
    width: usize,
    labels: &[&str],
    row: impl Fn(&Report) -> Vec<String>,
) {
    println!("\n{title}");
    print!("  {:<width$}", "");
    for label in labels {
        print!("{label:>9}");
    }
    println!();
    for rep in reports {
        print!("  {:<width$}", rep.name);
        for v in row(rep) {
            print!("{v:>9}");
        }
        println!();
    }
}

/// The built-in amps' output level vs the speaker-drive nonlinearity range —
/// the gain-staging contract the "alive" cab stages depend on.
fn print_engagement(width: usize) {
    // Pluck-enveloped low-E power chord at typical interface DI level (~0.3
    // peak): representative playing, not a unit-amplitude test sine.
    let n = (SR * 2.0) as usize;
    let mut di = vec![0.0f32; n];
    for k in 0..5 {
        let start = (SR * 0.38) as usize * k;
        for i in 0..(SR * 0.3) as usize {
            if start + i >= n {
                break;
            }
            let t = i as f32 / SR;
            let env = (-t * 6.0).exp();
            let s = (2.0 * PI * 82.41 * t).sin()
                + 0.7 * (2.0 * PI * 123.47 * t).sin()
                + 0.5 * (2.0 * PI * 164.81 * t).sin();
            di[start + i] += 0.30 * env * s;
        }
    }

    println!("\nNONLINEAR ENGAGEMENT — amp output into the shared speaker-drive stages");
    println!("  (thermal knee at envelope 0.35; motor droop/Doppler need |disp| ≳ 0.3.");
    println!("   0% engaged = power compression/growl/breakup never fire at this setting");
    println!("   and the cab plays back as a static IR)");
    println!(
        "  {:<width$}{:>9}{:>9}{:>11}{:>11}{:>11}",
        "", "rms", "peak", "thermal %", "comp dB", "|disp|"
    );
    for (name, model) in [
        ("marshall", AmpModel::Marshall),
        ("mesa", AmpModel::Mesa),
        ("randall", AmpModel::Randall),
    ] {
        // Plugin-default knobs (dsp::mod defaults) and everything cranked.
        for (setting, gain, master) in [("defaults", 0.75, 0.55), ("cranked", 1.0, 1.0)] {
            let mut bank = AmpBank::new(SR);
            let out: Vec<f32> = di
                .iter()
                .map(|&x| bank.process(model, x, gain, 1.0, 0.0, 0.65, 0.40, master))
                .collect();
            let rms = (out.iter().map(|&x| x * x).sum::<f32>() / out.len() as f32).sqrt();
            let peak = out.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
            let st = speaker_drive_stats(SR, &out);
            println!(
                "  {:<width$}{:>9.3}{:>9.3}{:>10.1}%{:>11.2}{:>11.3}",
                format!("{name} {setting}"),
                rms,
                peak,
                100.0 * st.thermal_engaged,
                st.max_compression_db,
                st.mean_abs_disp
            );
        }
    }
}

fn main() {
    let refs: Vec<String> = std::env::args().skip(1).collect();
    if refs.is_empty() {
        println!("(no reference .wav given — analysing built-ins only;");
        println!(" usage: cab_analysis <ref.wav> [more.wav…] for side-by-side)");
    }

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
    let n_refs = reports.len();
    let (l, r) = capture(&mut MesaCab::new(SR));
    reports.push(analyse("mesa", &l, &r));
    let (l, r) = capture(&mut MarshallCab::new(SR));
    reports.push(analyse("marshall", &l, &r));
    let (l, r) = capture(&mut OrangeCab::new(SR));
    reports.push(analyse("orange", &l, &r));

    let width = reports.iter().map(|r| r.name.len()).max().unwrap_or(8) + 2;
    let all: Vec<&Report> = reports.iter().collect();
    let oct_labels: Vec<&str> = OCTS.iter().map(|&(_, l)| l).collect();

    print_table(
        "MAGNITUDE (dB, per octave band)",
        &all,
        width,
        &oct_labels,
        |r| r.mags.iter().map(|v| format!("{v:>8.1}")).collect(),
    );
    println!("\nBODY–POCKET TILT (mean 400–1k − mean 1.25–2k, dB; real captures are positive)");
    for r in &all {
        println!("  {:<width$}{:>8.1}", r.name, r.tilt);
    }
    print_table(
        "DECAY early(0-8ms) → late(15-40ms) drop (dB from arrival; small = rings longer)",
        &all,
        width,
        &oct_labels,
        |r| r.decay.iter().map(|v| format!("{v:>8.1}")).collect(),
    );
    print_table(
        "LATE RING 5-20ms → 40-80ms drop (dB from arrival, capped 96 = silent/gated; small = still sounding late)",
        &all,
        width,
        &oct_labels,
        |r| r.late_ring.iter().map(|v| format!("{v:>8.1}")).collect(),
    );
    print_table(
        "RIPPLE (dB std-dev per octave; real captures are jagged)",
        &all,
        width,
        &oct_labels,
        |r| r.ripple.iter().map(|v| format!("{v:>8.1}")).collect(),
    );
    print_table(
        "RIPPLE DENSITY (extrema > 0.5 dB per octave; few + deep = hollow, many + fine = real)",
        &all,
        width,
        &oct_labels,
        |r| r.density.iter().map(|v| format!("{v:>8}")).collect(),
    );
    print_table(
        "ECHO DENSITY (peak/RMS crest per window from arrival; high late crest = sparse discrete taps)",
        &all,
        width,
        &CREST_WINS.iter().map(|&(_, _, l)| l).collect::<Vec<_>>(),
        |r| r.crest.iter().map(|v| format!("{v:>8.1}")).collect(),
    );
    print_table(
        "GROUP DELAY (ms per octave band; dispersion reads as depth)",
        &all,
        width,
        &oct_labels,
        |r| r.gdelay.iter().map(|v| format!("{v:>8.2}")).collect(),
    );
    println!("\nTIME / STEREO (real captures: corr 1.00, mono-sum 0.0 dB)");
    println!(
        "  {:<width$}{:>10}{:>14}{:>12}{:>14}",
        "", "L/R corr", "mono sum dB", "peak (ms)", "direct/total"
    );
    for r in &all {
        println!(
            "  {:<width$}{:>10.2}{:>14.2}{:>12.2}{:>14.2}",
            r.name, r.corr, r.mono_db, r.peak_ms, r.direct_ratio
        );
    }

    // Tuning targets: where each built-in strays furthest from the references.
    if n_refs > 0 {
        let ref_mean: Vec<f32> = (0..THIRD_OCT_HZ.len())
            .map(|i| {
                reports[..n_refs]
                    .iter()
                    .map(|r| r.third_oct[i])
                    .sum::<f32>()
                    / n_refs as f32
            })
            .collect();
        println!("\nWORST ⅓-OCT DEVIATIONS vs reference mean (tuning targets)");
        for r in &reports[n_refs..] {
            let mut devs: Vec<(f32, f32)> = r
                .third_oct
                .iter()
                .zip(&ref_mean)
                .enumerate()
                .map(|(i, (m, rm))| (THIRD_OCT_HZ[i], m - rm))
                .collect();
            devs.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
            print!("  {:<width$}", r.name);
            for (f, d) in devs.iter().take(4) {
                print!("  {d:>+5.1} dB @ {f:>4.0} Hz");
            }
            println!();
        }
    }

    print_engagement(width);
}
