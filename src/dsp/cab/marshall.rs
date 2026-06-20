use super::Cabinet;
use crate::dsp::biquad::Biquad;

/// Marshall 4×12 with Celestion Greenback speakers, sm57 close-mic simulation.
///
/// Greenback EQ signature (7 bands vs old 6):
///   • Sub HP at 80 Hz (slightly looser than V30 — GB has more low-end air)
///   • +2 dB low shelf at 120 Hz (GB low warmth)
///   • -3 dB at 250 Hz (reduce muddiness below the GB mid-body peak)
///   • +4 dB at 800 Hz (Greenback signature mid warmth — the "vintage" honk)
///   • -2 dB at 1500 Hz (upper-mid smoothness — GBs are less aggressive than V30s)
///   • +5 dB at 2500 Hz (GB presence peak — warmer/lower than V30's 3.5 kHz)
///   • -10 dB high shelf at 5000 Hz (softer cone rolloff vs V30)
///   • LP at 8 kHz (fizz cut — GBs are inherently smoother on top)
pub struct MarshallCab {
    sub_hp: Biquad,
    low_shelf: Biquad,
    mud_cut: Biquad,
    mid_body: Biquad,
    upper_mid_smooth: Biquad,
    presence: Biquad,
    air_shelf: Biquad,
    fizz_lp: Biquad,
}

impl MarshallCab {
    pub fn new(sr: f32) -> Self {
        Self {
            sub_hp: Biquad::highpass(sr, 80.0, 0.8),
            low_shelf: Biquad::low_shelf(sr, 120.0, 2.0),
            mud_cut: Biquad::peak_eq(sr, 250.0, 1.5, -3.0),
            mid_body: Biquad::peak_eq(sr, 800.0, 1.5, 4.0),
            upper_mid_smooth: Biquad::peak_eq(sr, 1500.0, 1.5, -2.0),
            presence: Biquad::peak_eq(sr, 2500.0, 1.8, 5.0),
            air_shelf: Biquad::high_shelf(sr, 5000.0, -10.0),
            fizz_lp: Biquad::lowpass(sr, 8000.0, 0.707),
        }
    }
}

impl Cabinet for MarshallCab {
    #[inline]
    fn process(&mut self, sample: f32) -> f32 {
        let x = self.sub_hp.process(sample);
        let x = self.low_shelf.process(x);
        let x = self.mud_cut.process(x);
        let x = self.mid_body.process(x);
        let x = self.upper_mid_smooth.process(x);
        let x = self.presence.process(x);
        let x = self.air_shelf.process(x);
        self.fizz_lp.process(x)
    }
}
