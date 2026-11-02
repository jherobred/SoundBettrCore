//! Ten octave bands of peaking EQ: the math behind the curve, the preamp and AutoEq fitting.

use std::f64::consts::PI;

pub const BANDS: [f32; 10] = [31., 62., 125., 250., 500., 1000., 2000., 4000., 8000., 16000.];
pub const Q: f32 = 1.41;
const FS: f64 = 48_000.;

#[derive(Debug, Clone, Copy)]
pub struct Biquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a1: f64,
    pub a2: f64,
}

/// RBJ cookbook peaking filter, normalized so a0 = 1.
pub fn peaking(fs: f64, f0: f64, gain_db: f64, q: f64) -> Biquad {
    let a = 10f64.powf(gain_db / 40.);
    let w0 = 2. * PI * f0 / fs;
    let alpha = w0.sin() / (2. * q);
    let cos = w0.cos();
    let a0 = 1. + alpha / a;
    Biquad {
        b0: (1. + alpha * a) / a0,
        b1: -2. * cos / a0,
        b2: (1. - alpha * a) / a0,
        a1: -2. * cos / a0,
        a2: (1. - alpha / a) / a0,
    }
}

impl Biquad {
    pub fn magnitude_db(&self, fs: f64, f: f64) -> f64 {
        let w = 2. * PI * f / fs;
        let (c1, s1, c2, s2) = (w.cos(), w.sin(), (2. * w).cos(), (2. * w).sin());
        let nr = self.b0 + self.b1 * c1 + self.b2 * c2;
        let ni = -(self.b1 * s1 + self.b2 * s2);
        let dr = 1. + self.a1 * c1 + self.a2 * c2;
        let di = -(self.a1 * s1 + self.a2 * s2);
        10. * ((nr * nr + ni * ni) / (dr * dr + di * di)).log10()
    }
}

