use crate::dsp::{BiquadFilterKind, DspProfile, HOST_EQ_FREQS, ParametricBand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApoSyncStatus {
    pub live_config_path: String,
    pub equalizer_apo_detected: bool,
    pub equalizer_apo_hooked: bool,
    pub last_synced_preset: String,
}

/// Renders an EqualizerAPO-compatible `.txt` configuration representing the active
/// SoundBettrCore profile (Preamp, 10-band Parametric EQ, Sub-Bass Exciter shelf,
/// Vocal Clarity presence filter, and Binaural 7.1 Surround / Crossfeed matrix).
pub fn render_equalizer_apo_txt(profile: &DspProfile) -> String {
    if profile.bypass {
        return "# SoundBettrCore EqualizerAPO Live Chain (BYPASSED)\nPreamp: 0.0 dB\n"
            .to_string();
    }

    let mut out = String::with_capacity(1024);
    out.push_str(&format!(
        "# SoundBettrCore EqualizerAPO Live Chain\n# Profile: {} ({})\n",
        profile.name, profile.id
    ));

    let max_pos = profile
        .bands
        .iter()
        .filter(|b| b.enabled)
        .map(|b| b.gain_db)
        .fold(0.0_f64, f64::max);
    let auto_trim = if profile.auto_headroom {
        -(max_pos * 0.55 + profile.sub_bass_exciter * 1.8)
    } else {
        0.0
    };
    let effective_preamp = (profile.preamp_db + auto_trim).clamp(-15.0, 12.0);
    out.push_str(&format!("Preamp: {:.2} dB\n", effective_preamp));

    for (idx, band) in profile.bands.iter().enumerate() {
        let state = if band.enabled { "ON" } else { "OFF" };
        let kind_code = match band.kind {
            BiquadFilterKind::LowShelf => "LSC",
            BiquadFilterKind::HighShelf => "HSC",
            _ => "PK",
        };
        out.push_str(&format!(
            "Filter {}: {} {} Fc {:.1} Hz Gain {:.2} dB Q {:.2}\n",
            idx + 1,
            state,
            kind_code,
            band.freq_hz,
            band.gain_db,
            band.q
        ));
    }

    if profile.sub_bass_exciter > 0.01 {
        let sub_gain = profile.sub_bass_exciter * 3.8;
        out.push_str(&format!(
            "Filter 11: ON LSC Fc 68.0 Hz Gain {:.2} dB Q 0.95\n",
            sub_gain
        ));
    }

    if profile.vocal_clarity > 0.01 {
        let vocal_gain = profile.vocal_clarity * 3.6;
        out.push_str(&format!(
            "Filter 12: ON PK Fc 2750.0 Hz Gain {:.2} dB Q 1.15\n",
            vocal_gain
        ));
    }

    if profile.surround_width > 0.01 || profile.crossfeed_amount > 0.01 {
        let width = profile.surround_width.clamp(0.0, 1.0);
        let cf = profile.crossfeed_amount.clamp(0.0, 0.45);
        let side_boost = 0.28 * width;
        let opp_mix = (cf - side_boost).clamp(-0.35, 0.35);
        out.push_str("# 3D Binaural Surround & Crossfeed Matrix\n");
        out.push_str(&format!(
            "Copy: L={:.3}*L+{:.3}*R R={:.3}*R+{:.3}*L\n",
            1.0 + side_boost * 0.5,
            opp_mix,
            1.0 + side_boost * 0.5,
            opp_mix
        ));
    }

    out
}

/// Parses an EqualizerAPO or AutoEq `.txt` configuration into `Preamp` and `ParametricBand`s
/// mapped onto `DspProfile`.
pub fn parse_equalizer_apo_txt(content: &str, base_profile: &DspProfile) -> Result<DspProfile, String> {
    let mut next = base_profile.clone();
    let mut parsed_bands: Vec<ParametricBand> = Vec::new();
    let mut saw_any_directive = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("preamp:") {
            if let Some(rest) = line.split(':').nth(1) {
                let num_str: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '+' || *c == '.')
                    .collect();
                let cleaned = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or(&num_str)
                    .trim();
                if let Ok(val) = cleaned.parse::<f64>() {
                    next.preamp_db = val.clamp(-15.0, 15.0);
                    saw_any_directive = true;
                }
            }
        } else if lower.starts_with("filter") {
            if let Some(band) = parse_filter_line(line) {
                parsed_bands.push(band);
                saw_any_directive = true;
            }
        }
    }

    if !saw_any_directive {
        return Err("No valid EqualizerAPO Preamp or Filter lines found".to_string());
    }

    if !parsed_bands.is_empty() {
        // Map parsed bands onto our 10 standard frequencies or replace the first N bands
        for (i, slot) in next.bands.iter_mut().enumerate() {
            if let Some(exact) = parsed_bands
                .iter()
                .find(|b| (b.freq_hz - HOST_EQ_FREQS[i]).abs() < HOST_EQ_FREQS[i] * 0.18)
            {
                slot.gain_db = exact.gain_db.clamp(-12.0, 12.0);
                slot.q = exact.q.clamp(0.2, 12.0);
                slot.kind = exact.kind;
                slot.enabled = exact.enabled;
            } else if i < parsed_bands.len() {
                slot.freq_hz = parsed_bands[i].freq_hz;
                slot.gain_db = parsed_bands[i].gain_db.clamp(-12.0, 12.0);
                slot.q = parsed_bands[i].q.clamp(0.2, 12.0);
                slot.kind = parsed_bands[i].kind;
                slot.enabled = parsed_bands[i].enabled;
            }
        }
    }

    next.id = "imported_apo".to_string();
    next.name = "Imported EqualizerAPO / AutoEq".to_string();
    Ok(next)
}

