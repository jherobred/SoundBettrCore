//! The app's state and every action on it. Tauri commands (and Android's background service) call in here.

use crate::a3959::{self, A3959SoundModes, A3959State, ButtonAction, ButtonPressKind, EarbudSide};
use crate::apo::ApoStatus;
use crate::autoeq::{Hit, Index};
use crate::device::{is_a3959_name, ConnState, Output, Tracker};
use crate::eq::fit_10band;
use crate::hearing::{apply_hearing, Ab, AbResult, HtStep, Side, Staircase};
use crate::rfcomm::{self, CommandResult, Hardware};
use crate::store::{Device, Hearing, Kind, Profile, Settings, Source, Store, StoreError};
use crate::voice::{fill_template, Announcer, VOICES};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const ANNOUNCE_DELAY: Duration = Duration::from_millis(1500);

pub trait Platform: Send + Sync {
    fn name(&self) -> &'static str;
    /// Puts the right sound on the output. `profile` is what the current device should play now.
    fn apply(&self, store: &Store, current: Option<&Device>, profile: Option<&Profile>, bypass: bool) -> Result<(), String>;
    fn play(&self, wav: &Path);
    fn hardware(&self, device: &Device) -> Option<Box<dyn Hardware>>;
    fn emit(&self, event: &str, payload: serde_json::Value);
    fn apo(&self) -> Option<ApoStatus> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum HwCommand {
    SoundModes { modes: A3959SoundModes },
    Gaming { on: bool },
    Eq { gains_tenths_db: [i16; 8] },
    Button { side: EarbudSide, press: ButtonPressKind, action: ButtonAction },
    Refresh,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub platform: &'static str,
    pub devices: Vec<Device>,
    pub profiles: Vec<Profile>,
    pub settings: Settings,
    pub conn: ConnState,
    pub current: Option<String>,
    pub apply_error: Option<String>,
    pub apo: Option<ApoStatus>,
    pub speaking: bool,
    pub voice_error: Option<String>,
    pub voices: Vec<VoiceInfo>,
    pub hardware: Option<A3959State>,
    pub hearing: Option<HtStep>,
    pub ab_round: Option<usize>,
    pub autoeq_entries: usize,
}

pub struct Core {
    dir: PathBuf,
    pub store: Store,
    tracker: Tracker,
    conn: ConnState,
    last_output: Option<Output>,
    bypass: bool,
    override_profile: Option<Profile>,
    apply_error: Option<String>,
    hearing: Option<(Staircase, HtStep)>,
    ab: Option<(String, Ab)>,
    autoeq: Index,
    announcer: Option<Arc<Announcer>>,
    voice_error: Option<String>,
    speaking: bool,
    hardware: HashMap<String, (Box<dyn Hardware>, Option<A3959State>)>,
    platform: Arc<dyn Platform>,
}

pub type Shared = Arc<Mutex<Core>>;

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

impl Core {
    pub fn new(dir: PathBuf, autoeq: Index, announcer: Result<Announcer, String>, platform: Arc<dyn Platform>) -> Core {
        let (announcer, voice_error) = match announcer {
            Ok(a) => (Some(Arc::new(a)), None),
            Err(e) => (None, Some(e)),
        };
        Core {
            store: Store::load(&dir),
            dir,
            tracker: Tracker::default(),
            conn: ConnState::Searching,
            last_output: None,
            bypass: false,
            override_profile: None,
            apply_error: None,
            hearing: None,
            ab: None,
            autoeq,
            announcer,
            voice_error,
            speaking: false,
            hardware: HashMap::new(),
            platform,
        }
    }

    /// The voice can arrive after startup (Android unpacks the model on first run).
    pub fn set_announcer(&mut self, a: Result<Announcer, String>) {
        match a {
            Ok(a) => {
                self.announcer = Some(Arc::new(a));
                self.voice_error = None;
            }
            Err(e) => self.voice_error = Some(e),
        }
        self.changed();
    }

