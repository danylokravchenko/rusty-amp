use super::biquad::Biquad;

/// Boss DS-1 Distortion simulation.
///
/// Signal path:
///   DC block → input HP (150 Hz) → [2× OS: hard-clip gain stage] → tone control → level
///
/// DS-1 character:
///   • Hard clipping via asymmetric op-amp + diode stack → more aggressive than TS soft-clip
///   • 2× oversampling through the hard-clip stage eliminates aliasing from the sharp
///     waveform discontinuities — the primary "digital buzz" cause in the naive model
///   • Input HP raised to 150 Hz (was 100 Hz): prevents the ~82 Hz low-E fundamental
///     from entering the hard clipper and creating bass-range intermodulation products
///   • Active tone stack: LP + HP blend controlled by a single knob
pub struct Distortion {
    sr: f32,
    dc_block: Biquad,
    input_hp: Biquad,
    // 2× oversampling for the hard-clip stage (hard clippers alias aggressively at base rate)
    os_up_a: Biquad,
    os_up_b: Biquad,
    os_dn_a: Biquad,
    os_dn_b: Biquad,
    tone_lp: Biquad,
    tone_hp: Biquad,
    last_tone: f32,
}

impl Distortion {
    pub fn new(sr: f32) -> Self {
        let sr2 = sr * 2.0;
        let half_nyq = sr / 2.0;
        let mut d = Self {
            sr,
            dc_block: Biquad::highpass(sr, 10.0, 0.707),
            // 100 Hz: matches the real DS-1 schematic; the 2× OS now handles aliasing
            // so we don't need to raise this further (150 Hz was cutting guitar fundamentals)
            input_hp: Biquad::highpass(sr, 100.0, 0.707),
            os_up_a: Biquad::lowpass(sr2, half_nyq, 0.5412),
            os_up_b: Biquad::lowpass(sr2, half_nyq, 1.3066),
            os_dn_a: Biquad::lowpass(sr2, half_nyq, 0.5412),
            os_dn_b: Biquad::lowpass(sr2, half_nyq, 1.3066),
            tone_lp: Biquad::lowpass(sr, 500.0, 0.707),
            tone_hp: Biquad::highpass(sr, 2000.0, 0.707),
            last_tone: -1.0,
        };
        d.update_tone(0.5);
        d
    }

    fn update_tone(&mut self, tone: f32) {
        let lp_fc = 200.0 + tone * 2800.0;
        let hp_fc = 300.0 + tone * 4700.0;
        self.tone_lp = Biquad::lowpass(self.sr, lp_fc, 0.5);
        self.tone_hp = Biquad::highpass(self.sr, hp_fc, 0.5);
        self.last_tone = tone;
    }

    #[inline]
    pub fn process(&mut self, sample: f32, drive: f32, tone: f32, level: f32) -> f32 {
        if (tone - self.last_tone).abs() > 0.001 {
            self.update_tone(tone);
        }

        let x = self.dc_block.process(sample);
        let x = self.input_hp.process(x);

        let gain = 1.0 + drive * 60.0;

        // ── 2× oversampled hard-clip stage ────────────────────────────────────
        // Even sample
        let up = self.os_up_b.process(self.os_up_a.process(x * 2.0));
        let clipped = ds1_clip(up * gain) / gain.sqrt();
        let x_os = self.os_dn_b.process(self.os_dn_a.process(clipped));

        // Odd sample — maintain filter state, discard output
        let up_z = self.os_up_b.process(self.os_up_a.process(0.0));
        let clipped_z = ds1_clip(up_z * gain) / gain.sqrt();
        self.os_dn_b.process(self.os_dn_a.process(clipped_z));
        // ── end oversampled section ───────────────────────────────────────────

        let dark = self.tone_lp.process(x_os);
        let bright = self.tone_hp.process(x_os);
        let x = dark * (1.0 - tone) + bright * tone;

        x * level * 0.6
    }
}

/// DS-1 diode clipper: hard asymmetric clipping.
/// Positive side — single silicon diode (clips earlier at ~0.7 V).
/// Negative side — two diodes in series (clips later at ~1.4 V).
/// The asymmetry produces even harmonics; with 2× OS the aliased content
/// is now above the original Nyquist and gets removed by the downsampler.
#[inline]
fn ds1_clip(x: f32) -> f32 {
    if x >= 0.0 {
        x.min(0.7) + (x - 0.7).max(0.0) * 0.05
    } else {
        x.max(-1.4) + (x + 1.4).min(0.0) * 0.05
    }
}
