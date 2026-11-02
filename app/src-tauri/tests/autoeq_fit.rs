//! Checks the 10-band fit against every AutoEq entry: how far the fitted curve strays from the real one.
use soundbettrcore_lib::autoeq::Index;
use soundbettrcore_lib::eq::{filters_db, fit_10band, response_db};
use std::path::Path;

#[test]
fn fit_tracks_every_autoeq_entry() {
    let idx = Index::load_gz(Path::new("resources/autoeq/index.json.gz")).unwrap();
    // Log-spaced 40 Hz to 14 kHz: the range the 10 bands can shape
    let freqs: Vec<f32> = (0..60).map(|i| 40. * (350f32).powf(i as f32 / 59.)).collect();
    let mut worst = (0f32, String::new());
    let mut total = 0f32;
    for e in &idx.0 {
        let target = filters_db(&e.filters, &freqs);
        let g = fit_10band(&e.filters);
        let got = response_db(&g, &freqs);
        let rms = (target.iter().zip(&got).map(|(a, b)| (a - b).powi(2)).sum::<f32>() / freqs.len() as f32).sqrt();
        total += rms;
        if rms > worst.0 {
            worst = (rms, e.model.clone());
        }
    }
    let mean = total / idx.0.len() as f32;
    println!("mean rms error {mean:.2} dB, worst {:.2} dB ({})", worst.0, worst.1);
    assert!(mean < 1.0, "mean rms error {mean:.2} dB");
}
