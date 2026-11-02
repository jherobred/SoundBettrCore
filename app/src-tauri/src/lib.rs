pub mod a3959;
pub mod apo;
pub mod autoeq;
pub mod core;
pub mod device;
pub mod eq;
pub mod hearing;
pub mod rfcomm;
pub mod store;
pub mod voice;
pub mod win_audio;
#[cfg(target_os = "android")]
mod android;

use crate::autoeq::{Hit, Index};
use crate::core::{Core, HwCommand, Shared, Snapshot};
use crate::hearing::{AbResult, HtStep, Side};
use crate::rfcomm::CommandResult;
use crate::store::{Device, Profile, Settings};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

type R<T> = Result<T, String>;

fn snap(s: &State<Shared>) -> Snapshot {
    s.lock().unwrap().snapshot()
}

#[tauri::command]
fn get_snapshot(s: State<Shared>) -> Snapshot {
    snap(&s)
}

#[tauri::command]
fn update_device(s: State<Shared>, device: Device) -> R<Snapshot> {
    s.lock().unwrap().update_device(device)?;
    Ok(snap(&s))
}

#[tauri::command]
fn forget_device(s: State<Shared>, id: String) -> R<Snapshot> {
    s.lock().unwrap().forget(&id)?;
    Ok(snap(&s))
}

#[tauri::command]
fn update_profile(s: State<Shared>, profile: Profile) -> R<Snapshot> {
    s.lock().unwrap().update_profile(profile)?;
    Ok(snap(&s))
}

#[tauri::command]
fn use_profile(s: State<Shared>, device_id: String, from: String) -> R<Snapshot> {
    s.lock().unwrap().use_profile(&device_id, &from)?;
    Ok(snap(&s))
}

#[tauri::command]
fn search_autoeq(s: State<Shared>, query: String, limit: usize) -> Vec<Hit> {
    s.lock().unwrap().search_autoeq(&query, limit.min(50))
}

#[tauri::command]
fn apply_autoeq(s: State<Shared>, device_id: String, index: usize) -> R<Snapshot> {
    s.lock().unwrap().apply_autoeq(&device_id, index)?;
    Ok(snap(&s))
}

#[tauri::command]
async fn announce(s: State<'_, Shared>, device_id: String) -> R<()> {
    let shared = s.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core::announce_now(&shared, &device_id, false)).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn preview_voice(s: State<'_, Shared>, voice: String, text: String) -> R<()> {
    let shared = s.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core::speak_text(&shared, &text, &voice)).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn set_settings(s: State<Shared>, settings: Settings) -> Snapshot {
    s.lock().unwrap().set_settings(settings);
    snap(&s)
}

#[tauri::command]
fn choose_device(s: State<Shared>, id: Option<String>) -> Snapshot {
    let r = s.lock().unwrap().choose_device(id.as_deref());
    after_choice(&s, r);
    snap(&s)
}

#[tauri::command]
fn not_this_set(s: State<Shared>) -> Snapshot {
    let r = s.lock().unwrap().not_this_set();
    after_choice(&s, r);
    snap(&s)
}

fn after_choice(s: &State<Shared>, r: Option<(device::ConnState, bool)>) {
    if let Some((device::ConnState::Recognized { device_id }, true)) = r {
        let shared = s.inner().clone();
        std::thread::spawn(move || core::announce_now(&shared, &device_id, true));
    }
}

#[tauri::command]
fn hearing_start(s: State<Shared>) -> HtStep {
    s.lock().unwrap().hearing_start()
}

#[tauri::command]
fn hearing_answer(s: State<Shared>, heard: bool) -> Option<HtStep> {
    s.lock().unwrap().hearing_answer(heard)
}

#[tauri::command]
fn hearing_stop(s: State<Shared>) -> Snapshot {
    s.lock().unwrap().hearing_stop();
    snap(&s)
}

#[tauri::command]
fn hearing_apply(s: State<Shared>, device_id: String) -> R<Snapshot> {
    s.lock().unwrap().hearing_apply(&device_id)?;
    Ok(snap(&s))
}

