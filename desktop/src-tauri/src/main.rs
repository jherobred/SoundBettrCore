#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod a3959;
pub mod apo;
pub mod bluetooth;
pub mod dsp;
pub mod presets;

use a3959::{
    A3959SoundModes, A3959State, BASS_UP_PRESET_ID, ButtonAction, ButtonPressKind,
    CUSTOM_EQ_PRESET_ID, EarbudSide, build_button_action_packet, build_equalizer_with_drc_packet,
    build_gaming_mode_packet, build_sound_modes_packet, build_surround_sound_packet,
    build_touch_tone_packet, compute_a3959_drc_bytes,
};
use apo::{ApoSyncStatus, parse_equalizer_apo_txt, render_equalizer_apo_txt, sync_profile_to_system_apo};
use bluetooth::{
    DiscoveredBtDevice, PacketDispatchResult, exchange_rfcomm_packet, format_hex,
    scan_bluetooth_devices, try_query_live_r50i_state,
};
use dsp::{DspPipeline, DspProfile, TestSignalKind, synthesize_test_wav};
use presets::builtin_presets;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

pub struct StudioStore {
    pub active_profile: Mutex<DspProfile>,
    pub r50i_state: Mutex<A3959State>,
    pub apo_status: Mutex<ApoSyncStatus>,
    pub last_packet_hex: Mutex<String>,
    pub active_bt_addr: Mutex<Option<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioSnapshot {
    pub profile: DspProfile,
    pub presets: Vec<DspProfile>,
    pub r50i: A3959State,
    pub apo: ApoSyncStatus,
    pub response_curve: Vec<(f64, f64)>,
    pub last_packet_hex: String,
    pub bt_devices: Vec<DiscoveredBtDevice>,
}

fn build_snapshot(store: &StudioStore) -> StudioSnapshot {
    let profile = store.active_profile.lock().unwrap().clone();
    let r50i = store.r50i_state.lock().unwrap().clone();
    let apo = store.apo_status.lock().unwrap().clone();
    let last_packet_hex = store.last_packet_hex.lock().unwrap().clone();
    let pipeline = DspPipeline::new(48000.0, profile.clone());
    let response_curve = pipeline.compute_response_curve(96);
    let bt_devices = scan_bluetooth_devices();

    StudioSnapshot {
        profile,
        presets: builtin_presets(),
        r50i,
        apo,
        response_curve,
        last_packet_hex,
        bt_devices,
    }
}

fn dispatch_r50i_packet(store: &StudioStore, packet: &[u8]) -> PacketDispatchResult {
    let hex = format_hex(packet);
    *store.last_packet_hex.lock().unwrap() = hex.clone();

    let maybe_addr = *store.active_bt_addr.lock().unwrap();
    let mut transmitted = false;
    if let Some(addr) = maybe_addr {
        if exchange_rfcomm_packet(addr, packet, false).is_ok() {
            transmitted = true;
        }
    }
    let state = store.r50i_state.lock().unwrap().clone();
    PacketDispatchResult {
        transmitted_over_rfcomm: transmitted,
        packet_hex: hex,
        state,
    }
}

#[tauri::command]
fn get_studio_snapshot(store: State<StudioStore>) -> StudioSnapshot {
    build_snapshot(&store)
}

#[tauri::command]
fn apply_dsp_profile(store: State<StudioStore>, profile: DspProfile) -> StudioSnapshot {
    let apo = sync_profile_to_system_apo(&profile);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        r50i.eq_gains_tenths_db = profile.hw_eq_tenths_db;
        let drc = compute_a3959_drc_bytes(&profile.hw_eq_tenths_db);
        r50i.drc_preview_tenths_db = std::array::from_fn(|i| (drc[i] as i16) - 120);
    }
    *store.active_profile.lock().unwrap() = profile;
    *store.apo_status.lock().unwrap() = apo;
    build_snapshot(&store)
}

#[tauri::command]
fn select_preset(store: State<StudioStore>, preset_id: String) -> Result<StudioSnapshot, String> {
    let all = builtin_presets();
    let found = all
        .into_iter()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| format!("Unknown preset: {preset_id}"))?;
    let apo = sync_profile_to_system_apo(&found);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        r50i.eq_gains_tenths_db = found.hw_eq_tenths_db;
        let drc = compute_a3959_drc_bytes(&found.hw_eq_tenths_db);
        r50i.drc_preview_tenths_db = std::array::from_fn(|i| (drc[i] as i16) - 120);
    }
    let pkt = build_equalizer_with_drc_packet(CUSTOM_EQ_PRESET_ID, &found.hw_eq_tenths_db);
    let _ = dispatch_r50i_packet(&store, &pkt);
    *store.active_profile.lock().unwrap() = found;
    *store.apo_status.lock().unwrap() = apo;
    Ok(build_snapshot(&store))
}

