use super::{
    Amplifier, Bloom, BrightCap, Cached, CathodeBias, FrontEnd, OutputTransformer, SpeakerLoad,
    ToneCache, VoiceBalance,
};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;
use crate::dsp::tonestack::{Components, ToneStack};

/// Mesa/Boogie Dual Rectifier — Modern channel simulation.
///
/// Signal path:
///   DC block → input HP → [8× OS: stage-1 + HP + stage-2 + HP + stage-3 silicon] → tone stack → power amp → presence
///
/// Character:
///   • 8× oversampling through all three nonlinear stages keeps aliasing inaudible
///   • Asymmetric waveshapers on tube stages add even-harmonic warmth
///   • Dynamic grid-bias bloom adds touch sensitivity under hard playing
///   • Two inter-stage HPs (680 Hz and 1 kHz) prevent bass accumulation across stages
///   • Presence shelf at 4 kHz — Recto's presence is brighter/tighter than the JCM800
pub struct Mesa {
    sr: f32,
    front: FrontEnd,
    os: Oversampler8,
    // Pre-clip HP at 8× rate — cuts sub-bass before the first gain stage
    pre_clip_hp: Biquad,
    // Two inter-stage coupling HPs at 8× rate
    stage_hp_1: Biquad,
    stage_hp_2: Biquad,
    // Power-section subsonic cut (base rate, 4th-order = two cascaded biquads).
    // The asymmetric silicon stage generates strong difference tones on a low
    // power chord; this strips that inaudible sub-bass "fart" while leaving the
    // 82 Hz low-E fundamental intact.
    power_hp: Biquad,
    power_hp2: Biquad,
    bloom: Bloom,
    // Treble-bleed cap across the gain control (Recto bright cap).
    bright: BrightCap,
    // Dynamic cathode-bias shift on the first triode stage (8× rate).
    cathode: CathodeBias,
    // Output-transformer core saturation + push-pull crossover (base rate).
    xfmr: OutputTransformer,
    // Passive FMV tone stack (base rate) — Fender-type values for the Recto's
    // thicker low end and gentler scoop.
    tone: ToneStack,
    tone_cache: ToneCache,
    // Presence (base rate)
    presence_shelf: Biquad,
    presence_cache: Cached,
    // Structural voicing balance (base rate): the FENDER tone stack is treble-heavy
    // and peak-normalised, so the upper mids/treble sit well above the low mids;
    // left alone, notes higher up the neck blast out. The low shelf restores body
    // and the high shelf tames the tilt, flattening the across-the-neck response.
    // The body shelf's corner is matched to the upper inter-stage HP so the low-mid
    // level it restores has no gap.
    voice: VoiceBalance,
    // Silicon-rectifier sag envelope
    envelope: f32,
    // Power-amp ↔ speaker impedance interaction.
    speaker: SpeakerLoad,
}