    pub fn snapshot(&self) -> Snapshot {
        let current = self.tracker.current().map(String::from);
        Snapshot {
            platform: self.platform.name(),
            devices: self.store.devices.clone(),
            profiles: self.store.profiles.clone(),
            settings: self.store.settings.clone(),
            conn: self.conn.clone(),
            hardware: current.as_ref().and_then(|c| self.hardware.get(c)).and_then(|h| h.1.clone()),
            current,
            apply_error: self.apply_error.clone(),
            apo: self.platform.apo(),
            speaking: self.speaking,
            voice_error: self.voice_error.clone(),
            voices: VOICES.iter().map(|v| VoiceInfo { id: v.0, name: v.1, desc: v.2 }).collect(),
            hearing: self.hearing.as_ref().map(|h| h.1.clone()),
            ab_round: self.ab.as_ref().map(|a| a.1.round()),
            autoeq_entries: self.autoeq.0.len(),
        }
    }

    fn save(&self) {
        if let Err(e) = self.store.save(&self.dir) {
            eprintln!("could not save: {e}");
        }
    }

    fn current_device(&self) -> Option<&Device> {
        self.tracker.current().and_then(|c| self.store.device(c))
    }

    /// Puts the current sound on the output and tells the UI.
    fn apply(&mut self) {
        let dev = self.current_device().cloned();
        let profile = self.override_profile.clone().or_else(|| dev.as_ref().and_then(|d| self.store.profile(&d.profile_id).cloned()));
        self.apply_error = self.platform.apply(&self.store, dev.as_ref(), profile.as_ref(), self.bypass).err();
    }

    fn changed(&mut self) {
        self.save();
        self.platform.emit("snapshot", serde_json::to_value(self.snapshot()).unwrap());
    }

    fn profile_mut_for(&mut self, device_id: &str) -> Result<&mut Profile, String> {
        let pid = self.store.device(device_id).ok_or("No such device")?.profile_id.clone();
        self.store.profiles.iter_mut().find(|p| p.id == pid).ok_or_else(|| "No profile".into())
    }

