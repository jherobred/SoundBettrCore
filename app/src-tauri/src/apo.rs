//! Writes EqualizerAPO config: one block per device, selected by endpoint GUID, so APO switches by itself.

use crate::eq::BANDS;
use crate::store::Profile;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct ApoTarget<'a> {
    /// Windows endpoint id, like "{0.0.0.00000000}.{guid}"
    pub endpoint_id: &'a str,
    pub profile: &'a Profile,
    /// Plays flat while a hearing test runs
    pub bypass: bool,
}

/// The endpoint's GUID. Anything else (names especially) never reaches the config.
fn endpoint_guid(id: &str) -> Option<String> {
    let g = id.rsplit('.').next()?.trim();
    let ok = g.len() == 38 && g.starts_with('{') && g.ends_with('}') && g[1..37].chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    ok.then(|| g.to_string())
}

fn db(v: f32) -> String {
    // + 0.0 turns -0.0 into 0.0
    format!("{:.1}", v + 0.0)
}

pub fn render(targets: &[ApoTarget], hrir_dir: &Path) -> String {
    let mut out = String::from("# Written by SoundBettrCore. Changes here are overwritten.\n");
    for t in targets {
        let Some(guid) = endpoint_guid(t.endpoint_id) else { continue };
        out += &format!("Device: {guid}\n");
        if t.bypass {
            continue;
        }
        let p = t.profile;
        out += &format!("Preamp: {} dB\n", db(p.preamp_db()));
        for (f, g) in BANDS.iter().zip(p.gains) {
            if g != 0. {
                out += &format!("Filter: ON PK Fc {f} Hz Gain {} dB Q 1.41\n", db(g));
            }
        }
        if p.balance != 0. {
            // Balance toward the left turns the right channel down, and the reverse
            let side = if p.balance < 0. { "R" } else { "L" };
            out += &format!("Channel: {side}\nPreamp: {} dB\nChannel: all\n", db(-p.balance.abs()));
        }
        if p.crossfeed {
            out += "Copy: L=0.85*L+0.15*R R=0.85*R+0.15*L\n";
        }
        if p.surround {
            // Virtual speakers at -30 and +30 degrees: a 2x2 matrix of head-related impulse responses
            let ir = |n: &str| hrir_dir.join(n).display().to_string();
            out += "Copy: VLL=L VLR=L VRL=R VRR=R\n";
            for (ch, file) in [("VLL", "l30_left.wav"), ("VLR", "l30_right.wav"), ("VRL", "r30_left.wav"), ("VRR", "r30_right.wav")] {
                out += &format!("Channel: {ch}\nConvolution: {}\n", ir(file));
            }
            out += "Channel: all\nCopy: L=VLL+VRL R=VLR+VRR\n";
        }
    }
    out += "Device: all\n";
    out
}

pub fn live_path() -> PathBuf {
    PathBuf::from(r"C:\ProgramData\SoundBettrCore\live_apo.txt")
}

pub fn write_live(text: &str) -> std::io::Result<()> {
    let p = live_path();
    std::fs::create_dir_all(p.parent().unwrap())?;
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, p)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApoStatus {
    pub installed: bool,
    pub hooked: bool,
}

pub fn config_txt() -> PathBuf {
    PathBuf::from(r"C:\Program Files\EqualizerAPO\config\config.txt")
}

pub fn status() -> ApoStatus {
    let cfg = config_txt();
    let installed = cfg.exists();
    let hooked = installed && std::fs::read_to_string(&cfg).is_ok_and(|t| t.contains("SoundBettrCore\\live_apo.txt"));
    ApoStatus { installed, hooked }
}

/// Adds our Include line to EqualizerAPO's config. Program Files needs admin, so Windows asks the user.
#[cfg(windows)]
pub fn hook() -> Result<(), String> {
    let line = format!("Include: {}", live_path().display());
    let script = format!(
        "Add-Content -LiteralPath '{}' -Value \"`r`n{}\"",
        config_txt().display(),
        line
    );
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!(
            "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-Command',{}",
            ps_quote(&script)
        )])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() && self::status().hooked { Ok(()) } else { Err("EqualizerAPO was not connected. Windows did not get permission.".into()) }
}

#[cfg(windows)]
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Profile;

    const A: &str = "{0.0.0.00000000}.{11111111-2222-3333-4444-555555555555}";
    const B: &str = "{0.0.0.00000000}.{aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee}";

    fn harman() -> Profile {
        let mut p = Profile::flat("p1".into());
        p.gains = [2., 1., -4.2, -3.4, -1., 0., 3.2, 3.8, 1., 0.];
        p
    }

    #[test]
    fn one_block_per_device() {
        let (h, f) = (harman(), Profile::flat("p2".into()));
        let t = render(&[ApoTarget { endpoint_id: A, profile: &h, bypass: false }, ApoTarget { endpoint_id: B, profile: &f, bypass: false }], Path::new("x"));
        let a = t.find("Device: {11111111-2222-3333-4444-555555555555}").unwrap();
        let b = t.find("Device: {aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee}").unwrap();
        assert!(a < b);
        assert!(t[a..b].contains("Preamp: -3.8 dB"));
        assert!(t[a..b].contains("Filter: ON PK Fc 125 Hz Gain -4.2 dB Q 1.41"));
        assert_eq!(t[a..b].matches("Filter:").count(), 8);
        assert!(!t[b..].contains("Filter:"));
        assert!(t.ends_with("Device: all\n"));
    }

    #[test]
    fn balance_left_attenuates_right() {
        let mut p = Profile::flat("p".into());
        p.balance = -2.;
        let t = render(&[ApoTarget { endpoint_id: A, profile: &p, bypass: false }], Path::new("x"));
        assert!(t.contains("Channel: R\nPreamp: -2.0 dB\nChannel: all\n"));
    }

    #[test]
    fn bypass_writes_no_filters() {
        let h = harman();
        let t = render(&[ApoTarget { endpoint_id: A, profile: &h, bypass: true }], Path::new("x"));
        assert!(t.contains("Device: {11111111"));
        assert!(!t.contains("Filter:") && !t.contains("Preamp:"));
    }

    #[test]
    fn names_never_reach_config() {
        let mut p = harman();
        p.name = "x\"; Preamp: +20 dB".into();
        let t = render(&[ApoTarget { endpoint_id: A, profile: &p, bypass: false }], Path::new("x"));
        assert!(!t.contains("+20"));
        let bad = render(&[ApoTarget { endpoint_id: "{0}.Preamp: +20 dB", profile: &p, bypass: false }], Path::new("x"));
        assert!(!bad.contains("+20") && !bad.contains("Filter:"));
    }

    #[test]
    fn surround_adds_convolution_matrix() {
        let mut p = Profile::flat("p".into());
        assert!(!render(&[ApoTarget { endpoint_id: A, profile: &p, bypass: false }], Path::new("x")).contains("Convolution"));
        p.surround = true;
        let t = render(&[ApoTarget { endpoint_id: A, profile: &p, bypass: false }], Path::new("hrir"));
        assert_eq!(t.matches("Convolution: ").count(), 4);
        assert!(t.contains("Copy: L=VLL+VRL R=VLR+VRR"));
    }

    #[test]
    fn crossfeed_adds_copy() {
        let mut p = Profile::flat("p".into());
        p.crossfeed = true;
        assert!(render(&[ApoTarget { endpoint_id: A, profile: &p, bypass: false }], Path::new("x")).contains("Copy: L=0.85*L+0.15*R"));
    }
}
