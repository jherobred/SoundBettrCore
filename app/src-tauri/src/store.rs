//! Devices, their sound profiles and app settings, saved as one JSON file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

pub const DEFAULT_TEMPLATE: &str = "Connected to {device}. {profile} profile on. {battery}";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Bluetooth,
    Usb,
    Wired,
    Builtin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Hue {
    Cobalt,
    Amber,
    Rose,
    Jade,
}

impl Hue {
    pub const ALL: [Hue; 4] = [Hue::Cobalt, Hue::Amber, Hue::Rose, Hue::Jade];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    /// The OS output key: the endpoint id on Windows, used to select the device in EqualizerAPO
    #[serde(default)]
    pub key: String,
    pub kind: Kind,
    pub name: String,
    pub hue: Hue,
    pub announce: bool,
    pub profile_id: String,
    pub last_seen: i64,
    pub is_new: bool,
    /// "a3959" when the earbuds answered a state request
    pub hardware: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Flat,
    Autoeq,
    Hearing,
    Ab,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hearing {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub source: Source,
    pub autoeq_model: Option<String>,
    pub gains: [f32; 10],
    /// None means automatic (minus the largest boost)
    pub preamp: Option<f32>,
    /// dB toward one side, negative is left
    pub balance: f32,
    pub surround: bool,
    pub crossfeed: bool,
    pub hearing: Option<Hearing>,
}

impl Profile {
    pub fn flat(id: String) -> Self {
        Profile {
            id,
            name: "Flat".into(),
            source: Source::Flat,
            autoeq_model: None,
            gains: [0.; 10],
            preamp: None,
            balance: 0.,
            surround: false,
            crossfeed: false,
            hearing: None,
        }
    }

    pub fn preamp_db(&self) -> f32 {
        self.preamp.unwrap_or_else(|| crate::eq::auto_preamp(&self.gains))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: Theme,
    pub motion: bool,
    pub voice: String,
    pub template: String,
    pub autoeq_updated: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: Theme::Light,
            motion: true,
            voice: "af_heart".into(),
            template: DEFAULT_TEMPLATE.into(),
            autoeq_updated: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Store {
    pub devices: Vec<Device>,
    pub profiles: Vec<Profile>,
    pub settings: Settings,
    next_id: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
    Connected,
    NotFound,
}

const FILE: &str = "store.json";

impl Store {
    /// Loads the store. A file that cannot be parsed is kept as `store.json.corrupt` and the app starts empty.
    pub fn load(dir: &Path) -> Store {
        let path = dir.join(FILE);
        let Ok(text) = fs::read_to_string(&path) else { return Store::default() };
        match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(_) => {
                let _ = fs::rename(&path, dir.join("store.json.corrupt"));
                Store::default()
            }
        }
    }

    /// Writes a temp file, then renames it over the old one, so a crash never leaves half a file.
    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let tmp = dir.join("store.json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        fs::rename(tmp, dir.join(FILE))
    }

    pub fn device(&self, id: &str) -> Option<&Device> {
        self.devices.iter().find(|d| d.id == id)
    }

    pub fn device_mut(&mut self, id: &str) -> Option<&mut Device> {
        self.devices.iter_mut().find(|d| d.id == id)
    }

    pub fn upsert_device(&mut self, device: Device) {
        match self.device_mut(&device.id) {
            Some(d) => *d = device,
            None => self.devices.push(device),
        }
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn profile_for(&self, device_id: &str) -> Option<&Profile> {
        self.device(device_id).and_then(|d| self.profile(&d.profile_id))
    }

    pub fn upsert_profile(&mut self, profile: Profile) {
        match self.profiles.iter_mut().find(|p| p.id == profile.id) {
            Some(p) => *p = profile,
            None => self.profiles.push(profile),
        }
    }

    pub fn new_flat_profile(&mut self) -> String {
        self.next_id += 1;
        let id = format!("p{}", self.next_id);
        self.profiles.push(Profile::flat(id.clone()));
        id
    }

    pub fn forget(&mut self, id: &str, current: Option<&str>) -> Result<(), StoreError> {
        if current == Some(id) {
            return Err(StoreError::Connected);
        }
        let i = self.devices.iter().position(|d| d.id == id).ok_or(StoreError::NotFound)?;
        let d = self.devices.remove(i);
        if !self.devices.iter().any(|x| x.profile_id == d.profile_id) {
            self.profiles.retain(|p| p.id != d.profile_id);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(id: &str, profile: &str) -> Device {
        Device {
            id: id.into(),
            key: String::new(),
            kind: Kind::Bluetooth,
            name: id.into(),
            hue: Hue::Cobalt,
            announce: true,
            profile_id: profile.into(),
            last_seen: 0,
            is_new: false,
            hardware: None,
        }
    }

    fn sample() -> Store {
        let mut s = Store::default();
        let p = s.new_flat_profile();
        s.upsert_device(dev("a", &p));
        let q = s.new_flat_profile();
        s.upsert_device(dev("b", &q));
        s
    }

    #[test]
    fn round_trip_keeps_everything() {
        let dir = tempfile::tempdir().unwrap();
        let s = sample();
        s.save(dir.path()).unwrap();
        assert_eq!(Store::load(dir.path()), s);
    }

    #[test]
    fn forget_connected_device_is_refused() {
        assert_eq!(sample().forget("a", Some("a")), Err(StoreError::Connected));
    }

    #[test]
    fn forget_removes_device_and_unused_profile() {
        let mut s = sample();
        s.forget("b", Some("a")).unwrap();
        assert!(s.device("b").is_none());
        assert_eq!(s.profiles.len(), 1);
    }

    #[test]
    fn truncated_file_loads_empty_and_keeps_backup() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(FILE), "{\"devices\":[").unwrap();
        assert_eq!(Store::load(dir.path()), Store::default());
        assert!(dir.path().join("store.json.corrupt").exists());
    }

    #[test]
    fn leftover_tmp_does_not_replace_saved_data() {
        let dir = tempfile::tempdir().unwrap();
        let s = sample();
        s.save(dir.path()).unwrap();
        fs::write(dir.path().join("store.json.tmp"), "{\"devi").unwrap();
        assert_eq!(Store::load(dir.path()), s);
    }

    #[test]
    fn defaults() {
        let s = Settings::default();
        assert_eq!(s.theme, Theme::Light);
        assert!(s.motion);
        assert_eq!(s.voice, "af_heart");
        assert_eq!(s.template, "Connected to {device}. {profile} profile on. {battery}");
    }
}
