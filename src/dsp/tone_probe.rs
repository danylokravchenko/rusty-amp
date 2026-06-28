//! TEMPORARY high-note analysis harness.
//! Run: `cargo test -q hi_probe -- --nocapture`

#[cfg(test)]
mod probe {
    use crate::dsp::amp::AmpBank;
    use crate::dsp::cab::CabBank;
    use crate::dsp::{AmpModel, CabModel};
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

    #[allow(clippy::too_many_arguments)]
    fn render(am: AmpModel, cm: CabModel, freq: f32, gain: f32) -> Vec<f32> {
        let mut amp = AmpBank::new(SR);
        let mut cab = CabBank::new(SR);
        let n = SR as usize;
        let warmup = n / 3;
        let mut out = Vec::with_capacity(n - warmup);
        for i in 0..n {
            let x = (2.0 * PI * freq * i as f32 / SR).sin() * 0.5;
            let a = amp.process(am, x, gain, 0.50, 0.45, 0.65, 0.50, 0.55);
            let (l, r) = cab.process(cm, a, 0.5, 0.15, 0.15);
            if i >= warmup {
                out.push(l + r);
            }
        }
        out
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|&x| x * x).sum::<f32>() / s.len() as f32).sqrt()
    }

    /// Low-level sine sweep → system magnitude response (amp ~linear at low drive),
    /// so we see the raw spectral tilt that makes some notes louder than others.
    #[test]
    fn freq_response() {
        let amps = [
            ("Marshall", AmpModel::Marshall, CabModel::Marshall),
            ("Mesa", AmpModel::Mesa, CabModel::Mesa),
            ("Randall", AmpModel::Randall, CabModel::Orange),
        ];
        let freqs = [
            82.0, 110.0, 165.0, 220.0, 330.0, 440.0, 660.0, 880.0, 1175.0, 1568.0, 2093.0,
        ];
        for (aname, am, cm) in amps {
            print!("{aname:>9} (dB rel 220Hz): ");
            let mut amp = AmpBank::new(SR);
            let mut cab = CabBank::new(SR);
            // tiny input → essentially linear path
            let mut levels = Vec::new();
            for &f in &freqs {
                amp = AmpBank::new(SR);
                cab = CabBank::new(SR);
                let n = SR as usize;
                let w = n / 3;
                let mut out = Vec::new();
                for i in 0..n {
                    let x = (2.0 * PI * f * i as f32 / SR).sin() * 0.02;
                    let a = amp.process(am, x, 0.30, 0.50, 0.45, 0.65, 0.50, 0.55);
                    let (l, r) = cab.process(cm, a, 0.5, 0.15, 0.15);
                    if i >= w {
                        out.push(l + r);
                    }
                }
                levels.push(goertzel(&out, f, SR));
            }
            let r = levels[3]; // 220 Hz reference
            for (f, l) in freqs.iter().zip(&levels) {
                print!("{:.0}:{:+.1} ", f, 20.0 * (l / r).log10());
            }
            println!();
        }
        // Cab-only tilt (feed sine straight into cab, no amp).
        for (aname, _am, cm) in amps {
            print!("{aname:>9} CAB-only:       ");
            let freqs2 = [110.0, 220.0, 440.0, 660.0, 880.0, 1175.0, 2093.0];
            let mut levels = Vec::new();
            for &f in &freqs2 {
                let mut cab = CabBank::new(SR);
                let n = SR as usize;
                let w = n / 3;
                let mut out = Vec::new();
                for i in 0..n {
                    let x = (2.0 * PI * f * i as f32 / SR).sin() * 0.1;
                    let (l, r) = cab.process(cm, x, 0.5, 0.15, 0.15);
                    if i >= w {
                        out.push(l + r);
                    }
                }
                levels.push(goertzel(&out, f, SR));
            }
            let r = levels[1];
            for (f, l) in freqs2.iter().zip(&levels) {
                print!("{:.0}:{:+.1} ", f, 20.0 * (l / r).log10());
            }
            println!();
        }
    }

    /// Power chords (root + fifth + octave) up the neck. Measures level evenness,
    /// sub-bass "fart", and how much the upper-harmonic hash dominates the chord
    /// body — the chord version of the high-note tilt problem.
    #[test]
    fn powerchord_probe() {
        // (name, root, fifth, octave)
        let chords = [
            ("E2", 82.41, 123.47, 164.81),
            ("G2", 98.0, 146.83, 196.0),
            ("A2", 110.0, 164.81, 220.0),
            ("C3", 130.81, 196.0, 261.63),
            ("E3", 164.81, 246.94, 329.63),
            ("A3", 220.0, 329.63, 440.0),
            ("D4", 293.66, 440.0, 587.33),
        ];
        let amps = [
            ("Marshall", AmpModel::Marshall, CabModel::Marshall),
            ("Mesa", AmpModel::Mesa, CabModel::Mesa),
            ("Randall", AmpModel::Randall, CabModel::Orange),
        ];
        for (gain, label) in [(0.30, "CLEANISH g=0.30"), (0.85, "HI-GAIN g=0.85")] {
            for (aname, am, cm) in amps {
                println!("\n===== {aname}  [{label}] =====");
                println!("{:>4} {:>8} {:>8} {:>8} {:>8}", "chord", "rms", "sub/body", "hi/body", "harsh");
                let mut rmss = Vec::new();
                for (cname, r, fifth, oct) in chords {
                    let mut amp = AmpBank::new(SR);
                    let mut cab = CabBank::new(SR);
                    let n = SR as usize;
                    let w = n / 3;
                    let mut out = Vec::new();
                    for i in 0..n {
                        let t = i as f32 / SR;
                        let x = ((2.0 * PI * r * t).sin()
                            + (2.0 * PI * fifth * t).sin()
                            + (2.0 * PI * oct * t).sin())
                            * 0.3;
                        let a = amp.process(am, x, gain, 0.50, 0.45, 0.65, 0.50, 0.55);
                        let (l, rr) = cab.process(cm, a, 0.5, 0.15, 0.15);
                        if i >= w {
                            out.push(l + rr);
                        }
                    }
                    // sub-bass below the root (difference-tone fart)
                    let sub = goertzel(&out, r * 0.5, SR) + goertzel(&out, r * 0.66, SR);
                    // chord body: root, fifth, octave
                    let body = goertzel(&out, r, SR) + goertzel(&out, fifth, SR)
                        + goertzel(&out, oct, SR);
                    // high hash: energy 1.5-4 kHz
                    let mut hi = 0.0;
                    let mut pf = 1500.0;
                    while pf < 4000.0 {
                        hi += goertzel(&out, pf, SR).powi(2);
                        pf *= 2.0_f32.powf(1.0 / 12.0);
                    }
                    let hi = hi.sqrt();
                    let mut harsh = 0.0;
                    let mut pf = 2000.0;
                    while pf < 5000.0 {
                        harsh += goertzel(&out, pf, SR).powi(2);
                        pf *= 2.0_f32.powf(1.0 / 12.0);
                    }
                    let bodymax = body.max(1e-9);
                    rmss.push(rms(&out));
                    println!(
                        "{cname:>4} {:>8.4} {:>8.2} {:>8.2} {:>8.3}",
                        rms(&out), sub / bodymax, hi / bodymax, harsh.sqrt()
                    );
                }
                let lo = rmss.iter().cloned().fold(f32::INFINITY, f32::min);
                let hi = rmss.iter().cloned().fold(0.0, f32::max);
                println!("  loudness spread across chords: {:.1}x", hi / lo);
            }
        }
    }

    #[test]
    fn hi_probe() {
        // High register, with a couple of mid refs for contrast.
        let notes = [
            ("G3", 196.0),
            ("E4", 329.63),
            ("A4", 440.0),
            ("C5", 523.25),
            ("E5", 659.25),
            ("A5", 880.0),
            ("C6", 1046.5),
            ("E6", 1318.5),
        ];
        let amps = [
            ("Marshall", AmpModel::Marshall, CabModel::Marshall),
            ("Mesa", AmpModel::Mesa, CabModel::Mesa),
            ("Randall", AmpModel::Randall, CabModel::Orange),
        ];
        for (gain, label) in [(0.22, "CLEAN g=0.22"), (0.65, "DEFAULT g=0.65")] {
            for (aname, am, cm) in amps {
                println!("\n===== {aname}  [{label}] =====");
                println!(
                    "{:>4} {:>7} {:>6} {:>6}  {:>5}{:>5}{:>5}{:>5}{:>5}{:>5}{:>5}  {:>6} {:>6}",
                    "note", "rms", "thd", "alias", "h1", "h2", "h3", "h4", "h5", "h6", "h7", "fizz%",
                    "harsh"
                );
                for (nname, f) in notes {
                    let out = render(am, cm, f, gain);
                    let h: Vec<f32> = (1..=8).map(|k| goertzel(&out, f * k as f32, SR)).collect();
                    let fund = h[0].max(1e-9);
                    let harm_e: f32 = h.iter().skip(1).map(|x| x * x).sum();
                    let thd = harm_e.sqrt() / fund;
                    // Inharmonic probes between harmonics → aliasing / IM hash.
                    let alias: f32 = [f * 1.5, f * 2.5, f * 3.5, f * 4.5]
                        .iter()
                        .map(|&pf| goertzel(&out, pf, SR).powi(2))
                        .sum::<f32>()
                        .sqrt()
                        / fund;
                    // Fizz: energy above 6 kHz.
                    let mut fizz = 0.0f32;
                    let mut total = 1e-12f32;
                    let mut pf = 80.0;
                    while pf < 14000.0 {
                        let g = goertzel(&out, pf, SR).powi(2);
                        total += g;
                        if pf > 6000.0 {
                            fizz += g;
                        }
                        pf *= 2.0_f32.powf(1.0 / 12.0);
                    }
                    let harsh = {
                        let mut e = 0.0;
                        let mut pf = 2000.0;
                        while pf < 5000.0 {
                            e += goertzel(&out, pf, SR).powi(2);
                            pf *= 2.0_f32.powf(1.0 / 12.0);
                        }
                        e.sqrt()
                    };
                    let n = |x: f32| x / fund;
                    println!(
                        "{nname:>4} {:>7.4} {:>6.2} {:>6.3}  {:>5.2}{:>5.2}{:>5.2}{:>5.2}{:>5.2}{:>5.2}{:>5.2}  {:>5.2}% {:>6.3}",
                        rms(&out), thd, alias,
                        n(h[0]), n(h[1]), n(h[2]), n(h[3]), n(h[4]), n(h[5]), n(h[6]),
                        100.0*fizz/total, harsh,
                    );
                }
            }
        }
    }
}
