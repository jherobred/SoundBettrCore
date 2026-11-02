//! The bundled AutoEq index: measured corrections for headphone models.

use crate::eq::{Filter, FilterKind};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "m")]
    pub model: String,
    /// The measurement source and form, like "oratory1990 in-ear"
    #[serde(rename = "s")]
    pub source: String,
    #[serde(rename = "p")]
    pub preamp: f32,
    #[serde(rename = "f")]
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    pub index: usize,
    pub model: String,
    pub source: String,
}

#[derive(Default)]
pub struct Index(pub Vec<Entry>);

impl Index {
    /// Reads the index, gzipped or plain (Android unpacks .gz assets when it builds the APK).
    pub fn load_gz(path: &Path) -> std::io::Result<Index> {
        let bytes = std::fs::read(path)?;
        let mut text = String::new();
        if bytes.starts_with(&[0x1f, 0x8b]) {
            flate2::read::GzDecoder::new(&bytes[..]).read_to_string(&mut text)?;
        } else {
            text = String::from_utf8(bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        }
        Ok(Index(serde_json::from_str(&text)?))
    }

    /// Every word of the query must appear in the model name, in any order.
    pub fn search(&self, query: &str, limit: usize) -> Vec<Hit> {
        let words: Vec<String> = query.to_lowercase().split_whitespace().map(String::from).collect();
        if words.is_empty() {
            return vec![];
        }
        self.0
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                let m = e.model.to_lowercase();
                words.iter().all(|w| m.contains(w.as_str()))
            })
            .take(limit)
            .map(|(index, e)| Hit { index, model: e.model.clone(), source: e.source.clone() })
            .collect()
    }
}

/// Parses AutoEq's ParametricEQ.txt.
pub fn parse_parametric_eq(model: &str, source: &str, txt: &str) -> Result<Entry, String> {
    let mut preamp = 0.;
    let mut filters = vec![];
    for line in txt.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let t: Vec<&str> = line.split_whitespace().collect();
        if t[0] == "Preamp:" {
            preamp = t.get(1).and_then(|v| v.parse().ok()).ok_or(format!("bad preamp: {line}"))?;
        } else if t[0] == "Filter" {
            if t.get(2) != Some(&"ON") {
                continue;
            }
            let kind = match t.get(3) {
                Some(&"PK") => FilterKind::Peak,
                Some(&"LSC") | Some(&"LS") => FilterKind::LowShelf,
                Some(&"HSC") | Some(&"HS") => FilterKind::HighShelf,
                other => return Err(format!("unsupported filter {other:?}")),
            };
            let num = |key: &str| -> Result<f32, String> {
                let i = t.iter().position(|w| *w == key).ok_or(format!("missing {key}: {line}"))?;
                t.get(i + 1).and_then(|v| v.parse().ok()).ok_or(format!("bad {key}: {line}"))
            };
            filters.push(Filter { kind, fc: num("Fc")?, gain: num("Gain")?, q: num("Q")? });
        }
    }
    if filters.is_empty() {
        return Err("no filters".into());
    }
    Ok(Entry { model: model.into(), source: source.into(), preamp, filters })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Preamp: -6.2 dB
Filter 1: ON LSC Fc 105 Hz Gain 5.3 dB Q 0.70
Filter 2: ON PK Fc 2000 Hz Gain -3.1 dB Q 1.20
Filter 3: ON HSC Fc 10000 Hz Gain 2.0 dB Q 0.70
";

    #[test]
    fn parses_parametric_eq_file() {
        let e = parse_parametric_eq("Soundcore R50i NC", "in-ear", SAMPLE).unwrap();
        assert_eq!(e.preamp, -6.2);
        assert_eq!(e.filters.len(), 3);
        assert_eq!(e.filters[0], Filter { kind: FilterKind::LowShelf, fc: 105., gain: 5.3, q: 0.7 });
        assert_eq!(e.filters[1].kind, FilterKind::Peak);
    }

    #[test]
    fn search_is_case_insensitive_any_order_and_limited() {
        let e = |m: &str| Entry { model: m.into(), source: "in-ear".into(), preamp: 0., filters: vec![] };
        let idx = Index(vec![e("Soundcore Life P3"), e("Soundcore R50i NC"), e("Sony WF-1000XM5"), e("Soundcore Space Q45")]);
        assert_eq!(idx.search("R50I", 5)[0].model, "Soundcore R50i NC");
        assert_eq!(idx.search("r50i soundcore", 5).len(), 1);
        assert_eq!(idx.search("soundcore", 2).len(), 2);
        assert!(idx.search("   ", 5).is_empty());
    }
}
