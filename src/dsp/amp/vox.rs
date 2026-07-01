use super::{
    Amplifier, Bloom, BrightCap, Cached, CathodeBias, FrontEnd, OutputTransformer, SpeakerLoad,
    ToneCache, VoiceBalance,
};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;
use crate::dsp::tonestack::{Components, ToneStack};

/// Vox AC30 (Top Boost) amplifier simulation.
///
/// Signal path:
///   DC block → input HP → [8× OS: stage-1 tube + inter-stage HP + stage-2 tube] → tone stack → power amp sag → presence
///
/// Character:
///   • 8× oversampling through the nonlinear gain stages keeps aliasing well above
///     the audible band, removing the harsh "digital" edge of stacked clippers
///   • Asymmetric 12AX7 waveshaper generates even harmonics (2nd, 4th) for warmth
///   • Dynamic grid-bias bloom adds touch sensitivity — the AC30's hallmark "give"
///   • Lighter inter-stage coupling HP (~450 Hz) than the JCM800 keeps more low-mid
///     body feeding the second stage, in keeping with the Top Boost's fuller voicing
///   • Bright cap is stronger and its corner higher than the Marshall's — the Top
///     Boost circuit is *itself* a treble-boost network, the source of the AC30 chime
///   • No global negative-feedback loop (unlike the Marshall/Mesa), so the power
///     stage sags more readily and the speaker-load interaction is more pronounced
pub struct Vox {
    sr: f32,
    // Pre-gain front end: DC block + input HP (base rate)
    front: FrontEnd,
    // 8× oversampling for the nonlinear section
    os: Oversampler8,
    // Bass cut before the first gain stage at 8× rate — prevents sub-bass from
    // entering the clipper and generating low-frequency IM products ("fart").
    pre_clip_hp: Biquad,
    // Inter-stage coupling HP between tube stages (at 8× rate)
    stage_hp: Biquad,
    // Dynamic preamp bloom
    bloom: Bloom,
    // Treble-bleed cap across the gain control — the Top Boost circuit's chime.
    bright: BrightCap,
    // Tone-stack change detector — recompute coefficients only when a knob moves.
    tone_cache: ToneCache,
    // Dynamic cathode-bias shift on the first triode stage (blocking-distortion
    // bloom / touch sensitivity), runs at 8× rate before the stage-1 waveshaper.
    cathode: CathodeBias,
    // Output-transformer core saturation + push-pull crossover (base rate).
    xfmr: OutputTransformer,
    // Passive FMV tone stack (base rate) — Vox values give a lighter mid scoop and
    // brighter treble than the Marshall's.
    tone: ToneStack,
    // Presence — power-amp NFB characteristic (base rate)
    presence_shelf: Biquad,
    presence_cache: Cached,
    // Structural voicing balance (base rate): low shelf restores low-mid body, high
    // shelf tames the tone stack's treble-forward tilt, so notes stay even across
    // the neck rather than the upper register blasting out.
    voice: VoiceBalance,
    // Output DC blocker (the asymmetric power clip leaves a small offset).
    out_hp: Biquad,
    // Power amp envelope follower (sag simulation)
    envelope: f32,
    // Power-amp ↔ speaker impedance interaction (dynamic low-end bloom).
    speaker: SpeakerLoad,
}

impl Vox {
    pub fn new(sr: f32) -> Self {
        let sr8 = sr * 8.0;
        let mut v = Self {
            sr,
            front: FrontEnd::new(sr, 70.0),
            os: Oversampler8::new(sr),
            // AC30 input coupling cap → sub-rumble cut at ~40 Hz, kept below the
            // 82 Hz low-E fundamental so the distorted bass string stays intact.
            pre_clip_hp: Biquad::highpass(sr8, 40.0, 0.707),
            // Inter-stage coupling lighter than the JCM800's (~450 Hz vs ~300 Hz):
            // the Top Boost's second stage runs hotter off a fuller low-mid feed,
            // part of why the AC30 stays fuller-sounding as it breaks up.
            stage_hp: Biquad::highpass(sr8, 450.0, 0.707),
            bloom: Bloom::new(sr, 10.0, 65.0),
            // Bright cap: the Top Boost's treble-boost network is itself the source
            // of the AC30's chime — stronger and a touch higher than the JCM800's.
            bright: BrightCap::new(sr, 2200.0, 0.22),
            tone_cache: ToneCache::new(),
            // First-stage cathode bias: slightly deeper than the JCM800's — the
            // AC30's Class A preamp is famously touch-sensitive — but recovers at
            // the same rate so note-to-note attack stays consistent.
            cathode: CathodeBias::new(sr8, 1.2, 40.0, 0.035, 1.0),
            // Output transformer: lighter drive and crossover than the Marshall's —
            // the AC30's EL84 pair breaks up smoothly rather than woolly.
            xfmr: OutputTransformer::new(sr, 175.0, 1.2, 0.03),
            tone: ToneStack::new(sr, Components::VOX),
            presence_shelf: Biquad::high_shelf(sr, 4500.0, 0.0),
            presence_cache: Cached::new(),
            voice: VoiceBalance::new(sr, 200.0, 2.5, 900.0, -5.0),
            out_hp: Biquad::highpass(sr, 12.0, 0.707),
            envelope: 0.0,
            // 2×12 Alnico Blue resonance ~85 Hz, tighter Q than the JCM800's 4×12
            // (ceramic-magnet-like sharper peak). No NFB loop means the power stage
            // sags more readily, so both the static and dynamic bloom amounts are
            // higher than the Marshall's — and the Alnico's extended top end gives
            // more inductive lift.
            speaker: SpeakerLoad::new(sr, 85.0, 1.3, 0.09, 0.45, 1.1),
        };
        v.update_tone_stack(0.5, 0.45, 0.65);
        v.update_presence(0.5);
        v
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        self.tone.update(bass, mid, treble);
    }

