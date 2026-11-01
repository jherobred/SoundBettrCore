use crate::dsp::{BiquadFilterKind, DspProfile, HOST_EQ_FREQS, ParametricBand, RoomModelKind};

fn make_bands(gains_db: [f64; 10]) -> Vec<ParametricBand> {
    HOST_EQ_FREQS
        .iter()
        .enumerate()
        .map(|(idx, &freq_hz)| ParametricBand {
            freq_hz,
            gain_db: gains_db[idx],
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
        .collect()
}

pub fn builtin_presets() -> Vec<DspProfile> {
    vec![
        DspProfile::default(),
        DspProfile {
            id: "dolby_cinema_3d".to_string(),
            name: "Dolby Cinema 3D".to_string(),
            subtitle: "Wide 7.1 virtual surround, theater room acoustics, dialog lift & loudness leveler".to_string(),
            bypass: false,
            auto_headroom: true,
            preamp_db: 0.5,
            harman_correction_enabled: true,
            bands: make_bands([3.5, 1.0, -3.5, -2.0, 0.5, 2.0, 3.8, 4.0, 2.0, 3.0]),
            surround_width: 0.78,
            crossfeed_amount: 0.18,
            sub_bass_exciter: 0.62,
            vocal_clarity: 0.68,
            room_space: 0.42,
            room_model: RoomModelKind::CinemaTheater,
            dolby_leveler_amount: 0.72,
            limiter_ceiling_db: -0.3,
            hw_eq_tenths_db: [15, -20, -10, 15, 38, 35, 20, 25],
        },
        DspProfile {
            id: "dolby_music_studio".to_string(),
            name: "Dolby Music Studio".to_string(),
            subtitle: "Mastered Harman target curve, studio crossfeed, warm harmonic sub-bass & natural dynamics".to_string(),
            bypass: false,
            auto_headroom: true,
            preamp_db: 0.0,
            harman_correction_enabled: true,
            bands: make_bands([2.5, -1.0, -4.0, -2.0, 0.5, 1.2, 3.2, 3.5, 2.2, 3.2]),
            surround_width: 0.48,
            crossfeed_amount: 0.28,
            sub_bass_exciter: 0.46,
            vocal_clarity: 0.45,
            room_space: 0.20,
            room_model: RoomModelKind::StudioBooth,
            dolby_leveler_amount: 0.48,
            limiter_ceiling_db: -0.3,
            hw_eq_tenths_db: [-10, -25, 0, 12, 32, 30, 22, 30],
        },
        DspProfile {
            id: "bassup_ultra_clean".to_string(),
            name: "BassUp Ultra Clean".to_string(),
            subtitle: "Deep 31-62 Hz sub-bass slam with 125 Hz anti-mud notch so vocals stay crystal clear".to_string(),
            bypass: false,
            auto_headroom: true,
            preamp_db: 0.0,
            harman_correction_enabled: true,
            bands: make_bands([5.5, 3.2, -4.8, -2.5, 0.0, 1.0, 2.8, 3.2, 2.0, 2.8]),
            surround_width: 0.40,
            crossfeed_amount: 0.20,
            sub_bass_exciter: 0.85,
            vocal_clarity: 0.40,
            room_space: 0.14,
            room_model: RoomModelKind::StudioBooth,
            dolby_leveler_amount: 0.55,
            limiter_ceiling_db: -0.3,
            hw_eq_tenths_db: [35, -30, -15, 10, 28, 30, 20, 25],
        },
        DspProfile {
            id: "fps_gaming_footsteps".to_string(),
            name: "FPS Gaming Footsteps".to_string(),
            subtitle: "Pinpoint 7.1 binaural imaging, 2-6 kHz footstep & reload detail, low-end explosion control".to_string(),
            bypass: false,
            auto_headroom: true,
            preamp_db: 0.5,
            harman_correction_enabled: true,
            bands: make_bands([-2.0, -3.5, -5.0, -1.5, 1.0, 2.5, 4.8, 5.2, 3.5, 2.5]),
            surround_width: 0.88,
            crossfeed_amount: 0.10,
            sub_bass_exciter: 0.15,
            vocal_clarity: 0.58,
            room_space: 0.08,
            room_model: RoomModelKind::StudioBooth,
            dolby_leveler_amount: 0.80,
            limiter_ceiling_db: -0.3,
            hw_eq_tenths_db: [-35, -40, 10, 25, 48, 50, 35, 20],
        },
        DspProfile {
            id: "podcast_late_night".to_string(),
            name: "Podcast & Late Night".to_string(),
            subtitle: "Dialog isolation, sub-bass rumble cut, and strong upward/downward loudness leveling".to_string(),
            bypass: false,
            auto_headroom: true,
            preamp_db: 1.0,
            harman_correction_enabled: true,
            bands: make_bands([-4.0, -4.5, -4.5, -1.0, 1.5, 3.2, 4.2, 3.8, 1.0, 0.5]),
            surround_width: 0.20,
            crossfeed_amount: 0.32,
            sub_bass_exciter: 0.05,
            vocal_clarity: 0.88,
            room_space: 0.10,
            room_model: RoomModelKind::StudioBooth,
            dolby_leveler_amount: 0.90,
            limiter_ceiling_db: -0.3,
            hw_eq_tenths_db: [-45, -35, 15, 32, 42, 38, 10, 5],
        },
    ]
}
