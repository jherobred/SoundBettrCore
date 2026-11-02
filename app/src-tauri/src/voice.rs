//! The connection announcement: template text, Kokoro speech, and a WAV cache so repeats play at once.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub struct Pcm {
    pub samples: Vec<f32>,
    pub rate: u32,
}

pub trait Synth: Send + Sync {
    fn render(&self, text: &str, sid: i32) -> Result<Pcm, String>;
}

/// The 8 English Kokoro voices offered, with speaker ids read from the model's id2speaker metadata.
pub const VOICES: [(&str, &str, &str, i32); 8] = [
    ("af_heart", "Heart", "American, warm", 3),
    ("af_bella", "Bella", "American, bright", 2),
    ("bf_emma", "Emma", "British", 21),
    ("af_nicole", "Nicole", "American, soft", 6),
    ("am_michael", "Michael", "American", 16),
    ("am_fenrir", "Fenrir", "American, deep", 14),
    ("bm_george", "George", "British", 26),
    ("am_puck", "Puck", "American, lively", 18),
];

pub fn sid(voice: &str) -> i32 {
    VOICES.iter().find(|v| v.0 == voice).map_or(3, |v| v.3)
}

pub fn fill_template(tpl: &str, device: &str, profile: &str, battery: Option<(u8, u8)>) -> String {
    let batt = battery.map(|(l, r)| format!("Battery {} percent.", l.min(r))).unwrap_or_default();
    tpl.replace("{device}", device)
        .replace("{profile}", profile)
        .replace("{battery}", &batt)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct KokoroSynth(sherpa_onnx::OfflineTts);

impl KokoroSynth {
    /// `dir` holds model.int8.onnx, voices.bin, tokens.txt, lexicon-us-en.txt and espeak-ng-data.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let p = |n: &str| Some(dir.join(n).display().to_string());
        let mut cfg = sherpa_onnx::OfflineTtsConfig::default();
        cfg.model.kokoro = sherpa_onnx::OfflineTtsKokoroModelConfig {
            model: p("model.int8.onnx"),
            voices: p("voices.bin"),
            tokens: p("tokens.txt"),
            data_dir: p("espeak-ng-data"),
            lexicon: p("lexicon-us-en.txt"),
            lang: Some("en-us".into()),
            length_scale: 1.0,
            ..Default::default()
        };
        cfg.model.num_threads = 2;
        cfg.max_num_sentences = 2;
        sherpa_onnx::OfflineTts::create(&cfg).map(KokoroSynth).ok_or_else(|| format!("could not load Kokoro from {}", dir.display()))
    }
}

impl Synth for KokoroSynth {
    fn render(&self, text: &str, sid: i32) -> Result<Pcm, String> {
        let params = sherpa_onnx::GenerationConfig { sid, speed: 1.0, ..Default::default() };
        let audio = self.0.generate_with_config(text, &params, None::<fn(&[f32], f32) -> bool>).ok_or("speech failed")?;
        Ok(Pcm { samples: audio.samples().to_vec(), rate: audio.sample_rate() as u32 })
    }
}

pub struct Announcer {
    pub synth: Box<dyn Synth>,
    pub cache_dir: PathBuf,
}

impl Announcer {
    /// Renders once per (voice, text) and reuses the WAV after that.
    pub fn wav_for(&self, text: &str, voice: &str) -> Result<PathBuf, String> {
        let mut h = DefaultHasher::new();
        (voice, text).hash(&mut h);
        let path = self.cache_dir.join(format!("{:016x}.wav", h.finish()));
        if path.exists() {
            return Ok(path);
        }
        let pcm = self.synth.render(text, sid(voice))?;
        std::fs::create_dir_all(&self.cache_dir).map_err(|e| e.to_string())?;
        write_wav(&path, &pcm).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

/// 16-bit PCM mono WAV.
pub fn write_wav(path: &Path, pcm: &Pcm) -> std::io::Result<()> {
    let data: Vec<u8> = pcm.samples.iter().flat_map(|s| ((s.clamp(-1., 1.) * 32767.) as i16).to_le_bytes()).collect();
    let mut b = Vec::with_capacity(44 + data.len());
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&pcm.rate.to_le_bytes());
    b.extend_from_slice(&(pcm.rate * 2).to_le_bytes());
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&(data.len() as u32).to_le_bytes());
    b.extend_from_slice(&data);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, b)?;
    std::fs::rename(tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Counting(Arc<AtomicUsize>);
    impl Synth for Counting {
        fn render(&self, _: &str, _: i32) -> Result<Pcm, String> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Pcm { samples: vec![0.; 100], rate: 24000 })
        }
    }

    const TPL: &str = crate::store::DEFAULT_TEMPLATE;

    #[test]
    fn fills_all_tokens() {
        assert_eq!(
            fill_template(TPL, "Jhems's R50i NC", "Harman fix", Some((82, 78))),
            "Connected to Jhems's R50i NC. Harman fix profile on. Battery 78 percent."
        );
    }

    #[test]
    fn no_battery_leaves_no_gap() {
        assert_eq!(fill_template(TPL, "Wired earphones", "Flat", None), "Connected to Wired earphones. Flat profile on.");
    }

    #[test]
    fn cache_hit_skips_render_and_voice_change_misses() {
        let dir = tempfile::tempdir().unwrap();
        let n = Arc::new(AtomicUsize::new(0));
        let a = Announcer { synth: Box::new(Counting(n.clone())), cache_dir: dir.path().into() };
        let p1 = a.wav_for("hello", "af_heart").unwrap();
        let p2 = a.wav_for("hello", "af_heart").unwrap();
        assert_eq!(p1, p2);
        assert_eq!(n.load(Ordering::SeqCst), 1);
        a.wav_for("hello", "bf_emma").unwrap();
        assert_eq!(n.load(Ordering::SeqCst), 2);
        assert_eq!(std::fs::read(p1).unwrap().len(), 44 + 200);
    }

    #[test]
    fn voice_ids_map_to_model_speakers() {
        assert_eq!(sid("af_heart"), 3);
        assert_eq!(sid("bf_emma"), 21);
        assert_eq!(sid("bm_george"), 26);
        assert_eq!(sid("unknown"), 3);
    }

    #[test]
    #[ignore = "needs the Kokoro model; set SBC_KOKORO to its folder"]
    fn kokoro_renders_one_line() {
        let dir = std::env::var("SBC_KOKORO").unwrap();
        let k = KokoroSynth::load(Path::new(&dir)).unwrap();
        let pcm = k.render("Connected to Jhems's R50i NC. Harman fix profile on.", 3).unwrap();
        assert_eq!(pcm.rate, 24000);
        assert!(pcm.samples.len() > 24000 && pcm.samples.iter().all(|s| s.is_finite()));
        write_wav(&Path::new(&dir).join("../heart.wav"), &pcm).unwrap();
    }
}