    fn template_text(&self, device_id: &str) -> Option<String> {
        let d = self.store.device(device_id)?;
        let p = self.store.profile(&d.profile_id)?;
        let batt = self.hardware.get(device_id).and_then(|h| h.1.as_ref()).map(|s| (s.left_battery_percent, s.right_battery_percent));
        Some(fill_template(&self.store.settings.template, &d.name, &p.name, batt))
    }
}

pub fn emit_snapshot(core: &Core) {
    core.platform.emit("snapshot", serde_json::to_value(core.snapshot()).unwrap());
}

/// The OS reported a new default output (or none).
pub fn on_output(shared: &Shared, o: Option<Output>) {
    let mut c = shared.lock().unwrap();
    c.last_output = o.clone();
    let t = {
        let Core { store, tracker, .. } = &mut *c;
        tracker.on_output(store, o.clone(), now_ms())
    };
    let Some(t) = t else { return };
    c.conn = t.state.clone();
    c.ab = None;
    c.override_profile = None;
    if c.hearing.take().is_some() {
        c.bypass = false;
    }
    c.apply();
    c.changed();
    enter(shared, c, t.state, t.announce);
}

fn enter(shared: &Shared, c: std::sync::MutexGuard<Core>, state: ConnState, announce: bool) {
    let id = match state {
        ConnState::Recognized { device_id } | ConnState::New { device_id } => device_id,
        _ => return,
    };
    let candidate = c.store.device(&id).is_some_and(|d| d.kind == Kind::Bluetooth && is_a3959_name(&d.name)) || c.last_output.as_ref().is_some_and(|o| is_a3959_name(&o.name));
    drop(c);
    let shared = shared.clone();
    std::thread::spawn(move || {
        let start = Instant::now();
        if candidate {
            probe_hardware(&shared, &id);
        }
        if announce {
            std::thread::sleep(ANNOUNCE_DELAY.saturating_sub(start.elapsed()));
            announce_now(&shared, &id, true);
        }
    });
}

/// Asks possible R50i earbuds for their state. Only an answer marks the device as having hardware control.
fn probe_hardware(shared: &Shared, id: &str) {
    let hw = {
        let c = shared.lock().unwrap();
        c.store.device(id).and_then(|d| c.platform.hardware(d))
    };
    let Some(mut hw) = hw else { return };
    let state = hw.exchange(&a3959::build_state_request_packet()).ok().flatten().and_then(|r| a3959::parse_a3959_state_packet(&r).ok());
    let mut c = shared.lock().unwrap();
    if let Some(d) = c.store.device_mut(id) {
        d.hardware = state.is_some().then(|| "a3959".into());
    }
    if state.is_some() {
        c.hardware.insert(id.to_string(), (hw, state));
    }
    c.changed();
}

/// Speaks the announcement for a device. `only_if_current` skips it when the output changed meanwhile.
pub fn announce_now(shared: &Shared, id: &str, only_if_current: bool) {
    let (text, voice, announcer, platform) = {
        let c = shared.lock().unwrap();
        if only_if_current && c.tracker.current() != Some(id) {
            return;
        }
        let Some(text) = c.template_text(id) else { return };
        (text, c.store.settings.voice.clone(), c.announcer.clone(), c.platform.clone())
    };
    speak(shared, &text, &voice, announcer, platform);
}

pub fn speak_text(shared: &Shared, text: &str, voice: &str) {
    let (announcer, platform) = {
        let c = shared.lock().unwrap();
        (c.announcer.clone(), c.platform.clone())
    };
    speak(shared, text, voice, announcer, platform);
}

fn speak(shared: &Shared, text: &str, voice: &str, announcer: Option<Arc<Announcer>>, platform: Arc<dyn Platform>) {
    let Some(a) = announcer else { return };
    set_speaking(shared, true);
    match a.wav_for(text, voice) {
        Ok(path) => {
            platform.play(&path);
            std::thread::sleep(wav_duration(&path));
        }
        Err(e) => shared.lock().unwrap().voice_error = Some(e),
    }
    set_speaking(shared, false);
}

fn set_speaking(shared: &Shared, on: bool) {
    let mut c = shared.lock().unwrap();
    c.speaking = on;
    c.platform.emit("speaking", serde_json::Value::Bool(on));
    if !on {
        emit_snapshot(&c);
    }
}

fn wav_duration(path: &Path) -> Duration {
    let len = std::fs::metadata(path).map_or(0, |m| m.len()).saturating_sub(44);
    Duration::from_millis(len * 1000 / (24_000 * 2))
}

// ---- actions the UI calls ----

impl Core {
    pub fn update_device(&mut self, d: Device) -> Result<(), String> {
        let old = self.store.device_mut(&d.id).ok_or("No such device")?;
        old.name = if d.name.trim().is_empty() { "Unnamed device".into() } else { d.name.trim().to_string() };
        old.hue = d.hue;
        old.announce = d.announce;
        if !d.is_new && old.is_new {
            old.is_new = false;
            if self.tracker.current() == Some(&d.id) {
                self.conn = ConnState::Recognized { device_id: d.id.clone() };
            }
        }
        self.changed();
        Ok(())
    }

    pub fn update_profile(&mut self, mut p: Profile) -> Result<(), String> {
        let old = self.store.profile(&p.id).ok_or("No such profile")?;
        p.name = p.name.trim().to_string();
        if p.name.is_empty() {
            p.name = old.name.clone();
        }
        p.gains = p.gains.map(|g| crate::eq::snap(g, -12., 12.));
        p.balance = crate::eq::snap(p.balance, -6., 6.);
        self.store.upsert_profile(p);
        self.apply();
        self.changed();
        Ok(())
    }

    pub fn forget(&mut self, id: &str) -> Result<(), String> {
        self.store.forget(id, self.tracker.current()).map_err(|e| match e {
            StoreError::Connected => "Disconnect it first".to_string(),
            StoreError::NotFound => "No such device".to_string(),
        })?;
        self.hardware.remove(id);
        self.changed();
        Ok(())
    }

    pub fn search_autoeq(&self, q: &str, limit: usize) -> Vec<Hit> {
        self.autoeq.search(q, limit)
    }

    pub fn apply_autoeq(&mut self, device_id: &str, index: usize) -> Result<(), String> {
        let e = self.autoeq.0.get(index).ok_or("No such AutoEq entry")?.clone();
        let p = self.profile_mut_for(device_id)?;
        p.gains = fit_10band(&e.filters);
        p.preamp = None;
        p.name = e.model.clone();
        p.source = Source::Autoeq;
        p.autoeq_model = Some(e.model);
        self.apply();
        self.changed();
        Ok(())
    }

