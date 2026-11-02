//! Turns an OS audio output into a remembered device, and tracks the connection state.

use crate::store::{Device, Hue, Kind, Store};
use serde::{Deserialize, Serialize};

/// An audio output as the OS reports it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub kind: Kind,
    /// Stable per output: the endpoint id on Windows, the MAC, "usb:<product>", "wired" or "builtin" on Android
    pub key: String,
    pub name: String,
    /// Bluetooth address when known, used for identity and earbud control
    pub address: Option<String>,
}

pub fn device_id(o: &Output) -> String {
    match (&o.kind, &o.address) {
        (Kind::Bluetooth, Some(a)) => format!("bt:{}", a.to_uppercase()),
        (Kind::Bluetooth, None) => format!("bt:{}", o.key),
        (Kind::Usb, _) => format!("usb:{}", o.key),
        (Kind::Wired, _) => format!("wired:{}", o.key),
        (Kind::Builtin, _) => format!("builtin:{}", o.key),
    }
}

/// Only these names can be an R50i NC or P30i. "Soundcore" alone is any Soundcore product.
pub fn is_a3959_name(name: &str) -> bool {
    let n = name.to_lowercase();
    ["r50i", "p30i", "a3959"].iter().any(|k| n.contains(k))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ConnState {
    Searching,
    #[serde(rename_all = "camelCase")]
    Recognized { device_id: String },
    #[serde(rename_all = "camelCase")]
    New { device_id: String },
    /// Several saved wired sets share one jack and the OS cannot tell them apart
    #[serde(rename_all = "camelCase")]
    Choose { device_ids: Vec<String> },
    Error { cause: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub state: ConnState,
    pub announce: bool,
}

pub struct Tracker {
    pub debounce_ms: u64,
    current: Option<String>,
    pending: Option<Output>,
    dropped: Option<(String, u64)>,
}

impl Default for Tracker {
    fn default() -> Self {
        Tracker { debounce_ms: 1500, current: None, pending: None, dropped: None }
    }
}

impl Tracker {
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub fn on_output(&mut self, store: &mut Store, o: Option<Output>, now_ms: u64) -> Option<Transition> {
        let Some(o) = o else {
            self.pending = None;
            let id = self.current.take()?;
            self.dropped = Some((id, now_ms));
            return Some(Transition { state: ConnState::Searching, announce: false });
        };
        let base = device_id(&o);
        if o.kind == Kind::Wired {
            let ids: Vec<String> = store
                .devices
                .iter()
                .filter(|d| d.id == base || d.id.starts_with(&format!("{base}#")))
                .map(|d| d.id.clone())
                .collect();
            if ids.len() > 1 {
                if self.current.as_ref().is_some_and(|c| ids.contains(c)) {
                    return None;
                }
                self.current = None;
                self.pending = Some(o);
                return Some(Transition { state: ConnState::Choose { device_ids: ids }, announce: false });
            }
        }
        if self.current.as_deref() == Some(&base) {
            return None;
        }
        Some(self.enter(store, &o, base, now_ms))
    }

    /// The user picked which wired set is plugged in. None means a set not saved yet.
    pub fn choose(&mut self, store: &mut Store, id: Option<&str>, now_ms: u64) -> Option<Transition> {
        let o = self.pending.take()?;
        let id = match id {
            Some(id) => id.to_string(),
            None => {
                let base = device_id(&o);
                (2..).map(|n| format!("{base}#{n}")).find(|c| store.device(c).is_none()).unwrap()
            }
        };
        Some(self.enter(store, &o, id, now_ms))
    }

    /// Treat the current wired device as a different set (Android cannot tell them apart).
    pub fn not_this_set(&mut self, store: &mut Store, o: Output, now_ms: u64) -> Option<Transition> {
        self.current = None;
        self.pending = Some(o);
        self.choose(store, None, now_ms)
    }

    fn enter(&mut self, store: &mut Store, o: &Output, id: String, now_ms: u64) -> Transition {
        let flapped = matches!(&self.dropped, Some((d, t)) if *d == id && now_ms.saturating_sub(*t) < self.debounce_ms);
        self.dropped = None;
        self.current = Some(id.clone());
        match store.device_mut(&id) {
            Some(d) => {
                d.last_seen = now_ms as i64;
                d.key = o.key.clone();
                let state = if d.is_new { ConnState::New { device_id: id } } else { ConnState::Recognized { device_id: id } };
                let announce = !flapped && matches!(state, ConnState::Recognized { .. }) && d.announce;
                Transition { state, announce }
            }
            None => {
                let profile_id = store.new_flat_profile();
                let used: Vec<Hue> = store.devices.iter().map(|d| d.hue).collect();
                let hue = Hue::ALL.into_iter().find(|h| !used.contains(h)).unwrap_or(Hue::ALL[store.devices.len() % 4]);
                let name = unique_name(store, &o.name);
                store.devices.push(Device {
                    id: id.clone(),
                    key: o.key.clone(),
                    kind: o.kind,
                    name,
                    hue,
                    announce: true,
                    profile_id,
                    last_seen: now_ms as i64,
                    is_new: true,
                    hardware: None,
                });
                Transition { state: ConnState::New { device_id: id }, announce: false }
            }
        }
    }
}

fn unique_name(store: &Store, name: &str) -> String {
    let taken = |n: &str| store.devices.iter().any(|d| d.name == n);
    if !taken(name) {
        return name.to_string();
    }
    (2..).map(|i| format!("{name} {i}")).find(|n| !taken(n)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bt(addr: &str, name: &str) -> Output {
        Output { kind: Kind::Bluetooth, key: addr.into(), name: name.into(), address: Some(addr.into()) }
    }

    fn wired() -> Output {
        Output { kind: Kind::Wired, key: "wired".into(), name: "Wired headset".into(), address: None }
    }

    #[test]
    fn soundcore_name_alone_is_not_a3959() {
        assert!(!is_a3959_name("Soundcore Life Q30"));
        assert!(is_a3959_name("Soundcore R50i NC"));
        assert!(is_a3959_name("Jhems's P30i"));
    }

    #[test]
    fn unknown_output_becomes_new_with_flat_profile() {
        let mut s = Store::default();
        let t = Tracker::default().on_output(&mut s, Some(bt("aa:bb", "R50i NC")), 0).unwrap();
        assert_eq!(t, Transition { state: ConnState::New { device_id: "bt:AA:BB".into() }, announce: false });
        let d = s.device("bt:AA:BB").unwrap();
        assert!(d.is_new);
        assert_eq!(d.name, "R50i NC");
        assert_eq!(s.profile_for(&d.id).unwrap().name, "Flat");
    }

    #[test]
    fn known_output_is_recognized_and_announces() {
        let mut s = Store::default();
        let mut t = Tracker::default();
        t.on_output(&mut s, Some(bt("aa", "R50i")), 0);
        s.device_mut("bt:AA").unwrap().is_new = false;
        t.on_output(&mut s, None, 10_000);
        let r = t.on_output(&mut s, Some(bt("aa", "R50i")), 20_000).unwrap();
        assert_eq!(r, Transition { state: ConnState::Recognized { device_id: "bt:AA".into() }, announce: true });
    }

    #[test]
    fn flapping_within_debounce_announces_once() {
        let mut s = Store::default();
        let mut t = Tracker::default();
        t.on_output(&mut s, Some(bt("aa", "R50i")), 0);
        s.device_mut("bt:AA").unwrap().is_new = false;
        t.on_output(&mut s, None, 100);
        let first = t.on_output(&mut s, Some(bt("aa", "R50i")), 5_000).unwrap();
        t.on_output(&mut s, None, 5_400);
        let second = t.on_output(&mut s, Some(bt("aa", "R50i")), 5_900).unwrap();
        assert!(first.announce);
        assert!(!second.announce);
        assert_eq!(second.state, ConnState::Recognized { device_id: "bt:AA".into() });
    }

    #[test]
    fn same_name_two_addresses_two_devices() {
        let mut s = Store::default();
        let mut t = Tracker::default();
        t.on_output(&mut s, Some(bt("aa", "R50i NC")), 0);
        t.on_output(&mut s, Some(bt("bb", "R50i NC")), 1);
        assert_eq!(s.devices.len(), 2);
        assert_eq!(s.device("bt:BB").unwrap().name, "R50i NC 2");
        assert_ne!(s.devices[0].hue, s.devices[1].hue);
    }

    #[test]
    fn none_output_is_searching() {
        let mut s = Store::default();
        let mut t = Tracker::default();
        t.on_output(&mut s, Some(bt("aa", "x")), 0);
        assert_eq!(t.on_output(&mut s, None, 1).unwrap().state, ConnState::Searching);
        assert_eq!(t.current(), None);
    }

    #[test]
    fn wired_with_two_saved_asks_choose() {
        let mut s = Store::default();
        let mut t = Tracker::default();
        t.on_output(&mut s, Some(wired()), 0);
        let made = t.not_this_set(&mut s, wired(), 1).unwrap();
        assert_eq!(made.state, ConnState::New { device_id: "wired:wired#2".into() });
        t.on_output(&mut s, None, 2);
        let r = t.on_output(&mut s, Some(wired()), 3).unwrap();
        assert_eq!(r.state, ConnState::Choose { device_ids: vec!["wired:wired".into(), "wired:wired#2".into()] });
        s.device_mut("wired:wired").unwrap().is_new = false;
        let picked = t.choose(&mut s, Some("wired:wired"), 4).unwrap();
        assert_eq!(picked, Transition { state: ConnState::Recognized { device_id: "wired:wired".into() }, announce: true });
    }
}