    fn update_presence(&mut self, presence: f32) {
        // Presence models the AC30's output-transformer NFB loop: shelf at 4.5 kHz,
        // a touch higher than the JCM800's 3.5 kHz — the AC30 lives higher up.
        self.presence_shelf = Biquad::high_shelf(self.sr, 4500.0, (presence - 0.5) * 12.0);
    }

    #[inline]
    fn power_amp(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        let coeff = if abs_x > self.envelope {
            1.0 - (-260.0 / self.sr).exp()
        } else {
            // No global NFB loop means the AC30's power stage sags more readily
            // than the Marshall's and holds the sag a touch longer — the springy,
            // elastic "give" players associate with a cranked Class A amp.
            1.0 - (-22.0 / self.sr).exp()
        };
        self.envelope += coeff * (abs_x - self.envelope);
        let sag = 1.0 / (1.0 + self.envelope * 0.75);
        tube_clip_asym(x * sag * 2.5) * 0.4
    }
}

impl Amplifier for Vox {
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
        // Bright cap across the gain pot: injects highs that then feed the clipper,
        // strongest at low gain (see BrightCap).
        let x = self.bright.process(x, gain);

        // Moderate max gain (32×) vs the Marshall's 39×/Mesa's 30× — the AC30 is a
        // lower-headroom vintage amp that chimes clean and breaks up gracefully
        // rather than a dedicated high-gain machine.
        let pregain = 1.0 + gain * 32.0;
        // Dynamic grid-bias offset (removed downstream by the inter-stage HP).
        let bias = self.bloom.follow(x) * 0.07;

        // ── 8× oversampled nonlinear section ──────────────────────────────────
        let up = self.os.upsample(x);
        let mut down = [0.0f32; 8];
        for (o, &u) in down.iter_mut().zip(up.iter()) {
            let u = self.pre_clip_hp.process(u); // cut sub-bass before clipping
            // Dynamic cathode bias shifts the operating point under hard drive
            // before the stage-1 waveshaper; the inter-stage HP strips its DC.
            let d = self.cathode.shift((u + bias) * pregain);
            let s = tube_clip_asym(d) / pregain.sqrt();
            let s = self.stage_hp.process(s);
            *o = tube_clip_asym(s * 3.0) / 3.0_f32.sqrt();
        }
        let x = self.os.downsample(down);
        // ── end oversampled section ───────────────────────────────────────────

        // Passive FMV tone stack (base rate — no aliasing risk)
        let x = self.tone.process(x);
        // Structural voicing balance: restore low-mid body, tame the upper-mid tilt.
        let x = self.voice.process(x);

        // Power amp: transformer sag + light saturation
        let x = self.power_amp(x);
        // Output transformer: low-frequency core saturation + push-pull crossover.
        let x = self.xfmr.process(x);

        // Speaker impedance interaction — dynamic low-end bloom driven by sag.
        let x = self.speaker.process(x, self.envelope);

        // Presence: output transformer NFB shelf
        let x = self.presence_shelf.process(x);

        // Output DC block: the asymmetric power-stage clip injects a small DC offset
        // and (like the Marshall) there is no power-section high-pass after it; a
        // real output transformer passes no DC, so strip it here before the trim.
        let x = self.out_hp.process(x);

        // Output trim: level-matched to the other three models so switching amps
        // mid-set doesn't jump the volume.
        x * master * 3.9
    }
}

/// Asymmetric 12AX7 triode waveshaper (see marshall.rs for rationale).
#[inline]
fn tube_clip_asym(x: f32) -> f32 {
    use std::f32::consts::FRAC_2_PI;
    if x >= 0.0 {
        FRAC_2_PI * x.atan()
    } else {
        // Negative half saturates faster; still asymptotically approaches -1
        FRAC_2_PI * (x * 1.1).atan()
    }
}
