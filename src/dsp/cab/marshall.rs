use super::Cabinet;
use crate::dsp::biquad::Biquad;

/// Marshall 4×12 with Celestion Greenback speakers, sm57 close-mic simulation.
///
/// Greenback character: warm low-mid body (~800 Hz), less scooped than Vintage 30s,
/// smoother presence peak around 2.5 kHz, softer fizz rolloff.
pub struct MarshallCab {
    sub_hp: Biquad,
    low_shelf: Biquad,
    mid_body: Biquad,
    presence: Biquad,
    air_shelf: Biquad,
    fizz_lp: Biquad,
}

impl MarshallCab {
    pub fn new(sr: f32) -> Self {
        Self {
            sub_hp: Biquad::highpass(sr, 80.0, 0.707),
            low_shelf: Biquad::low_shelf(sr, 120.0, 2.0),
            mid_body: Biquad::peak_eq(sr, 800.0, 2.0, 3.0),
            presence: Biquad::peak_eq(sr, 2500.0, 1.5, 4.0),
            air_shelf: Biquad::high_shelf(sr, 5000.0, -8.0),
            fizz_lp: Biquad::lowpass(sr, 8000.0, 0.707),
        }
    }
}

impl Cabinet for MarshallCab {
    #[inline]
    fn process(&mut self, sample: f32) -> f32 {
        let x = self.sub_hp.process(sample);
        let x = self.low_shelf.process(x);
        let x = self.mid_body.process(x);
        let x = self.presence.process(x);
        let x = self.air_shelf.process(x);
        self.fizz_lp.process(x)
    }
}