impl Mesa {
    pub fn new(sr: f32) -> Self {
        let sr8 = sr * 8.0;
        let mut m = Self {
            sr,
            front: FrontEnd::new(sr, 60.0),
            os: Oversampler8::new(sr),
            // Recto input coupling HP at ~70 Hz — keeps sub-bass out of the gain
            // stages so they don't generate difference-tone mud, while preserving
            // the 82 Hz low-E fundamental.
            pre_clip_hp: Biquad::highpass(sr8, 70.0, 0.707),
            // Between stage 1 and 2: ~225 Hz. The original 680 Hz stripped the
            // fundamental of every note under ~1 kHz before the next clipper, so the
            // note's body vanished under a 10th-harmonic fizz. This corner keeps the
            // fundamental intact while still cutting enough low-mid in the cascade
            // that a palm-muted chug decays fast and stays percussive (the post-stack
            // `body` shelf restores the steady low-mid level for sustained notes).
            stage_hp_1: Biquad::highpass(sr8, 225.0, 0.707),
            // Between stage 2 and 3: ~320 Hz (silicon stage compresses harder, so a
            // tighter corner) — the chug-tightening cut, still below the fundamentals.
            stage_hp_2: Biquad::highpass(sr8, 320.0, 0.707),
            // Subsonic cut at 55 Hz, cascaded → 24 dB/oct. Lowered from 70 Hz so the
            // 82 Hz low-E fundamental passes with full weight (it was ~6 dB down),
            // while the 24 dB/oct slope still kills the inaudible difference-tone
            // fart well below it.
            power_hp: Biquad::highpass(sr, 55.0, 0.707),
            power_hp2: Biquad::highpass(sr, 55.0, 0.707),
            bloom: Bloom::new(sr, 8.0, 55.0),
            // Recto bright cap: corner a touch higher (~2.4 kHz) and lighter than the
            // JCM800 — the Recto is already a bright amp.
            bright: BrightCap::new(sr, 2400.0, 0.12),
            // First-stage cathode bias, same fast-charge / RC-recovery shape as the
            // JCM800; the Recto's tighter feel keeps the depth modest.
            cathode: CathodeBias::new(sr8, 1.5, 45.0, 0.055, 0.8),
            // Output transformer: the big Recto iron compresses the lows; corner
            // ~140 Hz, gentle drive and a trace of crossover.
            xfmr: OutputTransformer::new(sr, 140.0, 1.5, 0.04),
            tone: ToneStack::new(sr, Components::FENDER),
            tone_cache: ToneCache::new(),
            presence_shelf: Biquad::high_shelf(sr, 4000.0, 0.0),
            presence_cache: Cached::new(),
            voice: VoiceBalance::new(sr, 320.0, 9.0, 600.0, -9.5),
            envelope: 0.0,
            // Recto 4×12 resonance ~100 Hz; silicon supply sags less than a tube
            // rectifier, so a tight dynamic bloom. Trimmed (0.45→0.22) so palm-muted
            // chugs stay percussive instead of blooming on after the attack.
            speaker: SpeakerLoad::new(sr, 100.0, 1.0, 0.06, 0.22, 0.8),
        };
        m.update_tone_stack(0.5, 0.45, 0.65);
        m.update_presence(0.5);
        m
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        self.tone.update(bass, mid, treble);
    }

    fn update_presence(&mut self, presence: f32) {
        // Recto presence: 4 kHz (brighter/tighter than JCM800 3.5 kHz), ±6 dB
        self.presence_shelf = Biquad::high_shelf(self.sr, 4000.0, (presence - 0.5) * 12.0 + 2.0);
    }

    /// Silicon rectifier sag: tight attack (0.5 ms), moderate release (80 ms).
    #[inline]
    fn power_amp(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        let coeff = if abs_x > self.envelope {
            1.0 - (-1.0 / (0.0005 * self.sr)).exp()
        } else {
            1.0 - (-1.0 / (0.080 * self.sr)).exp()
        };
        self.envelope += coeff * (abs_x - self.envelope);
        // Silicon supply: stiff, true to the Recto (deep sag also scales the
        // asymmetric clip's drive down under sustained level, which would kill
        // the h2 growth that makes the amp touch-sensitive). The static drive
        // still sits off the plateau — see marshall.rs `power_amp`.
        let sag = 1.0 / (1.0 + self.envelope * 0.45);
        silicon_clip_asym(x * sag * 1.7) * 0.55
    }
}

