use super::Cabinet;
use crate::dsp::biquad::Biquad;

/// Mesa/Boogie 4×12 with Celestion Vintage 30 speakers, sm57 close-mic simulation.
///
/// V30 character: tight low end, scooped lower-mids (~400 Hz), aggressive
/// presence peak around 3.5 kHz, and a hard rolloff above 6 kHz.
pub struct MesaCab {
    sub_hp: Biquad,
    low_shelf: Biquad,
    mid_scoop: Biquad,
    presence: Biquad,
    air_shelf: Biquad,
    fizz_lp: Biquad,
}

impl MesaCab {
    pub fn new(sr: f32) -> Self {
        Self {
            sub_hp: Biquad::highpass(sr, 80.0, 0.707),
            low_shelf: Biquad::low_shelf(sr, 100.0, 3.0),
            mid_scoop: Biquad::peak_eq(sr, 400.0, 1.5, -5.0),
            presence: Biquad::peak_eq(sr, 3500.0, 1.5, 5.0),
            air_shelf: Biquad::high_shelf(sr, 6000.0, -12.0),
            fizz_lp: Biquad::lowpass(sr, 9000.0, 0.707),
        }
    }
}

impl Cabinet for MesaCab {
    #[inline]
    fn process(&mut self, sample: f32) -> f32 {
        let x = self.sub_hp.process(sample);
        let x = self.low_shelf.process(x);
        let x = self.mid_scoop.process(x);
        let x = self.presence.process(x);
        let x = self.air_shelf.process(x);
        self.fizz_lp.process(x)
    }
}
