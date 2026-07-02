//! Extract a reference AU's cabinet+mic section as an IR: set AmpBypass so
//! only the (linear) cab/mic/mixer path runs, feed an impulse, and save the
//! response as a stereo .wav into the rusty-amp IR folder — ready to load
//! with <I> and A/B against the built-in cabs with <X>.
//!
//!     cargo run --release --example au_cab_extract -- jubilee
//!
//! A linearity check (two impulse amplitudes must scale identically) guards
//! against accidentally capturing a nonlinear path.
//!
//! AU hosting is macOS-only; this example is a no-op elsewhere.

#[cfg(target_os = "macos")]
const SR: f32 = 48_000.0;
#[cfg(target_os = "macos")]
const CAP: usize = 16_384; // ~340 ms, plenty for the room mics

#[cfg(target_os = "macos")]
fn capture(ins: &mut Box<dyn rusty_amp::dsp::StereoInsert>, amp: f32) -> (Vec<f32>, Vec<f32>) {
    let (mut l_out, mut r_out) = (Vec::with_capacity(CAP), Vec::with_capacity(CAP));
    let mut i = 0usize;
    while i < CAP {
        let len = 512.min(CAP - i);
        let mut l = vec![0.0f32; len];
        let mut r = vec![0.0f32; len];
        if i == 0 {
            l[0] = amp;
            r[0] = amp;
        }
        ins.process_block(&mut l, &mut r);
        l_out.extend_from_slice(&l);
        r_out.extend_from_slice(&r);
        i += len;
    }
    for v in l_out.iter_mut().chain(r_out.iter_mut()) {
        *v /= amp;
    }
    (l_out, r_out)
}

#[cfg(target_os = "macos")]
fn run() {
    let pat = std::env::args()
        .nth(1)
        .expect("usage: au_cab_extract <au-substring>")
        .to_lowercase();
    let found = rusty_amp::host::au::scan()
        .into_iter()
        .find(|a| a.name.to_lowercase().contains(&pat))
        .expect("no AU matches");
    let (mut loaded, mut ins) = rusty_amp::host::au::load(&found, SR, 512).expect("load");
    let bypass = loaded
        .params()
        .iter()
        .position(|p| p.name == "AmpBypass")
        .expect("no AmpBypass param — cab can't be isolated on this AU");
    loaded.set_param(bypass, 1.0);
    // One block to flush the param change through the audio path.
    let (mut l, mut r) = (vec![0.0f32; 512], vec![0.0f32; 512]);
    ins.process_block(&mut l, &mut r);

    let (l1, r1) = capture(&mut ins, 0.05);
    let (l2, _r2) = capture(&mut ins, 0.5);
    // Linearity: the normalized responses must match closely.
    let dot: f32 = l1.iter().zip(&l2).map(|(a, b)| a * b).sum();
    let (e1, e2): (f32, f32) = (
        l1.iter().map(|x| x * x).sum(),
        l2.iter().map(|x| x * x).sum(),
    );
    let corr = dot / (e1.sqrt() * e2.sqrt()).max(1e-12);
    assert!(
        corr > 0.999,
        "cab path is not linear (corr {corr:.4}) — capture invalid"
    );

    let energy: f32 = l1.iter().chain(&r1).map(|x| x * x).sum();
    println!(
        "captured {} taps, linearity corr {corr:.5}, energy {energy:.3}",
        l1.len()
    );

    let dir = dirs::home_dir().unwrap().join(".config/rusty-amp/irs");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!(
        "{}_cab.wav",
        found.name.replace([':', ' ', '/'], "_")
    ));
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SR as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut w = hound::WavWriter::create(&path, spec).expect("create wav");
    for (a, b) in l1.iter().zip(&r1) {
        w.write_sample(*a).unwrap();
        w.write_sample(*b).unwrap();
    }
    w.finalize().unwrap();
    println!("saved {}", path.display());
    println!("load it in rusty-amp with <I>, A/B against the built-in cab with <X>");
}

#[cfg(not(target_os = "macos"))]
fn run() {
    eprintln!("au_cab_extract is macOS-only (Audio Unit hosting).");
}

fn main() {
    run();
}