#[tauri::command]
fn toggle_bypass(store: State<StudioStore>) -> StudioSnapshot {
    let mut profile = store.active_profile.lock().unwrap().clone();
    profile.bypass = !profile.bypass;
    let apo = sync_profile_to_system_apo(&profile);
    *store.active_profile.lock().unwrap() = profile;
    *store.apo_status.lock().unwrap() = apo;
    build_snapshot(&store)
}

#[tauri::command]
fn scan_and_connect_r50i(store: State<StudioStore>, bt_addr: Option<u64>) -> StudioSnapshot {
    let current = store.r50i_state.lock().unwrap().clone();
    let devices = scan_bluetooth_devices();

    if let Some(addr) = bt_addr.or_else(|| devices.iter().find(|d| d.is_r50i_nc).map(|d| d.bt_addr))
    {
        *store.active_bt_addr.lock().unwrap() = Some(addr);
    }

    if let Some(updated) = try_query_live_r50i_state(&current) {
        *store.r50i_state.lock().unwrap() = updated;
    }
    build_snapshot(&store)
}

#[tauri::command]
fn set_r50i_sound_modes(
    store: State<StudioStore>,
    modes: A3959SoundModes,
) -> StudioSnapshot {
    let pkt = build_sound_modes_packet(&modes);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        r50i.sound_modes = modes;
    }
    let _ = dispatch_r50i_packet(&store, &pkt);
    build_snapshot(&store)
}

#[tauri::command]
fn flash_r50i_hardware_eq(
    store: State<StudioStore>,
    gains_tenths_db: [i16; 8],
    bass_up: bool,
) -> StudioSnapshot {
    let preset_id = if bass_up {
        BASS_UP_PRESET_ID
    } else {
        CUSTOM_EQ_PRESET_ID
    };
    let pkt = build_equalizer_with_drc_packet(preset_id, &gains_tenths_db);
    let drc = compute_a3959_drc_bytes(&gains_tenths_db);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        r50i.eq_preset_id = preset_id;
        r50i.bass_up_enabled = bass_up;
        r50i.eq_gains_tenths_db = gains_tenths_db;
        r50i.drc_preview_tenths_db = std::array::from_fn(|i| (drc[i] as i16) - 120);
    }
    {
        let mut prof = store.active_profile.lock().unwrap();
        prof.hw_eq_tenths_db = gains_tenths_db;
    }
    let _ = dispatch_r50i_packet(&store, &pkt);
    build_snapshot(&store)
}

#[tauri::command]
fn set_r50i_surround_mode(store: State<StudioStore>, enabled: bool) -> StudioSnapshot {
    let pkt = build_surround_sound_packet(enabled);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        r50i.onboard_surround = enabled;
    }
    let _ = dispatch_r50i_packet(&store, &pkt);
    build_snapshot(&store)
}

#[tauri::command]
fn set_r50i_gaming_mode(store: State<StudioStore>, enabled: bool) -> StudioSnapshot {
    let pkt = build_gaming_mode_packet(enabled);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        r50i.gaming_mode = Some(enabled);
    }
    let _ = dispatch_r50i_packet(&store, &pkt);
    build_snapshot(&store)
}

#[tauri::command]
fn set_r50i_touch_tone(store: State<StudioStore>, enabled: bool) -> StudioSnapshot {
    let pkt = build_touch_tone_packet(enabled);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        r50i.touch_tone = enabled;
    }
    let _ = dispatch_r50i_packet(&store, &pkt);
    build_snapshot(&store)
}

#[tauri::command]
fn set_r50i_button_action(
    store: State<StudioStore>,
    side: EarbudSide,
    press: ButtonPressKind,
    action: ButtonAction,
) -> StudioSnapshot {
    let pkt = build_button_action_packet(side, press, action);
    {
        let mut r50i = store.r50i_state.lock().unwrap();
        match (side, press) {
            (EarbudSide::Left, ButtonPressKind::Single) => r50i.buttons.left_single = action,
            (EarbudSide::Right, ButtonPressKind::Single) => r50i.buttons.right_single = action,
            (EarbudSide::Left, ButtonPressKind::Double) => r50i.buttons.left_double = action,
            (EarbudSide::Right, ButtonPressKind::Double) => r50i.buttons.right_double = action,
            (EarbudSide::Left, ButtonPressKind::Triple) => r50i.buttons.left_triple = action,
            (EarbudSide::Right, ButtonPressKind::Triple) => r50i.buttons.right_triple = action,
            (EarbudSide::Left, ButtonPressKind::Long) => r50i.buttons.left_long = action,
            (EarbudSide::Right, ButtonPressKind::Long) => r50i.buttons.right_long = action,
        }
    }
    let _ = dispatch_r50i_packet(&store, &pkt);
    build_snapshot(&store)
}

#[tauri::command]
fn render_test_signal_wav(
    store: State<StudioStore>,
    kind: TestSignalKind,
    duration_secs: f32,
) -> Vec<u8> {
    let profile = store.active_profile.lock().unwrap().clone();
    synthesize_test_wav(kind, duration_secs, &profile)
}

