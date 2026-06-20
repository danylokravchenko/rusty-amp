use super::Cabinet;
use crate::dsp::biquad::Biquad;

/// Mesa/Boogie 4×12 with Celestion Vintage 30 speakers, sm57 close-mic simulation.
///
/// V30 EQ signature (8 bands vs old 6):
///   • Sub HP at 80 Hz with slight resonance (ported cab alignment)
///   • +3 dB low shelf at 100 Hz (speaker low-end weight)
///   • -4 dB at 300 Hz (cardboard boxiness notch)
///   • -5 dB at 400 Hz (honky lower-mid cut — V30 mid scoop)
///   • +2 dB at 800 Hz (low-mid body / pick attack)
///   • +7 dB at 3500 Hz (V30 signature presence spike)
///   • -14 dB high shelf at 5500 Hz (speaker cone rolloff)
///   • LP at 9 kHz (fizz cut + cone break-up noise removal)
pub struct MesaCab {
    sr: f32,
    sub_hp: Biquad,
    low_shelf: Biquad,
    box_cut: Biquad,
    honk_cut: Biquad,
    body: Biquad,
    presence: Biquad,
    air_shelf: Biquad,
    fizz_lp: Biquad,
    mic_shelf: Biquad,
    last_mic_pos: f32,
}

impl MesaCab {
    pub fn new(sr: f32) -> Self {
        Self {
            sr,
            sub_hp: Biquad::highpass(sr, 80.0, 0.9),
            low_shelf: Biquad::low_shelf(sr, 100.0, 3.0),
            box_cut: Biquad::peak_eq(sr, 300.0, 1.8, -4.0),
            honk_cut: Biquad::peak_eq(sr, 400.0, 1.5, -5.0),
            body: Biquad::peak_eq(sr, 800.0, 1.5, 2.0),
            presence: Biquad::peak_eq(sr, 3500.0, 2.0, 7.0),
            air_shelf: Biquad::high_shelf(sr, 5500.0, -14.0),
            fizz_lp: Biquad::lowpass(sr, 9000.0, 0.707),
            mic_shelf: Biquad::high_shelf(sr, 5000.0, 0.0),
            last_mic_pos: -1.0,
        }
    }
}

impl Cabinet for MesaCab {
    #[inline]
    fn process(&mut self, sample: f32, mic_pos: f32) -> f32 {
        if (mic_pos - self.last_mic_pos).abs() > 0.001 {
            // 0 = edge (off-axis, dark), 1 = center (on-axis, bright)
            let db = (mic_pos - 0.5) * 12.0;
            self.mic_shelf = Biquad::high_shelf(self.sr, 5000.0, db);
            self.last_mic_pos = mic_pos;
        }
        let x = self.sub_hp.process(sample);
        let x = self.low_shelf.process(x);
        let x = self.box_cut.process(x);
        let x = self.honk_cut.process(x);
        let x = self.body.process(x);
        let x = self.presence.process(x);
        let x = self.air_shelf.process(x);
        let x = self.fizz_lp.process(x);
        self.mic_shelf.process(x)
    }
}
