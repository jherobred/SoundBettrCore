//! Hearing test (down 10 dB when heard, up 5 dB when missed) and the blind A/B session.

use crate::eq::snap;
use serde::Serialize;
use std::collections::HashMap;

pub const HT_FREQS: [f32; 6] = [250., 500., 1000., 2000., 4000., 8000.];
const BAND_OF_FREQ: [usize; 6] = [3, 4, 5, 6, 7, 8];
const START_DB: f32 = -20.;
const FLOOR_DB: f32 = -80.;
const MAX_STEPS: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Ear {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum HtStep {
    Play { ear: Ear, freq: f32, db: f32, index: usize, total: usize },
    Done { left: Vec<f32>, right: Vec<f32> },
}

pub struct Staircase {
    ear: Ear,
    fi: usize,
    level: f32,
    after_miss: bool,
    ascending_hits: HashMap<i32, u8>,
    steps: usize,
    left: Vec<f32>,
    right: Vec<f32>,
}

impl Staircase {
    pub fn new() -> (Staircase, HtStep) {
        let s = Staircase { ear: Ear::Left, fi: 0, level: START_DB, after_miss: false, ascending_hits: HashMap::new(), steps: 0, left: vec![], right: vec![] };
        let step = s.play();
        (s, step)
    }

    fn play(&self) -> HtStep {
        let index = if self.ear == Ear::Left { self.fi } else { HT_FREQS.len() + self.fi };
        HtStep::Play { ear: self.ear, freq: HT_FREQS[self.fi], db: self.level, index, total: HT_FREQS.len() * 2 }
    }

    pub fn answer(&mut self, heard: bool) -> HtStep {
        self.steps += 1;
        if heard {
            if self.after_miss {
                let n = self.ascending_hits.entry(self.level as i32).or_insert(0);
                *n += 1;
                if *n >= 2 {
                    return self.record(self.level);
                }
            }
            if self.level - 10. < FLOOR_DB {
                return self.record(FLOOR_DB);
            }
            self.level -= 10.;
            self.after_miss = false;
        } else {
            if self.level >= 0. {
                return self.record(0.);
            }
            self.level += 5.;
            self.after_miss = true;
        }
        if self.steps >= MAX_STEPS {
            return self.record(self.level);
        }
        self.play()
    }

    fn record(&mut self, threshold: f32) -> HtStep {
        match self.ear {
            Ear::Left => self.left.push(threshold),
            Ear::Right => self.right.push(threshold),
        }
        self.fi += 1;
        if self.fi == HT_FREQS.len() {
            if self.ear == Ear::Right {
                return HtStep::Done { left: self.left.clone(), right: self.right.clone() };
            }
            self.ear = Ear::Right;
            self.fi = 0;
        }
        self.level = START_DB;
        self.after_miss = false;
        self.ascending_hits.clear();
        self.steps = 0;
        self.play()
    }
}

/// Boost each band by a third of how much worse it is than the best band (max +6 dB),
/// and lean the balance toward the weaker ear.
pub fn apply_hearing(left: &[f32], right: &[f32]) -> ([f32; 10], f32) {
    let best = left.iter().chain(right).copied().fold(f32::MAX, f32::min);
    let mut gains = [0.; 10];
    for (i, band) in BAND_OF_FREQ.iter().enumerate() {
        let avg = (left[i] + right[i]) / 2.;
        gains[*band] = snap((avg - best) / 3., 0., 6.);
    }
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    let balance = snap(-(mean(left) - mean(right)) / 4., -6., 6.);
    (gains, balance)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    X,
    Y,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbResult {
    pub winner: String,
    pub wins: u8,
    pub rounds: u8,
}

pub struct Ab {
    pub a: String,
    pub b: String,
    x_is_a: [bool; 6],
    picks: Vec<String>,
}

impl Ab {
    pub fn new(a: String, b: String, x_is_a: [bool; 6]) -> Ab {
        Ab { a, b, x_is_a, picks: vec![] }
    }

    pub fn round(&self) -> usize {
        self.picks.len()
    }

    pub fn profile_for(&self, side: Side) -> &str {
        let x_is_a = self.x_is_a[self.round().min(5)];
        if (side == Side::X) == x_is_a { &self.a } else { &self.b }
    }

    pub fn pick(&mut self, side: Side) -> Option<AbResult> {
        if self.picks.len() == 6 {
            return None;
        }
        self.picks.push(self.profile_for(side).to_string());
        (self.picks.len() == 6).then(|| {
            let a_wins = self.picks.iter().filter(|p| **p == self.a).count() as u8;
            let (winner, wins) = if a_wins * 2 >= 6 { (self.a.clone(), a_wins) } else { (self.b.clone(), 6 - a_wins) };
            AbResult { winner, wins, rounds: 6 }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(step: &HtStep) -> (Ear, f32, f32) {
        match step {
            HtStep::Play { ear, freq, db, .. } => (*ear, *freq, *db),
            HtStep::Done { .. } => panic!("done early"),
        }
    }

    #[test]
    fn starts_left_250_at_minus_20() {
        let (_, s) = Staircase::new();
        assert_eq!(at(&s), (Ear::Left, 250., -20.));
    }

    #[test]
    fn heard_steps_down_10_missed_steps_up_5() {
        let (mut st, _) = Staircase::new();
        assert_eq!(at(&st.answer(true)).2, -30.);
        assert_eq!(at(&st.answer(false)).2, -25.);
    }

    #[test]
    fn threshold_is_level_heard_twice_ascending() {
        let (mut st, _) = Staircase::new();
        for heard in [true, true, false, true, false, false] {
            st.answer(heard);
        }
        let next = st.answer(true);
        assert_eq!(at(&next), (Ear::Left, 500., -20.));
        assert_eq!(st.left, vec![-35.]);
    }

    #[test]
    fn floor_and_ceiling() {
        let (mut st, _) = Staircase::new();
        while st.left.is_empty() {
            st.answer(true);
        }
        assert_eq!(st.left[0], -80.);
        while st.left.len() < 2 {
            st.answer(false);
        }
        assert_eq!(st.left[1], 0.);
    }

    #[test]
    fn left_then_right_then_done() {
        let (mut st, _) = Staircase::new();
        let mut last = None;
        for _ in 0..500 {
            let s = st.answer(false);
            if let HtStep::Done { left, right } = s {
                last = Some((left.len(), right.len()));
                break;
            }
        }
        assert_eq!(last, Some((6, 6)));
    }

    #[test]
    fn apply_boosts_relative_to_best() {
        let ear = [-40., -40., -40., -40., -25., -10.];
        let (g, bal) = apply_hearing(&ear, &ear);
        assert_eq!(g[7], 5.);
        assert_eq!(g[8], 6.);
        assert!(g[3..=6].iter().all(|v| *v == 0.));
        assert_eq!(bal, 0.);
    }

    #[test]
    fn apply_balance_toward_worse_ear() {
        let right = [-40.; 6];
        let left = [-32.; 6];
        assert_eq!(apply_hearing(&left, &right).1, -2.);
    }

    #[test]
    fn ab_maps_and_reveals_winner() {
        let mut ab = Ab::new("a".into(), "b".into(), [true, false, true, false, true, false]);
        assert_eq!(ab.profile_for(Side::X), "a");
        let mut r = None;
        for _ in 0..6 {
            r = ab.pick(Side::X);
        }
        assert_eq!(r, Some(AbResult { winner: "a".into(), wins: 3, rounds: 6 }));
        let mut ab = Ab::new("a".into(), "b".into(), [true, false, true, false, true, false]);
        for i in 0..6 {
            r = ab.pick(if i % 2 == 0 { Side::X } else { Side::Y });
        }
        assert_eq!(r.unwrap().wins, 6);
    }
}