    pub fn set_settings(&mut self, s: Settings) {
        let voice_ok = VOICES.iter().any(|v| v.0 == s.voice);
        let keep = self.store.settings.voice.clone();
        self.store.settings = s;
        if !voice_ok {
            self.store.settings.voice = keep;
        }
        if self.store.settings.template.trim().is_empty() {
            self.store.settings.template = crate::store::DEFAULT_TEMPLATE.into();
        }
        self.changed();
    }

    pub fn choose_device(&mut self, id: Option<&str>) -> Option<(ConnState, bool)> {
        let t = {
            let Core { store, tracker, .. } = self;
            tracker.choose(store, id, now_ms())?
        };
        self.conn = t.state.clone();
        self.apply();
        self.changed();
        Some((t.state, t.announce))
    }

    pub fn not_this_set(&mut self) -> Option<(ConnState, bool)> {
        let o = self.last_output.clone().filter(|o| o.kind == Kind::Wired)?;
        let t = {
            let Core { store, tracker, .. } = self;
            tracker.not_this_set(store, o, now_ms())?
        };
        self.conn = t.state.clone();
        self.apply();
        self.changed();
        Some((t.state, t.announce))
    }

    pub fn hearing_start(&mut self) -> HtStep {
        let (s, step) = Staircase::new();
        self.hearing = Some((s, step.clone()));
        self.bypass = true;
        self.apply();
        self.changed();
        step
    }

    pub fn hearing_answer(&mut self, heard: bool) -> Option<HtStep> {
        let (s, last) = self.hearing.as_mut()?;
        let step = s.answer(heard);
        *last = step.clone();
        if matches!(step, HtStep::Done { .. }) {
            self.bypass = false;
            self.apply();
        }
        self.changed();
        Some(step)
    }

    pub fn hearing_stop(&mut self) {
        if !matches!(self.hearing.as_ref().map(|h| &h.1), Some(HtStep::Done { .. })) {
            self.hearing = None;
        }
        self.bypass = false;
        self.apply();
        self.changed();
    }

    pub fn hearing_apply(&mut self, device_id: &str) -> Result<(), String> {
        let Some((_, HtStep::Done { left, right })) = self.hearing.take() else { return Err("Finish the test first".into()) };
        let (gains, balance) = apply_hearing(&left, &right);
        let p = self.profile_mut_for(device_id)?;
        p.gains = gains;
        p.balance = balance;
        p.preamp = None;
        p.name = "Hearing fit".into();
        p.source = Source::Hearing;
        p.hearing = Some(Hearing { left, right });
        self.apply();
        self.changed();
        Ok(())
    }

    /// `a` and `b` are profile ids. Each round maps X and Y to them at random.
    pub fn ab_start(&mut self, device_id: &str, a: &str, b: &str) -> Result<(), String> {
        if a == b || self.store.profile(a).is_none() || self.store.profile(b).is_none() {
            return Err("Pick two different profiles".into());
        }
        let order = std::array::from_fn(|_| fastrand::bool());
        self.ab = Some((device_id.to_string(), Ab::new(a.into(), b.into(), order)));
        self.ab_listen(Side::X)
    }

    pub fn ab_listen(&mut self, side: Side) -> Result<(), String> {
        let (_, ab) = self.ab.as_ref().ok_or("No A/B running")?;
        let p = self.store.profile(ab.profile_for(side)).cloned();
        self.override_profile = p;
        self.apply();
        self.changed();
        Ok(())
    }

    pub fn ab_pick(&mut self, side: Side) -> Option<AbResult> {
        let (_, ab) = self.ab.as_mut()?;
        let r = ab.pick(side);
        if r.is_some() {
            self.ab = None;
            self.override_profile = None;
            self.apply();
        } else {
            let _ = self.ab_listen(Side::X);
        }
        self.changed();
        r
    }

    pub fn ab_cancel(&mut self) {
        self.ab = None;
        self.override_profile = None;
        self.apply();
        self.changed();
    }

    /// Copies the sound of profile `from` onto the device's own profile.
    pub fn use_profile(&mut self, device_id: &str, from: &str) -> Result<(), String> {
        let src = self.store.profile(from).cloned().ok_or("No such profile")?;
        let p = self.profile_mut_for(device_id)?;
        let id = p.id.clone();
        *p = Profile { id, source: Source::Ab, ..src };
        self.apply();
        self.changed();
        Ok(())
    }

