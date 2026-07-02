//! Measure and compare the steady-state frequency response of the built-in
//! cabs against an external IR (e.g. God's Cab), through the exact same
//! processing path the app uses (speaker drive + convolution + mic model).
//!
//!     cargo run --release --example cab_spectrum -- <path-to-ir.wav>

use rusty_amp::dsp::cab::{Cabinet, ExternalIrCab, MAX_IR_LEN, load_ir};
use rusty_amp::dsp::cab::{MarshallCab, MesaCab, OrangeCab};
use std::f32::consts::PI;

const SR: f32 = 48_000.0;
const AMP: f32 = 0.1; // quiet: stay in the linear region of the speaker drive

/// Steady-state RMS gain of a cab at one frequency (default knob settings).
fn tone_gain(cab: &mut dyn Cabinet, freq: f32) -> f32 {
    let n = (SR * 0.75) as usize;
    let warm = n / 3;
    let mut acc = 0.0f64;
    for i in 0..n {
        let x = (2.0 * PI * freq * i as f32 / SR).sin() * AMP;
        let (l, r) = cab.process(x, 0.5, 0.15, 0.15);
        if i >= warm {
            let m = 0.5 * (l + r);
            acc += f64::from(m * m);
        }
    }
    let rms = (acc / (n - warm) as f64).sqrt() as f32;
    rms / (AMP / 2.0_f32.sqrt())
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: cab_spectrum <ir.wav>");
    let ir = load_ir(&path, SR, MAX_IR_LEN).expect("failed to load IR");

    // Quarter-octave grid in the lows, sixth-octave above 1.5 kHz for the top end.
    let mut freqs: Vec<f32> = (0..19)
        .map(|i| 50.0 * 2.0_f32.powf(i as f32 / 4.0))
        .take_while(|&f| f < 1500.0)
        .collect();
    let mut f = 1500.0f32;
    while f < 10_000.0 {
        freqs.push(f);
        f *= 2.0_f32.powf(1.0 / 6.0);
    }

    let mut ext = ExternalIrCab::new(SR, ir);
    let mut mesa = MesaCab::new(SR);
    let mut marshall = MarshallCab::new(SR);
    let mut orange = OrangeCab::new(SR);

    println!(
        "{:>8} {:>9} {:>9} {:>9} {:>9}",
        "Hz", "extIR", "mesa", "marshall", "orange"
    );
    let db = |g: f32| 20.0 * g.max(1e-9).log10();
    for &f in &freqs {
        println!(
            "{f:>8.0} {:>9.1} {:>9.1} {:>9.1} {:>9.1}",
            db(tone_gain(&mut ext, f)),
            db(tone_gain(&mut mesa, f)),
            db(tone_gain(&mut marshall, f)),
            db(tone_gain(&mut orange, f)),
        );
    }
}