impl Amplifier for Mesa {
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn process(
        &mut self,
        sample: f32,
        gain: f32,
        bass: f32,
        mid: f32,
        treble: f32,
        presence: f32,
        master: f32,
    ) -> f32 {
        if self.tone_cache.changed(bass, mid, treble) {
            self.update_tone_stack(bass, mid, treble);
        }
        if self.presence_cache.changed(presence) {
            self.update_presence(presence);
        }

        let x = self.front.process(sample);
        // Bright cap across the gain pot (see BrightCap).
        let x = self.bright.process(x, gain);

        let pregain = 1.0 + gain * 30.0;
        // Bias depth halved and bloom release shortened (above) so a note attacks
        // the same whether played alone or right after others — see marshall.rs.
        let bias = self.bloom.follow(x) * 0.09;

        // ── 8× oversampled nonlinear section ──────────────────────────────────
        // Per-stage drives kept moderate: three cascaded clippers multiply harmonic
        // content fast, and the old ×5/×3 inter-stage gains pushed the spectrum so
        // high that the played note was buried under its own overtones. ×2.6/×2.0
        // still saturates hard at high gain but lets the fundamental lead.
        // Pregain split across the three stages (see marshall.rs): the Recto
        // keeps a hotter final silicon stage than the tube amps — its modern
        // aggression — but no single stage runs deep on its plateau any more.
        let g1 = pregain.powf(0.62) * 1.4;
        let g2 = pregain.powf(0.22) * 1.8;
        let g3 = (pregain / (pregain.powf(0.62) * pregain.powf(0.22))) * 1.3;
        let up = self.os.upsample(x);
        let mut down = [0.0f32; 8];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            let u = self.pre_clip_hp.process(u); // cut sub-bass before clipping
            // Dynamic cathode bias on stage 1 (DC removed by the inter-stage HP).
            let d = self.cathode.shift((u + bias) * g1);
            let s = tube_clip_asym(d) / g1.sqrt();
            let s = self.stage_hp_1.process(s);
            let s = tube_clip_asym(s * g2) / g2.sqrt();
            let s = self.stage_hp_2.process(s);
            *o = silicon_clip_asym(s * g3) / g3.sqrt();
        }
        let x = self.os.downsample(down);
        // ── end oversampled section ───────────────────────────────────────────

        let x = self.tone.process(x);
        // Structural voicing balance: restore low-mid body, tame the upper-mid tilt.
        let x = self.voice.process(x);

        // Subsonic cut before the power stage so the silicon clipper can't fold
        // sub-bass into difference-tone mud.
        let x = self.power_hp.process(x);
        let x = self.power_amp(x);
        // Output transformer: low-frequency core saturation + push-pull crossover.
        let x = self.xfmr.process(x);
        let x = self.speaker.process(x, self.envelope);
        // Second subsonic stage after the asymmetric clipper, which regenerates a
        // low difference-tone "fart" from the chord's intervals.
        let x = self.power_hp2.process(x);
        let x = self.presence_shelf.process(x);

        // Output trim: level-match the Recto to the hotter solid-state Randall so
        // switching amp models doesn't produce a volume jump.
        x * master * 18.0
    }
}

/// Asymmetric 12AX7 triode waveshaper (see marshall.rs for rationale).
#[inline]
fn tube_clip_asym(x: f32) -> f32 {
    use std::f32::consts::FRAC_2_PI;
    if x >= 0.0 {
        FRAC_2_PI * x.atan()
    } else {
        FRAC_2_PI * (x * 1.1).atan()
    }
}

/// Asymmetric silicon diode clipper.
///
/// Positive half: 1 - e^{-x} (forward-biased exponential).
/// Negative half: atan-based with 1.1× input scale — models the different
/// reverse-bias characteristic of real junction diodes (harder knee on neg swing).
#[inline]
fn silicon_clip_asym(x: f32) -> f32 {
    // Both halves have unity slope at zero — silicon is *linear* until it nears
    // a rail, so quiet playing passes clean (this is what lets the amp's h2 grow
    // with picking strength instead of idling at a static floor). The asymmetry
    // lives in the rails: the positive half saturates exponentially toward +1,
    // the negative half clamps harder toward −0.79 — even harmonics appear only
    // once the stage is actually driven.
    if x >= 0.0 {
        1.0 - (-x).exp()
    } else {
        (2.0 * x).atan() / 2.0
    }
}