    pub fn set_bypass(&mut self, on: bool) {
        self.bypass = on;
        self.apply();
        self.changed();
    }

    pub fn export_store(&self) -> String {
        serde_json::to_string_pretty(&self.store).unwrap()
    }

    pub fn import_store(&mut self, json: &str) -> Result<(), String> {
        let s: Store = serde_json::from_str(json).map_err(|_| "That file is not a SoundBettrCore export.".to_string())?;
        self.store = s;
        self.apply();
        self.changed();
        Ok(())
    }

    pub fn set_autoeq(&mut self, index: Index, when: String) {
        self.autoeq = index;
        self.store.settings.autoeq_updated = Some(when);
        self.changed();
    }
}

/// Sends a command to the earbuds and refreshes their state.
pub fn hw_command(shared: &Shared, device_id: &str, cmd: HwCommand) -> CommandResult {
    let packet = match &cmd {
        HwCommand::SoundModes { modes } => a3959::build_sound_modes_packet(modes),
        HwCommand::Gaming { on } => a3959::build_gaming_mode_packet(*on),
        HwCommand::Eq { gains_tenths_db } => a3959::build_equalizer_with_drc_packet(a3959::CUSTOM_EQ_PRESET_ID, gains_tenths_db),
        HwCommand::Button { side, press, action } => a3959::build_button_action_packet(*side, *press, *action),
        HwCommand::Refresh => a3959::build_state_request_packet(),
    };
    // Take the session out so the slow Bluetooth round trip never holds the lock
    let Some((mut hw, old)) = shared.lock().unwrap().hardware.remove(device_id) else {
        return CommandResult::Failed { reason: "The earbuds are not connected.".into() };
    };
    let result = rfcomm::send(hw.as_mut(), &packet);
    let state = hw.exchange(&a3959::build_state_request_packet()).ok().flatten().and_then(|r| a3959::parse_a3959_state_packet(&r).ok());
    let mut c = shared.lock().unwrap();
    c.hardware.insert(device_id.to_string(), (hw, state.or(old)));
    emit_snapshot(&c);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::{Pcm, Synth};

    #[derive(Default)]
    struct Fake {
        applied: Mutex<Vec<(Option<String>, Option<[f32; 10]>, bool)>>,
        played: Mutex<usize>,
    }
    impl Platform for Fake {
        fn name(&self) -> &'static str {
            "test"
        }
        fn apply(&self, _: &Store, d: Option<&Device>, p: Option<&Profile>, bypass: bool) -> Result<(), String> {
            self.applied.lock().unwrap().push((d.map(|d| d.id.clone()), p.map(|p| p.gains), bypass));
            Ok(())
        }
        fn play(&self, _: &Path) {
            *self.played.lock().unwrap() += 1;
        }
        fn hardware(&self, _: &Device) -> Option<Box<dyn Hardware>> {
            None
        }
        fn emit(&self, _: &str, _: serde_json::Value) {}
    }

    struct Silent;
    impl Synth for Silent {
        fn render(&self, _: &str, _: i32) -> Result<Pcm, String> {
            Ok(Pcm { samples: vec![0.; 240], rate: 24000 })
        }
    }

    struct Broken;
    impl Synth for Broken {
        fn render(&self, _: &str, _: i32) -> Result<Pcm, String> {
            Err("model missing".into())
        }
    }

    fn setup(synth: Box<dyn Synth>) -> (Shared, Arc<Fake>, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let fake = Arc::new(Fake::default());
        let a = Announcer { synth, cache_dir: dir.path().join("voice") };
        let core = Core::new(dir.path().into(), Index::default(), Ok(a), fake.clone());
        (Arc::new(Mutex::new(core)), fake, dir)
    }

    fn out(addr: &str) -> Output {
        Output { kind: Kind::Bluetooth, key: addr.into(), name: "Wired-free buds".into(), address: Some(addr.into()) }
    }

    fn current(s: &Shared) -> String {
        s.lock().unwrap().snapshot().current.unwrap()
    }