fn parse_filter_line(line: &str) -> Option<ParametricBand> {
    let after_colon = line.split_once(':')?.1.trim();
    let tokens: Vec<&str> = after_colon.split_whitespace().collect();
    if tokens.len() < 4 {
        return None;
    }
    let enabled = !tokens[0].eq_ignore_ascii_case("OFF");
    let kind = match tokens[1].to_ascii_uppercase().as_str() {
        "LS" | "LSC" | "LS6" | "LS12" => BiquadFilterKind::LowShelf,
        "HS" | "HSC" | "HS6" | "HS12" => BiquadFilterKind::HighShelf,
        _ => BiquadFilterKind::Peaking,
    };

    let mut freq_hz = 1000.0;
    let mut gain_db = 0.0;
    let mut q = 1.41;

    let mut idx = 2;
    while idx < tokens.len() {
        let tok = tokens[idx];
        if tok.eq_ignore_ascii_case("Fc") && idx + 1 < tokens.len() {
            if let Ok(v) = tokens[idx + 1].replace(',', ".").parse::<f64>() {
                freq_hz = v;
            }
            idx += 2;
        } else if tok.eq_ignore_ascii_case("Gain") && idx + 1 < tokens.len() {
            if let Ok(v) = tokens[idx + 1].replace(',', ".").parse::<f64>() {
                gain_db = v;
            }
            idx += 2;
        } else if tok.eq_ignore_ascii_case("Q") && idx + 1 < tokens.len() {
            if let Ok(v) = tokens[idx + 1].replace(',', ".").parse::<f64>() {
                q = v;
            }
            idx += 2;
        } else {
            idx += 1;
        }
    }

    Some(ParametricBand {
        freq_hz,
        gain_db,
        q,
        kind,
        enabled,
    })
}

pub fn default_live_apo_path() -> PathBuf {
    if let Ok(program_data) = std::env::var("ProgramData") {
        let dir = Path::new(&program_data).join("SoundBettrCore");
        if fs::create_dir_all(&dir).is_ok() {
            return dir.join("live_apo.txt");
        }
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let dir = Path::new(&local_app_data).join("SoundBettrCore");
        let _ = fs::create_dir_all(&dir);
        return dir.join("live_apo.txt");
    }
    PathBuf::from("live_apo.txt")
}

pub fn sync_profile_to_system_apo(profile: &DspProfile) -> ApoSyncStatus {
    let live_path = default_live_apo_path();
    let txt = render_equalizer_apo_txt(profile);
    let _ = fs::write(&live_path, &txt);

    let eapo_dir = Path::new(r"C:\Program Files\EqualizerAPO\config");
    let eapo_detected = eapo_dir.exists();
    let mut eapo_hooked = false;

    if eapo_detected {
        let target_include = eapo_dir.join("soundbettrcore.txt");
        if fs::write(&target_include, &txt).is_ok() {
            let main_cfg = eapo_dir.join("config.txt");
            if let Ok(existing) = fs::read_to_string(&main_cfg) {
                if existing.contains("soundbettrcore.txt") {
                    eapo_hooked = true;
                } else {
                    let appended = format!("{}\nInclude: soundbettrcore.txt\n", existing.trim_end());
                    if fs::write(&main_cfg, appended).is_ok() {
                        eapo_hooked = true;
                    }
                }
            }
        }
    }

    ApoSyncStatus {
        live_config_path: live_path.display().to_string(),
        equalizer_apo_detected: eapo_detected,
        equalizer_apo_hooked: eapo_hooked,
        last_synced_preset: profile.name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_equalizer_apo_roundtrip_preserves_preamp_and_bands() {
        let mut original = DspProfile::default();
        original.auto_headroom = false;
        original.preamp_db = -3.5;
        original.bands[2].gain_db = -5.25; // 125 Hz
        original.bands[7].gain_db = 4.0; // 4000 Hz

        let txt = render_equalizer_apo_txt(&original);
        assert!(txt.contains("Preamp: -3.50 dB"));
        assert!(txt.contains("Filter 3: ON PK Fc 125.0 Hz Gain -5.25 dB"));

        let parsed = parse_equalizer_apo_txt(&txt, &DspProfile::default()).expect("parse APO txt");
        assert!((parsed.preamp_db - (-3.5)).abs() < 1e-4);
        assert!((parsed.bands[2].gain_db - (-5.25)).abs() < 1e-4);
        assert!((parsed.bands[7].gain_db - 4.0).abs() < 1e-4);
    }
}
