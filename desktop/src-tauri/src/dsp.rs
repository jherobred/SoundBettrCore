use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

pub const HOST_EQ_FREQS: [f64; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// R50i NC (A3959) Harman In-Ear 2019 / AutoEq Corrective Curve (in dB)
/// Cuts 62-250 Hz mid-bass bloat/mud and restores 2-4 kHz upper-mid presence + 16 kHz air.
pub const R50I_NC_HARMAN_GAINS_DB: [f64; 10] = [
    1.5, -2.0, -4.5, -2.5, 0.0, 1.0, 3.0, 3.5, 1.5, 2.5,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiquadFilterKind {
    LowShelf,
    Peaking,
    HighShelf,
    LowPass,
    HighPass,
    BandPass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParametricBand {
    pub freq_hz: f64,
    pub gain_db: f64,
    pub q: f64,
    pub kind: BiquadFilterKind,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomModelKind {
    StudioBooth,
    CinemaTheater,
    ConcertHall,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DspProfile {
    pub id: String,
    pub name: String,
    pub subtitle: String,
    pub bypass: bool,
    pub auto_headroom: bool,
    pub preamp_db: f64,
    pub harman_correction_enabled: bool,
    pub bands: Vec<ParametricBand>,
    /// Macro 1: 3D Binaural Surround / Virtual 7.1 Width (0.0 .. 1.0)
    pub surround_width: f64,
    /// Crossfeed percentage (0.0 .. 0.5)
    pub crossfeed_amount: f64,
    /// Macro 2: Harmonic Sub-Bass Exciter amount (0.0 .. 1.0)
    pub sub_bass_exciter: f64,
    /// Macro 3: Dialog & Vocal Clarity Enhancer (0.0 .. 1.0)
    pub vocal_clarity: f64,
    /// Macro 4: Room / Hall Acoustic Virtualizer mix (0.0 .. 1.0)
    pub room_space: f64,
    pub room_model: RoomModelKind,
    /// Stage 7: Dolby-style 3-band dynamic compressor & loudness leveler (0.0 .. 1.0)
    pub dolby_leveler_amount: f64,
    pub limiter_ceiling_db: f64,
    /// Linked R50i NC hardware 8-band EQ in tenths of dB (-120..=120)
    pub hw_eq_tenths_db: [i16; 8],
}

impl Default for DspProfile {
    fn default() -> Self {
        let bands = HOST_EQ_FREQS
            .iter()
            .enumerate()
            .map(|(idx, &freq)| ParametricBand {
                freq_hz: freq,
                gain_db: R50I_NC_HARMAN_GAINS_DB[idx],
                q: if idx == 0 || idx == 9 { 0.707 } else { 1.41 },
                kind: if idx == 0 {
                    BiquadFilterKind::LowShelf
                } else if idx == 9 {
                    BiquadFilterKind::HighShelf
                } else {
                    BiquadFilterKind::Peaking
                },
                enabled: true,
            })
            .collect();

        Self {
            id: "r50i_harman_reference".to_string(),
            name: "R50i NC Harman Reference".to_string(),
            subtitle: "AutoEq mid-bass mud cut + upper-mid vocal clarity & subtle 3D stage"
                .to_string(),
            bypass: false,
            auto_headroom: true,
            preamp_db: 0.0,
            harman_correction_enabled: true,
            bands,
            surround_width: 0.35,
            crossfeed_amount: 0.22,
            sub_bass_exciter: 0.38,
            vocal_clarity: 0.42,
            room_space: 0.16,
            room_model: RoomModelKind::StudioBooth,
            dolby_leveler_amount: 0.45,
            limiter_ceiling_db: -0.3,
            hw_eq_tenths_db: [-25, -18, 5, 18, 32, 24, 10, 15],
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoeffs {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Default for BiquadCoeffs {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

impl BiquadCoeffs {
    pub fn design(
        kind: BiquadFilterKind,
        freq_hz: f64,
        sample_rate: f64,
        gain_db: f64,
        q: f64,
    ) -> Self {
        let nyquist = (sample_rate * 0.49).max(1000.0);
        let f0 = freq_hz.clamp(16.0, nyquist);
        let q_safe = q.clamp(0.1, 18.0);
        let w0 = 2.0 * PI * f0 / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q_safe);
        let a = 10.0_f64.powf(gain_db / 40.0);

        let (b0, b1, b2, a0, a1, a2) = match kind {
            BiquadFilterKind::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            BiquadFilterKind::LowShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            BiquadFilterKind::HighShelf => {
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
                    (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
                )
            }
            BiquadFilterKind::LowPass => (
                (1.0 - cos_w0) * 0.5,
                1.0 - cos_w0,
                (1.0 - cos_w0) * 0.5,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            BiquadFilterKind::HighPass => (
                (1.0 + cos_w0) * 0.5,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) * 0.5,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            BiquadFilterKind::BandPass => (
                alpha,
                0.0,
                -alpha,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
        };

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    pub fn magnitude_db_at(&self, freq_hz: f64, sample_rate: f64) -> f64 {
        let w = 2.0 * PI * freq_hz / sample_rate;
        let cos_w = w.cos();
        let cos_2w = (2.0 * w).cos();
        let sin_w = w.sin();
        let sin_2w = (2.0 * w).sin();

        let num_re = self.b0 + self.b1 * cos_w + self.b2 * cos_2w;
        let num_im = -self.b1 * sin_w - self.b2 * sin_2w;
        let den_re = 1.0 + self.a1 * cos_w + self.a2 * cos_2w;
        let den_im = -self.a1 * sin_w - self.a2 * sin_2w;

        let num_mag_sq = num_re * num_re + num_im * num_im;
        let den_mag_sq = (den_re * den_re + den_im * den_im).max(1e-18);
        10.0 * (num_mag_sq / den_mag_sq).log10()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StereoBiquad {
    pub coeffs: BiquadCoeffs,
    z1_l: f64,
    z2_l: f64,
    z1_r: f64,
    z2_r: f64,
}

impl StereoBiquad {
    pub fn new(coeffs: BiquadCoeffs) -> Self {
        Self {
            coeffs,
            z1_l: 0.0,
            z2_l: 0.0,
            z1_r: 0.0,
            z2_r: 0.0,
        }
    }

    #[inline]
    pub fn process_stereo(&mut self, l: f64, r: f64) -> (f64, f64) {
        let out_l = self.coeffs.b0 * l + self.z1_l;
        self.z1_l = self.coeffs.b1 * l - self.coeffs.a1 * out_l + self.z2_l;
        self.z2_l = self.coeffs.b2 * l - self.coeffs.a2 * out_l;

        let out_r = self.coeffs.b0 * r + self.z1_r;
        self.z1_r = self.coeffs.b1 * r - self.coeffs.a1 * out_r + self.z2_r;
        self.z2_r = self.coeffs.b2 * r - self.coeffs.a2 * out_r;

        (out_l, out_r)
    }
}

#[derive(Debug, Clone)]
struct DelayLine {
    buf: Vec<f64>,
    write_idx: usize,
}

impl DelayLine {
    fn new(capacity: usize) -> Self {
        Self {
            buf: vec![0.0; capacity.max(4)],
            write_idx: 0,
        }
    }

    #[inline]
    fn push_and_read(&mut self, sample: f64, delay_samples: usize) -> f64 {
        let len = self.buf.len();
        self.buf[self.write_idx] = sample;
        let read_idx = (self.write_idx + len - (delay_samples % len)) % len;
        self.write_idx = (self.write_idx + 1) % len;
        self.buf[read_idx]
    }
}

#[derive(Debug, Clone)]
struct CombFilter {
    buf: Vec<f64>,
    idx: usize,
    filter_store: f64,
    feedback: f64,
    damp1: f64,
    damp2: f64,
}

impl CombFilter {
    fn new(size: usize, feedback: f64, damp: f64) -> Self {
        Self {
            buf: vec![0.0; size.max(8)],
            idx: 0,
            filter_store: 0.0,
            feedback,
            damp1: damp,
            damp2: 1.0 - damp,
        }
    }

    #[inline]
    fn process(&mut self, input: f64) -> f64 {
        let output = self.buf[self.idx];
        self.filter_store = output * self.damp2 + self.filter_store * self.damp1;
        self.buf[self.idx] = input + self.filter_store * self.feedback;
        self.idx = (self.idx + 1) % self.buf.len();
        output
    }
}

#[derive(Debug, Clone)]
struct AllPassFilter {
    buf: Vec<f64>,
    idx: usize,
    feedback: f64,
}

impl AllPassFilter {
    fn new(size: usize, feedback: f64) -> Self {
        Self {
            buf: vec![0.0; size.max(4)],
            idx: 0,
            feedback,
        }
    }

    #[inline]
    fn process(&mut self, input: f64) -> f64 {
        let buf_out = self.buf[self.idx];
        let output = -input + buf_out;
        self.buf[self.idx] = input + buf_out * self.feedback;
        self.idx = (self.idx + 1) % self.buf.len();
        output
    }
}

pub struct DspPipeline {
    pub sample_rate: f64,
    pub profile: DspProfile,
    preamp_linear: f64,
    eq_filters: Vec<StereoBiquad>,
    sub_bp_filter: StereoBiquad,
    sub_lp_filter: StereoBiquad,
    vocal_bp_filter: StereoBiquad,
    pinna_presence_filter: StereoBiquad,
    crossfeed_lp_filter: StereoBiquad,
    itd_delay_l: DelayLine,
    itd_delay_r: DelayLine,
    rear_delay_l: DelayLine,
    rear_delay_r: DelayLine,
    crossfeed_delay_l: DelayLine,
    crossfeed_delay_r: DelayLine,
    combs_l: [CombFilter; 4],
    combs_r: [CombFilter; 4],
    allpass_l: [AllPassFilter; 2],
    allpass_r: [AllPassFilter; 2],
    comp_low_filter: StereoBiquad,
    comp_high_filter: StereoBiquad,
    env_low: f64,
    env_mid: f64,
    env_high: f64,
    limiter_env: f64,
}

impl DspPipeline {
    pub fn new(sample_rate: f64, profile: DspProfile) -> Self {
        let max_delay = ((sample_rate * 0.005) as usize).max(64);
        let mut pipeline = Self {
            sample_rate,
            profile: profile.clone(),
            preamp_linear: 1.0,
            eq_filters: Vec::new(),
            sub_bp_filter: StereoBiquad::default(),
            sub_lp_filter: StereoBiquad::default(),
            vocal_bp_filter: StereoBiquad::default(),
            pinna_presence_filter: StereoBiquad::default(),
            crossfeed_lp_filter: StereoBiquad::default(),
            itd_delay_l: DelayLine::new(max_delay),
            itd_delay_r: DelayLine::new(max_delay),
            rear_delay_l: DelayLine::new(max_delay * 4),
            rear_delay_r: DelayLine::new(max_delay * 4),
            crossfeed_delay_l: DelayLine::new(max_delay),
            crossfeed_delay_r: DelayLine::new(max_delay),
            combs_l: [
                CombFilter::new(1116, 0.78, 0.25),
                CombFilter::new(1188, 0.78, 0.25),
                CombFilter::new(1277, 0.78, 0.25),
                CombFilter::new(1356, 0.78, 0.25),
            ],
            combs_r: [
                CombFilter::new(1139, 0.78, 0.25),
                CombFilter::new(1211, 0.78, 0.25),
                CombFilter::new(1300, 0.78, 0.25),
                CombFilter::new(1379, 0.78, 0.25),
            ],
            allpass_l: [
                AllPassFilter::new(556, 0.5),
                AllPassFilter::new(441, 0.5),
            ],
            allpass_r: [
                AllPassFilter::new(579, 0.5),
                AllPassFilter::new(464, 0.5),
            ],
            comp_low_filter: StereoBiquad::default(),
            comp_high_filter: StereoBiquad::default(),
            env_low: 0.0,
            env_mid: 0.0,
            env_high: 0.0,
            limiter_env: 0.0,
        };
        pipeline.update_profile(profile);
        pipeline
    }

    pub fn update_profile(&mut self, profile: DspProfile) {
        let max_positive_eq_gain = profile
            .bands
            .iter()
            .filter(|b| b.enabled)
            .map(|b| b.gain_db)
            .fold(0.0_f64, f64::max);
        let exciter_headroom = profile.sub_bass_exciter * 2.5 + profile.vocal_clarity * 1.8;
        let auto_trim_db = if profile.auto_headroom {
            -(max_positive_eq_gain * 0.55 + exciter_headroom)
        } else {
            0.0
        };
        let total_preamp_db = (profile.preamp_db + auto_trim_db).clamp(-18.0, 12.0);
        self.preamp_linear = 10.0_f64.powf(total_preamp_db / 20.0);

        self.eq_filters = profile
            .bands
            .iter()
            .filter(|b| b.enabled && b.gain_db.abs() > 0.01)
            .map(|b| {
                StereoBiquad::new(BiquadCoeffs::design(
                    b.kind,
                    b.freq_hz,
                    self.sample_rate,
                    b.gain_db,
                    b.q,
                ))
            })
            .collect();

        self.sub_bp_filter = StereoBiquad::new(BiquadCoeffs::design(
            BiquadFilterKind::BandPass,
            72.0,
            self.sample_rate,
            0.0,
            1.1,
        ));
        self.sub_lp_filter = StereoBiquad::new(BiquadCoeffs::design(
            BiquadFilterKind::LowPass,
            175.0,
            self.sample_rate,
            0.0,
            0.707,
        ));
        self.vocal_bp_filter = StereoBiquad::new(BiquadCoeffs::design(
            BiquadFilterKind::Peaking,
            2750.0,
            self.sample_rate,
            profile.vocal_clarity * 4.5,
            1.15,
        ));
        self.pinna_presence_filter = StereoBiquad::new(BiquadCoeffs::design(
            BiquadFilterKind::HighShelf,
            3400.0,
            self.sample_rate,
            profile.surround_width * 2.2,
            0.707,
        ));
        self.crossfeed_lp_filter = StereoBiquad::new(BiquadCoeffs::design(
            BiquadFilterKind::LowPass,
            680.0,
            self.sample_rate,
            0.0,
            0.707,
        ));
        self.comp_low_filter = StereoBiquad::new(BiquadCoeffs::design(
            BiquadFilterKind::LowPass,
            220.0,
            self.sample_rate,
            0.0,
            0.707,
        ));
        self.comp_high_filter = StereoBiquad::new(BiquadCoeffs::design(
            BiquadFilterKind::HighPass,
            2600.0,
            self.sample_rate,
            0.0,
            0.707,
        ));

        let (feedback, damp) = match profile.room_model {
            RoomModelKind::StudioBooth => (0.64, 0.42),
            RoomModelKind::CinemaTheater => (0.78, 0.28),
            RoomModelKind::ConcertHall => (0.86, 0.18),
        };
        for c in &mut self.combs_l {
            c.feedback = feedback;
            c.damp1 = damp;
            c.damp2 = 1.0 - damp;
        }
        for c in &mut self.combs_r {
            c.feedback = feedback;
            c.damp1 = damp;
            c.damp2 = 1.0 - damp;
        }

        self.profile = profile;
    }

    pub fn process_frame(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if self.profile.bypass {
            return (in_l, in_r);
        }

        // Stage 1: Pre-Amp & Auto-Headroom
        let mut l = (in_l as f64) * self.preamp_linear;
        let mut r = (in_r as f64) * self.preamp_linear;

        // Stage 2: 10-Band RBJ Biquad Parametric EQ (includes R50i NC Harman target)
        for filter in &mut self.eq_filters {
            let (nl, nr) = filter.process_stereo(l, r);
            l = nl;
            r = nr;
        }

        // Stage 3: Harmonic Sub-Bass Exciter ("BassUp Studio")
        if self.profile.sub_bass_exciter > 0.005 {
            let drive = 1.0 + self.profile.sub_bass_exciter * 3.2;
            let (sub_l, sub_r) = self.sub_bp_filter.process_stereo(l, r);
            let sat_l = (sub_l * drive).tanh() + 0.18 * (sub_l * drive * 2.0).tanh();
            let sat_r = (sub_r * drive).tanh() + 0.18 * (sub_r * drive * 2.0).tanh();
            let (harm_l, harm_r) = self.sub_lp_filter.process_stereo(sat_l, sat_r);
            let mix = self.profile.sub_bass_exciter * 0.42;
            l += harm_l * mix;
            r += harm_r * mix;
        }

        // Stage 4: Dialog / Vocal Clarity Enhancer (Mid-channel presence extractor)
        if self.profile.vocal_clarity > 0.005 {
            let mid = 0.5 * (l + r);
            let side = 0.5 * (l - r);
            let (mid_boosted, _) = self.vocal_bp_filter.process_stereo(mid, mid);
            let blend = self.profile.vocal_clarity * 0.65;
            let enhanced_mid = mid * (1.0 - blend) + mid_boosted * blend;
            l = enhanced_mid + side;
            r = enhanced_mid - side;
        }

        // Stage 5: 3D Binaural Surround / Virtual 7.1 Spatializer & Crossfeed
        if self.profile.surround_width > 0.005 || self.profile.crossfeed_amount > 0.005 {
            let mid = 0.5 * (l + r);
            let side = 0.5 * (l - r);

            let itd_samples = ((self.sample_rate * 0.00038) as usize).max(1);
            let rear_samples = ((self.sample_rate * 0.00165) as usize).max(2);
            let cf_samples = ((self.sample_rate * 0.00030) as usize).max(1);

            let side_delayed_l = self.itd_delay_l.push_and_read(side, itd_samples);
            let side_delayed_r = self.itd_delay_r.push_and_read(-side, itd_samples);
            let rear_l = self.rear_delay_l.push_and_read(side_delayed_l, rear_samples);
            let rear_r = self.rear_delay_r.push_and_read(side_delayed_r, rear_samples);

            let (pinna_l, pinna_r) = self
                .pinna_presence_filter
                .process_stereo(side_delayed_l + 0.35 * rear_l, side_delayed_r + 0.35 * rear_r);

            let width = self.profile.surround_width;
            let wide_l = mid + side * (1.0 + width * 0.58) + pinna_l * (width * 0.44);
            let wide_r = mid - side * (1.0 + width * 0.58) + pinna_r * (width * 0.44);

            let (cf_low_l, cf_low_r) = self.crossfeed_lp_filter.process_stereo(wide_l, wide_r);
            let delayed_cf_l = self.crossfeed_delay_l.push_and_read(cf_low_l, cf_samples);
            let delayed_cf_r = self.crossfeed_delay_r.push_and_read(cf_low_r, cf_samples);

            let cf = self.profile.crossfeed_amount.clamp(0.0, 0.45);
            l = wide_l * (1.0 - cf * 0.35) + delayed_cf_r * cf;
            r = wide_r * (1.0 - cf * 0.35) + delayed_cf_l * cf;
        }

        // Stage 6: Room / Concert Hall Acoustic Model (Schroeder/Freeverb)
        if self.profile.room_space > 0.005 {
            let input_mono = (l + r) * 0.015;
            let mut rev_l = 0.0;
            let mut rev_r = 0.0;
            for c in &mut self.combs_l {
                rev_l += c.process(input_mono);
            }
            for c in &mut self.combs_r {
                rev_r += c.process(input_mono);
            }
            for ap in &mut self.allpass_l {
                rev_l = ap.process(rev_l);
            }
            for ap in &mut self.allpass_r {
                rev_r = ap.process(rev_r);
            }
            let wet = self.profile.room_space * 0.48;
            l = l * (1.0 - wet * 0.25) + rev_l * wet;
            r = r * (1.0 - wet * 0.25) + rev_r * wet;
        }

        // Stage 7: Dolby-Style 3-Band Compressor / Loudness Leveler + -0.3 dBFS Limiter
        if self.profile.dolby_leveler_amount > 0.005 {
            let (low_l, low_r) = self.comp_low_filter.process_stereo(l, r);
            let (high_l, high_r) = self.comp_high_filter.process_stereo(l, r);
            let mid_l = l - low_l - high_l;
            let mid_r = r - low_r - high_r;

            let amt = self.profile.dolby_leveler_amount.clamp(0.0, 1.0);
            let g_low = Self::leveler_band_gain(&mut self.env_low, low_l.abs().max(low_r.abs()), amt);
            let g_mid = Self::leveler_band_gain(&mut self.env_mid, mid_l.abs().max(mid_r.abs()), amt);
            let g_high =
                Self::leveler_band_gain(&mut self.env_high, high_l.abs().max(high_r.abs()), amt);

            l = low_l * g_low + mid_l * g_mid + high_l * g_high;
            r = low_r * g_low + mid_r * g_mid + high_r * g_high;
        }

        // Brickwall Soft-Knee Limiter at ceiling (-0.3 dBFS default => 0.96605)
        let ceiling = 10.0_f64
            .powf(self.profile.limiter_ceiling_db.clamp(-6.0, -0.1) / 20.0)
            .min(0.966);
        let peak = l.abs().max(r.abs());
        if peak > self.limiter_env {
            self.limiter_env = 0.25 * self.limiter_env + 0.75 * peak;
        } else {
            self.limiter_env = 0.9985 * self.limiter_env + 0.0015 * peak;
        }
        if self.limiter_env > ceiling {
            let scale = ceiling / self.limiter_env;
            l *= scale;
            r *= scale;
        }
        // Soft saturation guard so single-sample transients never exceed ceiling
        if l.abs() > ceiling {
            l = ceiling * (l / ceiling).tanh();
        }
        if r.abs() > ceiling {
            r = ceiling * (r / ceiling).tanh();
        }

        (l as f32, r as f32)
    }

    #[inline]
    fn leveler_band_gain(env: &mut f64, peak: f64, amount: f64) -> f64 {
        let coeff = if peak > *env { 0.04 } else { 0.0008 };
        *env += coeff * (peak - *env);
        let level = (*env).max(1e-5);
        // Upward lift for quiet details (< 0.12) and gentle downward compression (> 0.35)
        let raw_gain = if level < 0.12 {
            1.0 + amount * 0.55 * ((0.12 - level) / 0.12)
        } else if level > 0.35 {
            1.0 / (1.0 + amount * 1.35 * (level - 0.35))
        } else {
            1.0 + amount * 0.12
        };
        raw_gain.clamp(0.45, 1.65)
    }

    pub fn process_interleaved_buffer(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_exact_mut(2) {
            let (l, r) = self.process_frame(frame[0], frame[1]);
            frame[0] = l;
            frame[1] = r;
        }
    }

    /// Computes composite EQ frequency response curve at `num_points` log-spaced frequencies (20 Hz..20 kHz)
    pub fn compute_response_curve(&self, num_points: usize) -> Vec<(f64, f64)> {
        let n = num_points.max(16);
        let min_log = 20.0_f64.ln();
        let max_log = 20000.0_f64.ln();
        (0..n)
            .map(|i| {
                let t = (i as f64) / ((n - 1) as f64);
                let freq = (min_log + t * (max_log - min_log)).exp();
                let mut db = self.profile.preamp_db;
                for filter in &self.eq_filters {
                    db += filter.coeffs.magnitude_db_at(freq, self.sample_rate);
                }
                (freq, db)
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestSignalKind {
    Orbit71,
    SubBassSweep,
    VocalPulse,
    PinkNoise,
}

/// Synthesizes stereo 48 kHz test audio and optionally passes it through the active `DspProfile`,
/// returning a standard 16-bit PCM RIFF WAV byte buffer ready for instant playback.
pub fn synthesize_test_wav(
    kind: TestSignalKind,
    duration_secs: f32,
    profile: &DspProfile,
) -> Vec<u8> {
    let sample_rate = 48000usize;
    let frames = ((duration_secs.clamp(1.0, 12.0) * (sample_rate as f32)) as usize).max(4800);
    let mut stereo_buf = Vec::with_capacity(frames * 2);
    let mut rng_state = 0xA395_9BEE_u32;
    let mut pink_b0 = 0.0f64;
    let mut pink_b1 = 0.0f64;
    let mut pink_b2 = 0.0f64;

    for i in 0..frames {
        let t = (i as f64) / (sample_rate as f64);
        rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let white = ((rng_state >> 9) as f64 / 4194304.0) - 1.0;
        pink_b0 = 0.99765 * pink_b0 + white * 0.0990460;
        pink_b1 = 0.96300 * pink_b1 + white * 0.2965164;
        pink_b2 = 0.57000 * pink_b2 + white * 1.0526913;
        let pink = (pink_b0 + pink_b1 + pink_b2 + white * 0.1848) * 0.06;

        let (l, r) = match kind {
            TestSignalKind::Orbit71 => {
                let angle = 2.0 * PI * (t / (duration_secs as f64));
                let pan_l = ((1.0 - angle.sin()) * 0.5).sqrt();
                let pan_r = ((1.0 + angle.sin()) * 0.5).sqrt();
                let chime = (2.0 * PI * 440.0 * t).sin() * (-((t * 2.0).fract() * 5.0)).exp() * 0.28
                    + (2.0 * PI * 880.0 * t).sin() * (-((t * 2.0).fract() * 8.0)).exp() * 0.14;
                let sig = chime + pink * 0.45;
                ((sig * pan_l) as f32, (sig * pan_r) as f32)
            }
            TestSignalKind::SubBassSweep => {
                let progress = t / (duration_secs as f64);
                let f_start = 25.0;
                let f_end = 160.0;
                let phase = 2.0
                    * PI
                    * f_start
                    * (duration_secs as f64)
                    * ((f_end / f_start).powf(progress) - 1.0)
                    / (f_end / f_start).ln();
                let val = (phase.sin() * 0.55) as f32;
                (val, val)
            }
            TestSignalKind::VocalPulse => {
                let env = ((2.0 * PI * 1.5 * t).sin().max(0.0)).powi(2);
                let f0 = 190.0;
                let formant = ((2.0 * PI * f0 * t).sin() * 0.25
                    + (2.0 * PI * (f0 * 2.0) * t).sin() * 0.20
                    + (2.0 * PI * 2400.0 * t).sin() * 0.16
                    + (2.0 * PI * 3200.0 * t).sin() * 0.12)
                    * env;
                (formant as f32, formant as f32)
            }
            TestSignalKind::PinkNoise => ((pink * 0.7) as f32, (pink * 0.7) as f32),
        };
        stereo_buf.push(l);
        stereo_buf.push(r);
    }

    let mut pipeline = DspPipeline::new(sample_rate as f64, profile.clone());
    pipeline.process_interleaved_buffer(&mut stereo_buf);
    encode_wav_16bit(&stereo_buf, sample_rate as u32)
}

pub fn encode_wav_16bit(interleaved_stereo: &[f32], sample_rate: u32) -> Vec<u8> {
    let data_bytes = interleaved_stereo.len() * 2;
    let total_riff_len = 36 + data_bytes;
    let mut wav = Vec::with_capacity(44 + data_bytes);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(total_riff_len as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&2u16.to_le_bytes()); // Stereo
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 4).to_le_bytes()); // byte rate
    wav.extend_from_slice(&4u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    for &sample in interleaved_stereo {
        let clamped = (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16;
        wav.extend_from_slice(&clamped.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harman_target_cuts_125hz_mud_and_boosts_3khz_clarity() {
        let profile = DspProfile::default();
        let pipeline = DspPipeline::new(48000.0, profile);
        let curve = pipeline.compute_response_curve(128);

        let mud_125 = curve
            .iter()
            .min_by(|a, b| (a.0 - 125.0).abs().partial_cmp(&(b.0 - 125.0).abs()).unwrap())
            .unwrap()
            .1;
        let clarity_3500 = curve
            .iter()
            .min_by(|a, b| {
                (a.0 - 3500.0)
                    .abs()
                    .partial_cmp(&(b.0 - 3500.0).abs())
                    .unwrap()
            })
            .unwrap()
            .1;

        assert!(
            mud_125 < -3.0,
            "Expected 125 Hz mid-bass mud cut < -3.0 dB, got {mud_125:.2} dB"
        );
        assert!(
            clarity_3500 > 2.0,
            "Expected 3.5 kHz upper-mid boost > +2.0 dB, got {clarity_3500:.2} dB"
        );
    }

    #[test]
    fn test_brickwall_limiter_prevents_clipping_under_extreme_boost() {
        let mut profile = DspProfile::default();
        profile.auto_headroom = false;
        profile.preamp_db = 12.0;
        profile.sub_bass_exciter = 1.0;
        profile.vocal_clarity = 1.0;
        profile.surround_width = 1.0;
        profile.dolby_leveler_amount = 1.0;
        for b in &mut profile.bands {
            b.gain_db = 12.0;
        }

        let mut pipeline = DspPipeline::new(48000.0, profile);
        let mut max_abs = 0.0f32;
        for i in 0..4800 {
            let t = i as f64 / 48000.0;
            let raw = (2.0 * PI * 80.0 * t).sin() as f32 * 0.95;
            let (l, r) = pipeline.process_frame(raw, raw);
            max_abs = max_abs.max(l.abs()).max(r.abs());
        }
        assert!(
            max_abs <= 0.967,
            "Limiter must clamp output <= -0.3 dBFS (0.967), got {max_abs}"
        );
        assert!(max_abs > 0.4, "Signal should still be audible, got {max_abs}");
    }

    #[test]
    fn test_synthesize_test_wav_produces_valid_riff_header() {
        let profile = DspProfile::default();
        let wav = synthesize_test_wav(TestSignalKind::Orbit71, 1.0, &profile);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert!(wav.len() > 48000 * 4);
    }
}