#[tauri::command]
fn export_equalizer_apo_text(store: State<StudioStore>) -> String {
    let profile = store.active_profile.lock().unwrap().clone();
    render_equalizer_apo_txt(&profile)
}

#[tauri::command]
fn import_equalizer_apo_text(
    store: State<StudioStore>,
    content: String,
) -> Result<StudioSnapshot, String> {
    let current = store.active_profile.lock().unwrap().clone();
    let next = parse_equalizer_apo_txt(&content, &current)?;
    let apo = sync_profile_to_system_apo(&next);
    *store.active_profile.lock().unwrap() = next;
    *store.apo_status.lock().unwrap() = apo;
    Ok(build_snapshot(&store))
}

#[tauri::command]
fn export_preset_json(store: State<StudioStore>) -> Result<String, String> {
    let profile = store.active_profile.lock().unwrap().clone();
    serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_preset_json(
    store: State<StudioStore>,
    json_str: String,
) -> Result<StudioSnapshot, String> {
    let profile: DspProfile = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;
    let apo = sync_profile_to_system_apo(&profile);
    *store.active_profile.lock().unwrap() = profile;
    *store.apo_status.lock().unwrap() = apo;
    Ok(build_snapshot(&store))
}

fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Open SoundBettrCore Rack", true, None::<&str>)?;
    let bypass = MenuItem::with_id(app, "bypass", "Toggle Instant A/B Bypass", true, None::<&str>)?;
    let p_harman = MenuItem::with_id(
        app,
        "preset:r50i_harman_reference",
        "Preset: R50i NC Harman Reference",
        true,
        None::<&str>,
    )?;
    let p_cinema = MenuItem::with_id(
        app,
        "preset:dolby_cinema_3d",
        "Preset: Dolby Cinema 3D",
        true,
        None::<&str>,
    )?;
    let p_bass = MenuItem::with_id(
        app,
        "preset:bassup_ultra_clean",
        "Preset: BassUp Ultra Clean",
        true,
        None::<&str>,
    )?;
    let p_fps = MenuItem::with_id(
        app,
        "preset:fps_gaming_footsteps",
        "Preset: FPS Gaming Footsteps",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit SoundBettrCore", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &show,
            &bypass,
            &PredefinedMenuItem::separator(app)?,
            &p_harman,
            &p_cinema,
            &p_bass,
            &p_fps,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(tauri::image::Image::from_bytes(include_bytes!(
            "../icons/icon.png"
        ))?)
        .tooltip("SoundBettrCore · R50i NC Studio Rack")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            if id == "show" {
                show_main_window(app);
            } else if id == "bypass" {
                let store = app.state::<StudioStore>();
                let snap = toggle_bypass(store);
                let _ = app.emit("studio://updated", snap);
            } else if let Some(preset_id) = id.strip_prefix("preset:") {
                let store = app.state::<StudioStore>();
                if let Ok(snap) = select_preset(store, preset_id.to_string()) {
                    let _ = app.emit("studio://updated", snap);
                }
            } else if id == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--quit") {
        std::process::exit(0);
    }

    let initial_profile = DspProfile::default();
    let initial_apo = sync_profile_to_system_apo(&initial_profile);
    let mut initial_r50i = A3959State::default();
    if let Some(live) = try_query_live_r50i_state(&initial_r50i) {
        initial_r50i = live;
    }
    let init_pkt = build_equalizer_with_drc_packet(
        CUSTOM_EQ_PRESET_ID,
        &initial_profile.hw_eq_tenths_db,
    );

    let store = StudioStore {
        active_profile: Mutex::new(initial_profile),
        r50i_state: Mutex::new(initial_r50i),
        apo_status: Mutex::new(initial_apo),
        last_packet_hex: Mutex::new(format_hex(&init_pkt)),
        active_bt_addr: Mutex::new(None),
    };

    tauri::Builder::default()
        .manage(store)
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if args.iter().any(|a| a == "--quit") {
                app.exit(0);
            } else {
                show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            create_tray(&handle)?;

            let start_hidden = std::env::args().any(|a| a == "--background");
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("SoundBettrCore — Soundcore R50i NC (A3959) Spatial & Harman DSP Rack")
                .inner_size(1220.0, 820.0)
                .min_inner_size(960.0, 680.0)
                .resizable(true)
                .visible(!start_hidden)
                .build()?;

            let win_handle = handle.clone();
            win.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Some(w) = win_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_studio_snapshot,
            apply_dsp_profile,
            select_preset,
            toggle_bypass,
            scan_and_connect_r50i,
            set_r50i_sound_modes,
            flash_r50i_hardware_eq,
            set_r50i_surround_mode,
            set_r50i_gaming_mode,
            set_r50i_touch_tone,
            set_r50i_button_action,
            render_test_signal_wav,
            export_equalizer_apo_text,
            import_equalizer_apo_text,
            export_preset_json,
            import_preset_json,
        ])
        .build(tauri::generate_context!())
        .expect("SoundBettrCore failed to start")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
