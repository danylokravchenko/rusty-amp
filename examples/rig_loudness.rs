//! RMS loudness of each amp+cab rig on a matched DI (the loudness-matching
//! check at the level real playing hits, not a single sine).
use rusty_amp::dsp::amp::AmpBank;
use rusty_amp::dsp::cab::CabBank;
use rusty_amp::dsp::{AmpModel, CabModel};
const SR: f32 = 48_000.0;

fn main() {
    // Simple deterministic chug DI: low-E bursts (enough for level comparison).
    let mut di = vec![0.0f32; (SR * 3.0) as usize];
    for k in 0..8 {
        let start = (SR * 0.35) as usize * k;
        for i in 0..(SR * 0.25) as usize {
            let t = i as f32 / SR;
            let env = (-t * 9.0).exp();
            di[start + i] +=
                0.6 * env * (2.0 * std::f32::consts::PI * 82.41 * t).sin().signum() * 0.5;
        }
    }
    for (am, cm) in [
        (AmpModel::Marshall, CabModel::Marshall),
        (AmpModel::Mesa, CabModel::Mesa),
        (AmpModel::Randall, CabModel::Orange),
    ] {
        let mut amp = AmpBank::new(SR);
        let mut cab = CabBank::new(SR);
        let out: Vec<f32> = di
            .iter()
            .map(|&x| {
                let a = amp.process(am, x, 0.65, 0.50, 0.45, 0.65, 0.50, 0.50);
                let (l, r) = cab.process(cm, a, 0.5, 0.15, 0.15);
                0.5 * (l + r)
            })
            .collect();
        let rms = (out.iter().map(|&x| x * x).sum::<f32>() / out.len() as f32).sqrt();
        // Perceived-loudness proxy: the ear weights the mids far more than the
        // low end a chug rig is full of — measure the 300 Hz–5 kHz band too.
        let mut hp = rusty_amp::dsp::biquad::Biquad::highpass(SR, 300.0, 0.707);
        let mut lp = rusty_amp::dsp::biquad::Biquad::lowpass(SR, 5000.0, 0.707);
        let mid: Vec<f32> = out.iter().map(|&x| lp.process(hp.process(x))).collect();
        let mid_rms = (mid.iter().map(|&x| x * x).sum::<f32>() / mid.len() as f32).sqrt();
        let peak = out.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        println!(
            "{am:?} + {cm:?}: {:.1} dB rms   {:.1} dB mid-band (perceived)   peak {:.2}",
            20.0 * rms.log10(),
            20.0 * mid_rms.log10(),
            peak
        );
    }
}