#[tauri::command]
fn ab_start(s: State<Shared>, device_id: String, a: String, b: String) -> R<Snapshot> {
    s.lock().unwrap().ab_start(&device_id, &a, &b)?;
    Ok(snap(&s))
}

#[tauri::command]
fn ab_listen(s: State<Shared>, side: Side) -> R<Snapshot> {
    s.lock().unwrap().ab_listen(side)?;
    Ok(snap(&s))
}

#[tauri::command]
fn ab_pick(s: State<Shared>, side: Side) -> Option<AbResult> {
    s.lock().unwrap().ab_pick(side)
}

#[tauri::command]
fn ab_cancel(s: State<Shared>) -> Snapshot {
    s.lock().unwrap().ab_cancel();
    snap(&s)
}

#[tauri::command]
async fn hw_command(s: State<'_, Shared>, device_id: String, command: HwCommand) -> R<CommandResult> {
    let shared = s.inner().clone();
    tauri::async_runtime::spawn_blocking(move || core::hw_command(&shared, &device_id, command)).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn export_store(s: State<Shared>, path: String) -> R<()> {
    let text = s.lock().unwrap().export_store();
    std::fs::write(path, text).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_store(s: State<Shared>, path: String) -> R<Snapshot> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    s.lock().unwrap().import_store(&text)?;
    Ok(snap(&s))
}

/// The app's only network call, and it runs only when the user presses Update.
#[tauri::command]
async fn update_autoeq_db(app: AppHandle, s: State<'_, Shared>) -> R<Snapshot> {
    const URL: &str = "https://github.com/jherobred/SoundBettrCore/releases/latest/download/autoeq-index.json.gz";
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let path = dir.join("autoeq-index.json.gz");
    let p2 = path.clone();
    tauri::async_runtime::spawn_blocking(move || -> R<()> {
        let mut body = ureq::get(URL).call().map_err(|e| format!("Download failed: {e}"))?.into_body();
        let bytes = body.with_config().limit(64 << 20).read_to_vec().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(p2.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&p2, bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    let index = Index::load_gz(&path).map_err(|_| "The downloaded database is damaged. The bundled one stays.".to_string())?;
    let today = chrono_free_date();
    s.lock().unwrap().set_autoeq(index, today);
    Ok(snap(&s))
}

fn chrono_free_date() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() / 86_400) as i64;
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[tauri::command]
fn connect_apo() -> R<()> {
    #[cfg(windows)]
    return apo::hook();
    #[cfg(not(windows))]
    Err("Only on Windows".into())
}

#[cfg(windows)]
mod windows_platform {
    use super::*;
    use crate::apo::{self, ApoTarget};
    use crate::core::Platform;
    use crate::rfcomm::{parse_mac, Hardware, RfcommSession};
    use crate::store::{Kind, Store};
    use std::path::{Path, PathBuf};

    pub struct WinPlatform {
        pub app: AppHandle,
        pub hrir: PathBuf,
    }

    impl Platform for WinPlatform {
        fn name(&self) -> &'static str {
            "windows"
        }

        fn apply(&self, store: &Store, current: Option<&Device>, profile: Option<&Profile>, bypass: bool) -> Result<(), String> {
            let targets: Vec<ApoTarget> = store
                .devices
                .iter()
                .filter(|d| !d.key.is_empty())
                .filter_map(|d| {
                    let is_current = current.is_some_and(|c| c.id == d.id);
                    let p = if is_current { profile } else { store.profile(&d.profile_id) }?;
                    Some(ApoTarget { endpoint_id: &d.key, profile: p, bypass: is_current && bypass })
                })
                .collect();
            apo::write_live(&apo::render(&targets, &self.hrir)).map_err(|e| e.to_string())?;
            let st = apo::status();
            if !st.installed {
                Err("System sound needs EqualizerAPO. Install it to tune what every app plays.".into())
            } else if !st.hooked {
                Err("EqualizerAPO is installed but not connected to SoundBettrCore yet.".into())
            } else {
                Ok(())
            }
        }

        fn play(&self, wav: &Path) {
            crate::win_audio::play_wav(wav);
        }

        fn hardware(&self, d: &Device) -> Option<Box<dyn Hardware>> {
            if d.kind != Kind::Bluetooth {
                return None;
            }
            let addr = parse_mac(d.id.strip_prefix("bt:")?)?;
            Some(Box::new(RfcommSession::new(addr)))
        }

        fn emit(&self, event: &str, payload: serde_json::Value) {
            let _ = self.app.emit(event, payload);
        }

        fn apo(&self) -> Option<apo::ApoStatus> {
            Some(apo::status())
        }
    }
}

#[cfg(windows)]
fn load_core(app: &AppHandle, platform: Arc<dyn core::Platform>) -> Shared {
    let data = app.path().app_data_dir().expect("app data dir");
    let res = app.path().resource_dir().expect("resource dir");
    let updated = data.join("autoeq-index.json.gz");
    let autoeq = Index::load_gz(&updated).or_else(|_| Index::load_gz(&res.join("resources/autoeq/index.json.gz"))).unwrap_or_default();
    let shared: Shared = Arc::new(Mutex::new(Core::new(data.clone(), autoeq, Err("Preparing the voice".into()), platform)));
    // Loading Kokoro takes a few seconds; the window shows meanwhile
    let s2 = shared.clone();
    std::thread::spawn(move || {
        let a = voice::KokoroSynth::load(&res.join("resources/kokoro")).map(|k| voice::Announcer { synth: Box::new(k), cache_dir: data.join("voice-cache") });
        s2.lock().unwrap().set_announcer(a);
    });
    shared
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| show_main(app)));
    }
    builder
        .setup(|app| {
            let handle = app.handle().clone();
            #[cfg(windows)]
            {
                let res = app.path().resource_dir()?;
                let platform = Arc::new(windows_platform::WinPlatform { app: handle.clone(), hrir: res.join("resources/hrir") });
                let shared = load_core(&handle, platform);
                app.manage(shared.clone());
                let s2 = shared.clone();
                win_audio::watch(move |o| core::on_output(&s2, o));
                tray::create(&handle)?;
            }
            #[cfg(target_os = "android")]
            {
                // MainActivity calls NativeBridge.init before Tauri starts, so the core already exists
                let shared = android::shared().ok_or("NativeBridge.init was not called")?;
                android::attach(handle.clone());
                app.manage(shared);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            update_device,
            forget_device,
            update_profile,
            use_profile,
            search_autoeq,
            apply_autoeq,
            announce,
            preview_voice,
            set_settings,
            choose_device,
            not_this_set,
            hearing_start,
            hearing_answer,
            hearing_stop,
            hearing_apply,
            ab_start,
            ab_listen,
            ab_pick,
            ab_cancel,
            hw_command,
            export_store,
            import_store,
            update_autoeq_db,
            connect_apo,
        ])
        .on_window_event(|w, e| {
            // Closing the window keeps the app in the tray so it can announce connections
            #[cfg(desktop)]
            if let tauri::WindowEvent::CloseRequested { api, .. } = e {
                api.prevent_close();
                let _ = w.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running SoundBettrCore");
}

#[cfg(desktop)]
fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg(desktop)]
mod tray {
    use super::*;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    pub fn create(app: &AppHandle) -> tauri::Result<()> {
        let open = MenuItem::with_id(app, "open", "Open SoundBettrCore", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
        let menu = Menu::with_items(app, &[&open, &quit])?;
        TrayIconBuilder::with_id("main")
            .icon(app.default_window_icon().unwrap().clone())
            .tooltip("SoundBettrCore")
            .menu(&menu)
            .show_menu_on_left_click(false)
            .on_menu_event(|app, e| match e.id.as_ref() {
                "open" => show_main(app),
                "quit" => app.exit(0),
                _ => {}
            })
            .on_tray_icon_event(|t, e| {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                    show_main(t.app_handle());
                }
            })
            .build(app)?;
        Ok(())
    }
}