    #[test]
    fn new_output_applies_flat_and_does_not_announce() {
        let (s, fake, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        std::thread::sleep(Duration::from_millis(1800));
        let (dev, gains, bypass) = fake.applied.lock().unwrap().last().cloned().unwrap();
        assert_eq!(dev.as_deref(), Some("bt:AA"));
        assert_eq!(gains, Some([0.; 10]));
        assert!(!bypass);
        assert_eq!(*fake.played.lock().unwrap(), 0);
        assert!(matches!(s.lock().unwrap().snapshot().conn, ConnState::New { .. }));
    }

    #[test]
    fn known_output_applies_profile_then_announces_after_delay() {
        let (s, fake, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        {
            let mut c = s.lock().unwrap();
            let mut d = c.store.device("bt:AA").unwrap().clone();
            d.is_new = false;
            c.update_device(d).unwrap();
        }
        on_output(&s, None);
        std::thread::sleep(Duration::from_millis(1600));
        on_output(&s, Some(out("aa")));
        std::thread::sleep(Duration::from_millis(500));
        assert_eq!(*fake.played.lock().unwrap(), 0, "too early");
        std::thread::sleep(Duration::from_millis(1500));
        assert_eq!(*fake.played.lock().unwrap(), 1);
    }

    #[test]
    fn update_profile_on_current_device_reapplies() {
        let (s, fake, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        let mut c = s.lock().unwrap();
        let mut p = c.store.profile_for("bt:AA").unwrap().clone();
        p.gains[5] = 3.26;
        c.update_profile(p).unwrap();
        assert_eq!(fake.applied.lock().unwrap().last().unwrap().1.unwrap()[5], 3.5);
    }

    #[test]
    fn hearing_start_sets_bypass_and_stop_clears_it() {
        let (s, fake, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        let mut c = s.lock().unwrap();
        c.hearing_start();
        assert!(fake.applied.lock().unwrap().last().unwrap().2);
        c.hearing_stop();
        assert!(!fake.applied.lock().unwrap().last().unwrap().2);
    }

    #[test]
    fn ab_listen_applies_the_mapped_profile() {
        let (s, fake, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        let mut c = s.lock().unwrap();
        let a = c.store.profile_for("bt:AA").unwrap().id.clone();
        let b = c.store.new_flat_profile();
        c.store.profiles.iter_mut().find(|p| p.id == b).unwrap().gains[0] = 6.;
        c.ab_start("bt:AA", &a, &b).unwrap();
        let x = fake.applied.lock().unwrap().last().unwrap().1.unwrap();
        c.ab_listen(Side::Y).unwrap();
        let y = fake.applied.lock().unwrap().last().unwrap().1.unwrap();
        assert_ne!(x, y);
        assert!([x[0], y[0]].contains(&6.) && [x[0], y[0]].contains(&0.));
    }

    #[test]
    fn forget_current_is_refused() {
        let (s, _f, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        let id = current(&s);
        assert_eq!(s.lock().unwrap().forget(&id), Err("Disconnect it first".into()));
    }

    #[test]
    fn voice_failure_still_shows_state() {
        let (s, fake, _d) = setup(Box::new(Broken));
        on_output(&s, Some(out("aa")));
        announce_now(&s, "bt:AA", false);
        let snap = s.lock().unwrap().snapshot();
        assert!(matches!(snap.conn, ConnState::New { .. }));
        assert!(!snap.speaking);
        assert_eq!(snap.voice_error.as_deref(), Some("model missing"));
        assert_eq!(*fake.played.lock().unwrap(), 0);
    }

    #[test]
    fn import_rejects_invalid_json_and_keeps_store() {
        let (s, _f, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        let mut c = s.lock().unwrap();
        assert!(c.import_store("{nope").is_err());
        assert_eq!(c.store.devices.len(), 1);
    }

    #[test]
    fn blank_name_saves_as_unnamed_device() {
        let (s, _f, _d) = setup(Box::new(Silent));
        on_output(&s, Some(out("aa")));
        let mut c = s.lock().unwrap();
        let mut d = c.store.device("bt:AA").unwrap().clone();
        d.name = "   ".into();
        c.update_device(d).unwrap();
        assert_eq!(c.store.device("bt:AA").unwrap().name, "Unnamed device");
    }
}
