use super::{
    Amplifier, Bloom, BrightCap, Cached, CathodeBias, FrontEnd, OutputTransformer, SpeakerLoad,
    ToneCache, VoiceBalance,
};
use crate::dsp::biquad::Biquad;
use crate::dsp::oversample::Oversampler8;
use crate::dsp::tonestack::{Components, ToneStack};

/// Marshall JCM800 amplifier simulation.
///
/// Signal path:
///   DC block → input HP → [8× OS: stage-1 tube + inter-stage HP + stage-2 tube] → tone stack → power amp sag → presence
///
/// Character:
///   • 8× oversampling through the nonlinear gain stages keeps aliasing well above
///     the audible band, removing the harsh "digital" edge of stacked clippers
///   • Asymmetric 12AX7 waveshaper generates even harmonics (2nd, 4th) for warmth
///   • Dynamic grid-bias bloom adds touch sensitivity under hard playing
///   • Inter-stage coupling HP at ~720 Hz (JCM800 22 nF coupling cap) tightens low-end
///   • Presence shelf in the power-amp NFB loop adds air and cut at 3.5 kHz
pub struct Marshall {
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
    // Treble-bleed cap across the gain control (sparkle at low gain).
    bright: BrightCap,
    // Tone-stack change detector — recompute coefficients only when a knob moves.
    tone_cache: ToneCache,
    // Dynamic cathode-bias shift on the first triode stage (blocking-distortion
    // bloom / touch sensitivity), runs at 8× rate before the stage-1 waveshaper.
    cathode: CathodeBias,
    // Output-transformer core saturation + push-pull crossover (base rate).
    xfmr: OutputTransformer,
    // Passive FMV tone stack (base rate) — bass/mid/treble interact like the real
    // JCM800 network, with the characteristic mid scoop.
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

impl Marshall {
    pub fn new(sr: f32) -> Self {
        let sr8 = sr * 8.0;
        let mut m = Self {
            sr,
            front: FrontEnd::new(sr, 60.0),
            os: Oversampler8::new(sr),
            // JCM800 input coupling cap → sub-rumble cut at ~35 Hz, kept below the
            // 82 Hz low-E fundamental so the distorted bass string stays intact.
            pre_clip_hp: Biquad::highpass(sr8, 35.0, 0.707),
            // JCM800 inter-stage coupling → HP at ~300 Hz. The 22 nF cap with the
            // following grid resistor actually corners well below the old 720 Hz;
            // dropping it keeps the fundamental of mid-neck notes feeding the second
            // stage so the note leads its overtones, while still tightening the lows.
            stage_hp: Biquad::highpass(sr8, 300.0, 0.707),
            bloom: Bloom::new(sr, 8.0, 55.0),
            // Bright cap: JCM800 treble-bleed, corner ~2 kHz, gentle so it adds
            // sparkle at low gain without turning the clean edge brittle.
            bright: BrightCap::new(sr, 2000.0, 0.16),
            tone_cache: ToneCache::new(),
            // First-stage cathode bias: grid current charges fast (~1.5 ms), bleeds
            // back over ~45 ms. Threshold/depth kept light so the dynamic give lives
            // within a note and recovers between notes (no cross-note timbre drift).
            cathode: CathodeBias::new(sr8, 1.5, 45.0, 0.030, 1.0),
            // Output transformer: lows (core flux) below ~160 Hz compress; modest
            // drive and a trace of crossover for the woolly, complex cranked-PA low end.
            xfmr: OutputTransformer::new(sr, 160.0, 1.4, 0.045),
            tone: ToneStack::new(sr, Components::MARSHALL),
            presence_shelf: Biquad::high_shelf(sr, 3500.0, 0.0),
            presence_cache: Cached::new(),
            voice: VoiceBalance::new(sr, 180.0, 3.5, 750.0, -7.0),
            out_hp: Biquad::highpass(sr, 12.0, 0.707),
            envelope: 0.0,
            // 8×12 resonance ~95 Hz; tube amp has moderate damping. Dynamic bloom
            // trimmed (0.55→0.30): the big sag-driven low resonance was ringing on
            // after a palm-muted chug, smearing the percussive tightness — a real
            // power amp blooms, but not so much the chug stops feeling muted.
            speaker: SpeakerLoad::new(sr, 95.0, 1.0, 0.06, 0.30, 0.8),
        };
        m.update_tone_stack(0.5, 0.45, 0.65);
        m.update_presence(0.5);
        m
    }

    fn update_tone_stack(&mut self, bass: f32, mid: f32, treble: f32) {
        self.tone.update(bass, mid, treble);
    }

    fn update_presence(&mut self, presence: f32) {
        // Presence models the JCM800 output-transformer NFB loop: shelf at 3.5 kHz, ±6 dB
        self.presence_shelf = Biquad::high_shelf(self.sr, 3500.0, (presence - 0.5) * 12.0);
    }

    #[inline]
    fn power_amp(&mut self, x: f32) -> f32 {
        let abs_x = x.abs();
        let coeff = if abs_x > self.envelope {
            1.0 - (-220.0 / self.sr).exp()
        } else {
            // Sag recovery sped up (~200 ms → ~60 ms): the slow release held the gain
            // reduction long after a palm-muted chug's attack, then recovered into a
            // swell that re-energised the note 20–40 ms in — so the chug built up
            // instead of punching. A quicker recovery keeps the attack percussive.
            1.0 - (-16.0 / self.sr).exp()
        };
        self.envelope += coeff * (abs_x - self.envelope);
        let sag = 1.0 / (1.0 + self.envelope * 0.6);
        tube_clip_asym(x * sag * 2.5) * 0.4
    }
}

impl Amplifier for Marshall {
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

        let pregain = 1.0 + gain * 39.0;
        // Dynamic grid-bias offset (removed downstream by the inter-stage HP).
        // Bias depth halved and the bloom release shortened (above): the slow,
        // deep grid-bias follower stayed elevated between notes, so a note played
        // right after others got a louder, more even-harmonic attack than the same
        // note played alone — an audible note-to-note inconsistency. A lighter,
        // faster bloom keeps the touch-sensitive give within a note but recovers
        // between them so every note attacks the same.
        let bias = self.bloom.follow(x) * 0.06;

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
            *o = tube_clip_asym(s * 3.2) / 3.2_f32.sqrt();
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
        // and (unlike the Mesa/Randall) there is no power-section high-pass after it;
        // a real output transformer passes no DC, so strip it here before the trim.
        let x = self.out_hp.process(x);

        // Output trim: the tube power stage runs at a conservative level; this
        // makeup brings the JCM800 up to the same loudness as the (much hotter)
        // solid-state Randall so switching models doesn't jump in volume.
        x * master * 3.6
    }
}

/// Asymmetric 12AX7 triode waveshaper.
///
/// Positive half: atan soft-clip (triode toward cutoff — gentle knee).
/// Negative half: atan with 1.1× input scale (toward plate saturation — clips sooner).
/// The asymmetry produces 2nd-harmonic content that gives tube amps their warmth.
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