/// RBJ cookbook shelf; `low` picks a low shelf, otherwise high.
pub fn shelf(fs: f64, f0: f64, gain_db: f64, q: f64, low: bool) -> Biquad {
    let a = 10f64.powf(gain_db / 40.);
    let w0 = 2. * PI * f0 / fs;
    let (cos, alpha) = (w0.cos(), w0.sin() / (2. * q));
    let k = 2. * a.sqrt() * alpha;
    let s = if low { 1. } else { -1. };
    let a0 = (a + 1.) + s * (a - 1.) * cos + k;
    Biquad {
        b0: a * ((a + 1.) - s * (a - 1.) * cos + k) / a0,
        b1: s * 2. * a * ((a - 1.) - s * (a + 1.) * cos) / a0,
        b2: a * ((a + 1.) - s * (a - 1.) * cos - k) / a0,
        a1: -s * 2. * ((a - 1.) + s * (a + 1.) * cos) / a0,
        a2: ((a + 1.) + s * (a - 1.) * cos - k) / a0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FilterKind {
    #[serde(rename = "PK")]
    Peak,
    #[serde(rename = "LSC")]
    LowShelf,
    #[serde(rename = "HSC")]
    HighShelf,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    pub kind: FilterKind,
    pub fc: f32,
    pub gain: f32,
    pub q: f32,
}

impl Filter {
    pub fn biquad(&self) -> Biquad {
        let (f, g, q) = (self.fc as f64, self.gain as f64, self.q as f64);
        match self.kind {
            FilterKind::Peak => peaking(FS, f, g, q),
            FilterKind::LowShelf => shelf(FS, f, g, q, true),
            FilterKind::HighShelf => shelf(FS, f, g, q, false),
        }
    }
}

/// Summed response of a filter chain at each frequency.
pub fn filters_db(filters: &[Filter], freqs: &[f32]) -> Vec<f32> {
    let bq: Vec<Biquad> = filters.iter().filter(|f| f.gain != 0.).map(Filter::biquad).collect();
    freqs
        .iter()
        .map(|&f| bq.iter().map(|b| b.magnitude_db(FS, f as f64)).sum::<f64>() as f32)
        .collect()
}

pub fn band_filters(gains: &[f32; 10]) -> Vec<Filter> {
    BANDS.iter().zip(gains).map(|(&fc, &gain)| Filter { kind: FilterKind::Peak, fc, gain, q: Q }).collect()
}

pub fn response_db(gains: &[f32; 10], freqs: &[f32]) -> Vec<f32> {
    filters_db(&band_filters(gains), freqs)
}

/// Approximates any filter chain with the 10 bands: damped least squares over 40 Hz to 14 kHz,
/// so the whole curve matches, not just the 10 center points (that zig-zags).
pub fn fit_10band(filters: &[Filter]) -> [f32; 10] {
    let freqs: Vec<f32> = (0..48).map(|i| 40. * 350f32.powf(i as f32 / 47.)).collect();
    let target = filters_db(filters, &freqs);
    // Each band's shape at +1 dB; a peaking filter's dB response is close to linear in its gain
    let basis: Vec<Vec<f32>> = (0..10)
        .map(|i| {
            let mut g = [0.; 10];
            g[i] = 1.;
            response_db(&g, &freqs)
        })
        .collect();
    let mut g = [0f32; 10];
    for _ in 0..6 {
        let now = response_db(&g, &freqs);
        let err: Vec<f32> = target.iter().zip(&now).map(|(t, n)| t - n).collect();
        // Normal equations (A^T A + lambda I) d = A^T e
        let mut m = [[0f64; 11]; 10];
        for i in 0..10 {
            for j in 0..10 {
                m[i][j] = basis[i].iter().zip(&basis[j]).map(|(a, b)| (a * b) as f64).sum();
            }
            m[i][i] += 0.3;
            m[i][10] = basis[i].iter().zip(&err).map(|(a, e)| (a * e) as f64).sum();
        }
        let d = solve(m);
        for i in 0..10 {
            g[i] = (g[i] + d[i] as f32).clamp(-12., 12.);
        }
    }
    g.map(|v| snap(v, -12., 12.))
}

pub fn auto_preamp(gains: &[f32; 10]) -> f32 {
    -gains.iter().copied().fold(0f32, f32::max)
}

/// Clamp, then round to the nearest 0.5.
pub fn snap(v: f32, min: f32, max: f32) -> f32 {
    ((v.clamp(min, max)) * 2.).round() / 2.
}

/// Gaussian elimination with partial pivoting on an augmented 10x11 matrix.
fn solve(mut m: [[f64; 11]; 10]) -> [f64; 10] {
    for c in 0..10 {
        let p = (c..10).max_by(|&a, &b| m[a][c].abs().total_cmp(&m[b][c].abs())).unwrap();
        m.swap(c, p);
        for r in 0..10 {
            if r != c && m[c][c] != 0. {
                let f = m[r][c] / m[c][c];
                for k in c..11 {
                    m[r][k] -= f * m[c][k];
                }
            }
        }
    }
    std::array::from_fn(|i| if m[i][i] == 0. { 0. } else { m[i][10] / m[i][i] })
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_band_peaks_at_its_center() {
        let mut g = [0.; 10];
        g[5] = 6.;
        let r = response_db(&g, &[1000., 31.]);
        assert!((r[0] - 6.).abs() < 0.05, "{}", r[0]);
        assert!(r[1].abs() < 0.1, "{}", r[1]);
    }

    #[test]
    fn flat_is_zero() {
        assert!(response_db(&[0.; 10], &BANDS).iter().all(|v| v.abs() < 0.001));
    }

    #[test]
    fn preamp_is_negative_max_boost() {
        assert_eq!(auto_preamp(&[2., 1., -4.2, -3.4, -1., 0., 3.2, 3.8, 1., 0.]), -3.8);
        assert_eq!(auto_preamp(&[-1.; 10]), 0.);
    }

    #[test]
    fn shelves_reach_their_gain_far_from_the_corner() {
        let lo = Filter { kind: FilterKind::LowShelf, fc: 105., gain: 5., q: 0.7 };
        let hi = Filter { kind: FilterKind::HighShelf, fc: 8000., gain: -4., q: 0.7 };
        let r = filters_db(&[lo, hi], &[20., 1000., 20000.]);
        assert!((r[0] - 5.).abs() < 0.3, "{r:?}");
        assert!(r[1].abs() < 0.5, "{r:?}");
        assert!((r[2] + 4.).abs() < 0.6, "{r:?}");
    }

    #[test]
    fn fit_single_1k_filter() {
        let g = fit_10band(&[Filter { kind: FilterKind::Peak, fc: 1000., gain: 6., q: Q }]);
        assert!((g[5] - 6.).abs() <= 0.5, "{g:?}");
        assert!(g.iter().enumerate().all(|(i, v)| i == 5 || v.abs() <= 1.), "{g:?}");
    }

    #[test]
    fn fit_stays_in_range() {
        let g = fit_10band(&[Filter { kind: FilterKind::Peak, fc: 1000., gain: 20., q: 1. }]);
        assert!(g.iter().all(|v| (-12. ..=12.).contains(v)), "{g:?}");
    }

    #[test]
    fn snap_rounds_and_clamps() {
        assert_eq!(snap(3.26, -12., 12.), 3.5);
        assert_eq!(snap(20., -12., 12.), 12.);
    }
}
