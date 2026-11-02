# SoundBettrCore v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild SoundBettrCore as one Tauri 2 app for Windows and Android that recognizes every audio output, applies that output's saved profile, announces the connection with a bundled Kokoro voice, and provides EQ, balance, AutoEq, a hearing test, a balance check and a blind A/B.

**Architecture:** A pure Rust core (identity, store, EQ math, AutoEq, tests, templates) under `app/src-tauri/src/`, plus thin platform layers: EqualizerAPO and WinRT/WASAPI on Windows, and a Kotlin Tauri plugin (`DynamicsProcessing`, `AudioDeviceCallback`, RFCOMM, sherpa-onnx) on Android. One React UI talks to the core through Tauri commands and events.

**Tech Stack:** Tauri 2, Rust 2024 edition, `windows` 0.62, sherpa-onnx (version pinned in Task 2), React 19, Vite, TypeScript, Tailwind 4, `motion`, zustand, vitest, Kotlin, Android SDK at `C:\Users\J30N\android-sdk`.

**Spec:** `docs/superpowers/specs/2026-11-02-soundbettrcore-v2-design.md`. **Visual reference:** `docs/superpowers/specs/assets/2026-11-02-v2-mockup.html`, which has the exact tokens, markup structure and copy for every screen.

## Global Constraints

- Windows Controlled Folder Access blocks build scripts and Git Bash writes inside `Documents`. Edit in the repo, but run every build and test through `scripts/stage.ps1` (Task 1), which mirrors the repo to `C:\Users\J30N\.sbc-build\repo` and runs there. Use PowerShell, not Git Bash, for file operations in the repo.
- `CARGO_TARGET_DIR` is `C:\Users\J30N\.sbc-build\target`. Drop v0.1's shared `C:\Users\J30N\LodiSave\app\target`.
- EQ bands: `[31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000]` Hz. Gain range -12 to +12 dB in 0.5 dB steps. Q is 1.41 (octave).
- Balance is -6 to +6 dB, negative meaning toward the left, in 0.5 dB steps.
- Preamp is `-max(0, max gain)` unless the user sets it.
- Android limiter at -0.3 dBFS.
- Announcement template default: `Connected to {device}. {profile} profile on. {battery}`. Battery text: `Battery {min(l, r)} percent.`, empty when the device reports none.
- Announcement plays 1.5 s after the route is active (R6).
- Voice: Kokoro only, 8 voices: Heart, Bella, Emma, Nicole, Michael, Fenrir, George, Puck. Default Heart.
- Device hues: `cobalt`, `amber`, `rose`, `jade`. Tokens are copied from the mockup's `:root` and `html[data-theme="dark"]` blocks.
- Font: Manrope 400/500/600 bundled with `@fontsource/manrope`. The CSP allows no remote fonts.
- UI copy: no em dashes, sentence case, no tracked caps. 44px minimum targets, visible focus, every control named.
- Motion is an in-app setting (`settings.motion`, default `true`). Off sets `data-motion="off"`, which removes all animation.
- No network calls except `update_autoeq_db`, and only when the user presses its button.
- Commit messages are plain, with no co-author trailer (the user's rule). Work on branch `v2-redesign`.

## Review Focus

1. **Hearing test through an active EQ:** the test tones pass through the device's own profile on Windows (APO) and Android (session 0), which skews the thresholds. A person expects the test to measure their ears, not the EQ. Task 8 adds a bypass, and Task 17 tests that a hearing session sets it and clears it.
2. **Bluetooth flapping:** earbuds that drop and reconnect within seconds. A person expects one announcement, not three. Task 6 tests that changes within the 1.5 s debounce produce a single `Recognized` and a single announce request.
3. **Device names in EqualizerAPO config:** a user renames a device to `R50i"; Preamp: +20 dB`. A person expects only the name to change. Task 8 tests that APO blocks select by endpoint ID and that names never reach the config text.
4. **Corrupt or missing store file:** the app was killed mid-write. A person expects the app to open with their other data intact, or empty, never a crash. Task 5 tests atomic save and recovery from a truncated JSON.
5. **Two identical devices:** two R50i NC sets with the same name. A person expects two separate entries with separate profiles. Task 6 tests that identity uses the address, and that the setup card's default name gets a " 2" suffix.

---

### Task 1: Scaffold, staging runner and kept protocol code

**Files:**
- Create: `scripts/stage.ps1`, `app/` (Tauri 2 + React + Vite + TS template), `app/src-tauri/src/a3959.rs` (moved from `desktop/src-tauri/src/a3959.rs`, unchanged)
- Modify: `.gitignore` (add `app/node_modules/`, `app/dist/`, `app/src-tauri/gen/schemas/`)
- Test: the existing 4 tests inside `a3959.rs`

**Interfaces:**
- Produces: `pwsh scripts/stage.ps1 "<command>" [-Dir app|app/src-tauri]`, which mirrors the repo with `robocopy /MIR` (excluding `.git`, `node_modules`, `target`, `dist`) to `C:\Users\J30N\.sbc-build\repo`, sets `CARGO_TARGET_DIR`, `ANDROID_HOME` and `ANDROID_SDK_ROOT`, runs the command in that directory, and returns its exit code.

- [ ] **Step 1:** Write `scripts/stage.ps1` as above. `node_modules` lives only in the stage copy, so the script runs `npm ci` when `package-lock.json` changed.
- [ ] **Step 2:** Create the Tauri 2 app in `app/` (`npm create tauri-app`, React + TS template), identifier `com.jhems.soundbettrcore`, version `0.2.0`. Add `tailwindcss @tailwindcss/vite motion zustand @fontsource/manrope`, and dev deps `vitest jsdom @testing-library/react`. Set `"test": "vitest run"`.
- [ ] **Step 3:** Move `a3959.rs` into `app/src-tauri/src/` and declare `mod a3959;` in `lib.rs`.
- [ ] **Step 4:** Run `pwsh scripts/stage.ps1 "cargo test" -Dir app/src-tauri`. Expected: 4 passed (`test_openscq30_set_equalizer_with_drc_vector`, `test_parse_a3959_state_update_packet_issue_149`, `test_rejects_corrupted_checksum_and_old_firmware_gaming_mode`, `test_outbound_sound_modes_and_button_packets`).
- [ ] **Step 5:** Run `pwsh scripts/stage.ps1 "npm test -- --passWithNoTests" -Dir app` and `pwsh scripts/stage.ps1 "npm run tauri build -- --debug" -Dir app`. Expected: both exit 0.
- [ ] **Step 6:** Commit with `git add scripts/stage.ps1 app .gitignore` and the message `feat: scaffold Tauri 2 app and staging runner`.

### Task 2: Spike R2, Kokoro through sherpa-onnx on Windows

**Files:**
- Create: `app/src-tauri/src/voice.rs`, `app/src-tauri/resources/kokoro/` (model files, Git LFS if over 50 MB per file)
- Test: `voice.rs` `#[cfg(test)]`

**Interfaces:**
- Produces: `pub trait Synth { fn render(&self, text: &str, voice: Voice) -> Result<Pcm, VoiceError>; }`, `pub struct Pcm { pub samples: Vec<f32>, pub rate: u32 }`, `pub enum Voice { Heart, Bella, Emma, Nicole, Michael, Fenrir, George, Puck }` with `fn sid(self) -> i32` and `fn from_id(&str) -> Option<Voice>` (ids `af_heart`, `af_bella`, `bf_emma`, `af_nicole`, `am_michael`, `am_fenrir`, `bm_george`, `am_puck`), and `pub struct KokoroSynth` implementing `Synth`, built with `KokoroSynth::load(dir: &Path)`.

- [ ] **Step 1:** **Ask the user before downloading.** Get the sherpa-onnx Kokoro release that contains `af_heart` (the multi-lang v1.0 set, int8 if offered) from the k2-fsa GitHub releases. Record the exact file names, total size and license in `resources/kokoro/README.md`. If the size is not about 86 MB, update spec §10 with the real number in the same commit.
- [ ] **Step 2:** Map the 8 voice ids to speaker ids from the release's voices list (do not guess). Write a test `voice_ids_map_to_release_sids` that asserts each `Voice::sid()` against that list.
- [ ] **Step 3:** Write the test `kokoro_renders_one_line`: `KokoroSynth::load(resources)?.render("Connected to test device.", Voice::Heart)` returns `rate == 24000` and `samples.len() > 12000`, with no NaN. Mark it `#[ignore]` unless the model is present (`SBC_KOKORO=1`).
- [ ] **Step 4:** Add the official `sherpa-onnx` crate, pinning the version that builds on `x86_64-pc-windows-msvc`. Implement `KokoroSynth`. Run `pwsh scripts/stage.ps1 '$env:SBC_KOKORO=1; cargo test voice -- --include-ignored' -Dir app/src-tauri`. Expected: PASS. Write the WAV to `C:\Users\J30N\.sbc-build\heart.wav` and ask the user to listen.
- [ ] **Step 5:** If the crate cannot build or load on Windows, stop and report to the user with the error. The fallback is the sherpa-onnx C API through `bindgen`, which needs the user's go-ahead.
- [ ] **Step 6:** Commit with the message `feat: Kokoro voice through sherpa-onnx on Windows`.

### Task 3: Spike R2, Android plugin speaking one line

**Files:**
- Create: `app/src-tauri/gen/android/` (`tauri android init`), `app/plugins/android-audio/` (Tauri plugin: `android/src/main/java/com/jhems/soundbettrcore/audio/AudioPlugin.kt`, `build.gradle.kts`, `src/lib.rs`)
- Test: manual on a phone or emulator

**Interfaces:**
- Produces: plugin command `speak(text: String, voice: String)` returning `{ ok: bool, error?: string }`, exposed to Rust as `android_audio::speak`.

- [ ] **Step 1:** Run `tauri android init`, and add Rust targets `aarch64-linux-android` and `x86_64-linux-android`.
- [ ] **Step 2:** Add the sherpa-onnx Android AAR from the same release as Task 2 to the plugin. Copy the Kokoro files into Android assets and load them with `OfflineTts`.
- [ ] **Step 3:** Implement `AudioPlugin.speak` to render and play through `AudioTrack` (`USAGE_ASSISTANCE_SONIFICATION`).
- [ ] **Step 4:** Run `pwsh scripts/stage.ps1 "npm run tauri android build -- --debug --apk" -Dir app`, install, and call `speak("Connected to test device.", "af_heart")` from a temporary button. Expected: you hear it. Record the APK size in the commit message.
- [ ] **Step 5:** Commit with the message `feat: Android audio plugin speaks with Kokoro (APK <size> MB)`.

### Task 4: EQ math

**Files:**
- Create: `app/src-tauri/src/eq.rs`
- Test: `eq.rs` `#[cfg(test)]`

**Interfaces:**
- Produces: `pub const BANDS: [f32; 10]`, `pub const Q: f32 = 1.41`, `pub struct Biquad { b0, b1, b2, a1, a2: f64 }`, `pub fn peaking(fs: f64, f0: f64, gain_db: f64, q: f64) -> Biquad` (RBJ cookbook), `pub fn response_db(gains: &[f32; 10], freqs: &[f32]) -> Vec<f32>` (sum of the 10 peaking responses at fs 48000), `pub fn auto_preamp(gains: &[f32; 10]) -> f32`, `pub fn snap(v: f32, min: f32, max: f32) -> f32` (clamp, then round to 0.5).

- [ ] **Step 1:** Write the failing tests:
  - `single_band_peaks_at_its_center`: gains with only index 5 = 6.0. `response_db` at 1000 Hz is within 6.0 ± 0.05, and at 31 Hz it is under 0.1.
  - `flat_is_zero`: all zeros give every response within ±0.001.
  - `preamp_is_negative_max_boost`: `[2, 1, -4.2, -3.4, -1, 0, 3.2, 3.8, 1, 0]` gives -3.8, and all negative gains give 0.0.
  - `snap_rounds_and_clamps`: `snap(3.26, -12, 12) == 3.5` and `snap(20.0, -12, 12) == 12.0`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "cargo test eq" -Dir app/src-tauri`. Expected: FAIL, not defined.
- [ ] **Step 3:** Implement the functions with the signatures above.
- [ ] **Step 4:** Run the same command. Expected: 4 passed.
- [ ] **Step 5:** Commit with the message `feat: EQ biquad math and preamp rule`.

### Task 5: Store

**Files:**
- Create: `app/src-tauri/src/store.rs`
- Test: `store.rs` `#[cfg(test)]` (uses `tempfile` as a dev dependency)

**Interfaces:**
- Consumes: `eq::snap`
- Produces (all `serde` camelCase, also hand-written in `app/src/types.ts`):
  - `pub enum Kind { Bluetooth, Usb, Wired, Builtin }`
  - `pub enum Hue { Cobalt, Amber, Rose, Jade }`
  - `pub struct Device { id: String, kind: Kind, name: String, hue: Hue, announce: bool, profile_id: String, last_seen: i64, is_new: bool, hardware: Option<String> }`
  - `pub enum Source { Flat, Autoeq, Hearing, Ab, Custom }`
  - `pub struct Profile { id: String, name: String, source: Source, autoeq_model: Option<String>, gains: [f32; 10], preamp: Option<f32>, balance: f32, surround: bool, crossfeed: bool, hearing: Option<Hearing> }`
  - `pub struct Hearing { left: Vec<f32>, right: Vec<f32> }`
  - `pub enum Theme { Light, Dark, System }`
  - `pub struct Settings { theme: Theme, motion: bool, voice: String, template: String }`
  - `pub struct Store { devices: Vec<Device>, profiles: Vec<Profile>, settings: Settings }`
  - Methods: `Store::load(dir: &Path) -> Store`, `save(&self, dir: &Path) -> io::Result<()>`, `upsert_device(&mut self, Device)`, `forget(&mut self, id: &str, current: Option<&str>) -> Result<(), StoreError>`, `profile_for(&self, device_id: &str) -> &Profile`, `new_flat_profile(&mut self, name: &str) -> String`.
  - `pub enum StoreError { Connected, NotFound }`

- [ ] **Step 1:** Write the failing tests:
  - `round_trip_keeps_everything`: save, then load, gives an equal store.
  - `forget_connected_device_is_refused`: `forget("a", Some("a"))` returns `Err(Connected)`.
  - `forget_removes_device_and_unused_profile`.
  - `truncated_file_loads_empty_and_keeps_backup`: write `{"devices":[` to `store.json`. `load` returns the default store, and `store.json.corrupt` exists.
  - `save_is_atomic`: save writes `store.json.tmp`, then renames it. After a simulated crash (the tmp exists, the old file is intact), `load` returns the old data.
  - `defaults`: theme Light, motion true, voice `af_heart`, and the template exactly as in Global Constraints.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "cargo test store" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 3:** Implement. `save` writes the tmp file, then renames it over `store.json`. `load` renames an unparsable file to `.corrupt`.
- [ ] **Step 4:** Run the tests. Expected: 6 passed.
- [ ] **Step 5:** Commit with the message `feat: device and profile store with atomic save`.

### Task 6: Device identity and connection state machine

**Files:**
- Create: `app/src-tauri/src/device.rs`
- Test: `device.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `store::{Store, Device, Kind}`
- Produces:
  - `pub enum Output { Bluetooth { address: String, name: String }, Usb { key: String, name: String }, Wired { key: String }, Builtin { key: String, name: String } }`. Windows fills `key` with the endpoint ID. Android uses `"usb:" + productName`, `"wired"` and `"builtin"`.
  - `pub fn device_id(o: &Output) -> String`: `bt:<address uppercase>`, `usb:<key>`, `wired:<key>` or `builtin:<key>`.
  - `pub fn is_a3959_name(name: &str) -> bool`: true only for names containing `r50i`, `p30i` or `a3959` (case-insensitive).
  - `pub enum ConnState { Searching, Recognized { device_id: String }, New { device_id: String }, Choose { device_ids: Vec<String> }, Error { cause: String } }` (serde tag `state`). `Choose` is for a shared wired key with two or more saved wired devices (Android). Their ids are `wired:<key>`, `wired:<key>#2` and so on.
  - `pub struct Tracker { debounce_ms: u64 }` with `fn on_output(&mut self, store: &mut Store, o: Option<Output>, now_ms: u64) -> Option<Transition>` and `fn choose(&mut self, store: &mut Store, id: &str) -> Transition`, where `pub struct Transition { pub state: ConnState, pub announce: bool }`.

- [ ] **Step 1:** Write the failing tests:
  - `soundcore_name_alone_is_not_a3959`: `"Soundcore Life Q30"` gives false. `"Soundcore R50i NC"` and `"Jhems's P30i"` give true.
  - `unknown_output_becomes_new_with_flat_profile`: the store gains a device with `is_new: true`, profile source `Flat`, and name from the output.
  - `known_output_is_recognized_and_announces`.
  - `flapping_within_debounce_announces_once`: Some(A) at 0 ms, None at 400 ms, Some(A) at 900 ms. Exactly one transition has `announce: true` (Review Focus 2).
  - `same_name_two_addresses_two_devices`: two Bluetooth outputs named `R50i NC`, addresses `AA:..` and `BB:..`. Two devices exist, and the second is named `R50i NC 2` (Review Focus 5).
  - `none_output_is_searching`.
  - `wired_with_two_saved_asks_choose`: two saved devices with ids `wired:wired` and `wired:wired#2`, then `Output::Wired { key: "wired" }`, gives `Choose { device_ids }` with both ids. `Tracker::choose(&mut self, store, id)` then gives `Recognized` with `announce: true`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "cargo test device" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 3:** Implement. `debounce_ms` defaults to 1500. A disconnect followed by a reconnect of the same id inside the window gives `Recognized` with `announce: false`.
- [ ] **Step 4:** Run the tests. Expected: 7 passed.
- [ ] **Step 5:** Commit with the message `feat: device identity and connection state machine`.

### Task 7: AutoEq index, search and 10-band fit

**Files:**
- Create: `scripts/build-autoeq-index.mjs`, `app/src-tauri/resources/autoeq/index.json.gz`, `app/src-tauri/src/autoeq.rs`
- Test: `autoeq.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `eq::{BANDS, response_db, peaking}`
- Produces: `pub struct Filter { pub fc: f32, pub gain: f32, pub q: f32 }`, `pub struct Entry { pub model: String, pub kind: String, pub preamp: f32, pub filters: Vec<Filter> }`, `pub struct Index(Vec<Entry>)` with `Index::load_gz(path) -> io::Result<Index>`, `search(&self, q: &str, limit: usize) -> Vec<(usize, &Entry)>`, `pub fn parse_parametric_eq(txt: &str) -> Result<Entry, String>` (AutoEq `ParametricEQ.txt`), and `pub fn fit_10band(filters: &[Filter]) -> [f32; 10]`.

- [ ] **Step 1:** Check R4: read the LICENSE in `jaakkopasanen/AutoEq`. If `results/` is not covered by MIT or an equally permissive license, stop and report to the user. Otherwise record the license in `resources/autoeq/README.md`.
- [ ] **Step 2:** Write the failing tests:
  - `parses_parametric_eq_file`: a 3-filter sample (`Preamp: -6.2 dB`, `Filter 1: ON PK Fc 105 Hz Gain -4.2 dB Q 0.70`, etc.) parses exactly.
  - `search_is_case_insensitive_and_limited`.
  - `fit_single_1k_filter`: `[Filter { fc: 1000, gain: 6, q: 1.41 }]` gives `gains[5]` within 6 ± 0.5 and every other band within ±1.0.
  - `fit_stays_in_range`: a +20 dB filter gives every gain within -12..=12.
- [ ] **Step 3:** Run `pwsh scripts/stage.ps1 "cargo test autoeq" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 4:** Implement `fit_10band`. Sample the target response at the 10 band centers, then run 4 passes of `gains[i] += target[i] - response_db(gains)[i]`, snapping each band with `eq::snap`.
- [ ] **Step 5:** Write `build-autoeq-index.mjs`. Given a local AutoEq clone path, it walks `results/**/ParametricEQ.txt` and writes `{model, kind, preamp, filters}[]` gzipped. **Ask the user before cloning AutoEq.** Run it and record the entry count and size in the commit message.
- [ ] **Step 6:** Run the tests. Expected: 4 passed.
- [ ] **Step 7:** Commit with the message `feat: bundled AutoEq index, search and 10-band fit`.

### Task 8: EqualizerAPO writer and surround

**Files:**
- Create: `app/src-tauri/src/apo.rs` (new, replacing v0.1's), `app/src-tauri/resources/hrir/` (SADIE II subject file plus `README.md` with license and citation)
- Test: `apo.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `store::Profile`, `eq::{BANDS, Q, auto_preamp}`
- Produces: `pub struct ApoTarget { pub endpoint_id: String, pub profile: Profile, pub bypass: bool }`, `pub fn render(targets: &[ApoTarget], hrir: &Path) -> String`, `pub fn write_live(text: &str) -> io::Result<PathBuf>` (to `C:\ProgramData\SoundBettrCore\live_apo.txt`), and `pub fn apo_status() -> ApoStatus { installed: bool, hooked: bool }`.

- [ ] **Step 1:** Check R1: confirm the SADIE II package's license file allows bundling with attribution. If it does not, stop and report to the user.
- [ ] **Step 2:** Write the failing tests:
  - `one_block_per_device`: two targets produce two `Device: <endpoint_id>` lines in order, each followed by its own `Preamp:` and 10 `Filter: ON PK Fc <f> Hz Gain <g> dB Q 1.41` lines.
  - `balance_left_attenuates_right`: balance -2 gives `Channel: R` then `Preamp: -2 dB`, and then `Channel: all`.
  - `bypass_writes_no_filters`: a target with `bypass: true` gives a `Device:` line and no `Filter` lines (Review Focus 1).
  - `names_never_reach_config`: a profile named `x"; Preamp: +20 dB` does not appear in the output, and the output has no `+20` (Review Focus 3).
  - `surround_adds_convolution`: `Convolution: <hrir path>` appears only when `surround`.
  - `crossfeed_adds_copy`.
- [ ] **Step 3:** Run `pwsh scripts/stage.ps1 "cargo test apo" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 4:** Implement. The Windows installer keeps hooking `live_apo.txt` into EqualizerAPO's `config.txt` (unchanged `installer/`).
- [ ] **Step 5:** Run the tests. Expected: 6 passed.
- [ ] **Step 6:** Commit with the message `feat: per-device EqualizerAPO config with bypass and HRIR surround`.

### Task 9: Announcer

**Files:**
- Modify: `app/src-tauri/src/voice.rs`
- Test: `voice.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `Synth`, `Voice`, `store::{Device, Profile, Settings}`
- Produces: `pub fn fill_template(tpl: &str, device: &str, profile: &str, battery: Option<(u8, u8)>) -> String`, and `pub struct Announcer<S: Synth> { synth: S, cache_dir: PathBuf }` with `fn wav_for(&self, text: &str, voice: Voice) -> Result<PathBuf, VoiceError>` (cached by a SHA-256 of `voice_id + text`).

- [ ] **Step 1:** Write the failing tests:
  - `fills_all_tokens`: the default template with `Jhems's R50i NC`, `Harman fix` and `Some((82, 78))` gives exactly `Connected to Jhems's R50i NC. Harman fix profile on. Battery 78 percent.`
  - `no_battery_leaves_no_gap`: `None` gives `Connected to Wired earphones. Flat profile on.`
  - `cache_hit_skips_render`: a counting fake `Synth` is called once for two identical requests.
  - `voice_change_misses_cache`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "cargo test voice" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 3:** Implement. Write the WAV as 16-bit PCM mono at the synth rate.
- [ ] **Step 4:** Run the tests. Expected: 4 passed (plus Task 2's).
- [ ] **Step 5:** Commit with the message `feat: announcement template and voice cache`.

### Task 10: Hearing test and A/B logic

**Files:**
- Create: `app/src-tauri/src/tests_audio.rs`
- Test: `tests_audio.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `eq::snap`
- Produces:
  - `pub const HT_FREQS: [f32; 6] = [250., 500., 1000., 2000., 4000., 8000.]`
  - `pub enum Ear { Left, Right }`
  - `pub enum HtStep { Play { ear: Ear, freq: f32, db: f32 }, Done { left: Vec<f32>, right: Vec<f32> } }`
  - `pub struct Staircase` with `new() -> (Staircase, HtStep)` and `answer(&mut self, heard: bool) -> HtStep`
  - `pub fn apply_hearing(left: &[f32], right: &[f32]) -> ([f32; 10], f32)`
  - `pub struct Ab` with `new(a: String, b: String, x_is_a: [bool; 6])`, `round(&self) -> usize`, `profile_for(&self, side: Side) -> &str`, `pick(&mut self, side: Side) -> Option<AbResult>`
  - `pub enum Side { X, Y }`
  - `pub struct AbResult { winner: String, wins: u8, rounds: u8 }`

- [ ] **Step 1:** Write the failing tests:
  - `starts_left_250_at_minus_20`.
  - `heard_steps_down_10_missed_steps_up_5`.
  - `threshold_is_level_heard_twice_ascending`: answers at 250 Hz of -20 heard, -30 heard, -40 missed, -35 heard (first ascending hit at -35), -45 missed, -40 missed, -35 heard (second ascending hit at -35). The recorded threshold is -35, and the next step is 500 Hz at -20.
  - `floor_and_ceiling`: always heard reaches -80 and records -80. Never heard records 0.
  - `left_then_right_then_done`: 12 frequency results.
  - `apply_boosts_relative_to_best`: left `[-40,-40,-40,-40,-25,-10]` and right the same give band 7 (4 kHz) +5.0, band 8 (8 kHz) +6.0 (capped), bands 3..=6 0.0, and balance 0.
  - `apply_balance_toward_worse_ear`: left thresholds 8 dB worse on average than right give balance -2.0 (toward the left).
  - `ab_maps_and_reveals_winner`: `x_is_a = [true, false, true, false, true, false]`, picking X every round, gives a winner of `a` with 3 wins. Picking A every round gives 6 wins.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "cargo test tests_audio" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 3:** Implement. Frequency to band mapping: 250→3, 500→4, 1000→5, 2000→6, 4000→7, 8000→8. Boost = `snap((avg - best) / 3, 0, 6)`. Balance = `snap(-(avgL - avgR) / 4, -6, 6)`. Thresholds are clamped to -80..=0.
- [ ] **Step 4:** Run the tests. Expected: 8 passed.
- [ ] **Step 5:** Commit with the message `feat: hearing test staircase and blind A/B logic`.

### Task 11: Windows outputs and R50i session

**Files:**
- Create: `app/src-tauri/src/win_audio.rs`, `app/src-tauri/src/rfcomm.rs` (`#[cfg(windows)]`, with pure helpers outside the cfg)
- Test: pure helpers in each file

**Interfaces:**
- Consumes: `device::Output`, `a3959::*`
- Produces:
  - `pub fn classify_endpoint(id: &str, name: &str, form_factor: u32, enumerator: &str, bt_address: Option<String>) -> device::Output`
  - `pub fn watch(tx: std::sync::mpsc::Sender<Option<Output>>) -> Result<Watcher, String>` (`IMMNotificationClient` on `eRender`/`eMultimedia` default changes, plus an initial send)
  - `pub struct A3959Session` with `open(address: u64) -> Result<Self, String>`, `state(&mut self) -> Result<A3959State, String>`, `send(&mut self, packet: &[u8]) -> CommandResult`
  - `pub enum CommandResult { Confirmed, NoReply, Failed(String) }`
  - `pub fn is_ack_for(sent: &[u8], reply: &[u8]) -> bool`
  - `pub trait Hardware { fn state(&mut self) -> Result<A3959State, String>; fn send(&mut self, packet: &[u8]) -> CommandResult; }`, implemented by `A3959Session` (and on Android by the plugin-backed `A3959Socket` in Task 19)

- [ ] **Step 1:** Write the failing tests:
  - `bluetooth_enumerator_maps_to_bluetooth`: enumerator `BTHENUM` with an address gives `Output::Bluetooth`.
  - `headphone_form_factor_maps_to_wired`.
  - `speaker_form_factor_maps_to_builtin`.
  - `usb_enumerator_maps_to_usb`.
  - `ack_matches_command_bytes`: the reply header `09 ff 00 00 01` with the same command id as the sent packet is true. A different command id is false. A bad checksum is false.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "cargo test win_audio rfcomm" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 3:** Implement the helpers, then `watch` and `A3959Session`. The session keeps one socket open, reads with a 900 ms timeout, reconnects once on failure, and never fabricates state (v0.1's `simulated` field is gone). Resolve the Bluetooth address from the endpoint's container ID. When that fails, match by name to `BluetoothFindFirstDevice` results (R5).
- [ ] **Step 4:** Run the tests. Expected: 5 passed.
- [ ] **Step 5:** Manual check with the R50i NC: connecting prints `Bluetooth { address, .. }`, `state()` returns battery values, and an ANC change returns `Confirmed`.
- [ ] **Step 6:** Commit with the message `feat: Windows output watcher and persistent A3959 session`.

### Task 12: Core service, commands and events

**Files:**
- Create: `app/src-tauri/src/core.rs`
- Modify: `app/src-tauri/src/lib.rs` (register commands, start the watcher and tray)
- Test: `core.rs` `#[cfg(test)]` with a fake platform

**Interfaces:**
- Consumes: everything from Tasks 4 to 11
- Produces:
  - `pub trait Platform { fn apply(&self, device: &Device, profile: &Profile, bypass: bool) -> Result<(), String>; fn play_wav(&self, path: &Path); fn a3959(&self, device: &Device) -> Option<Box<dyn Hardware>>; }`
  - `pub struct Snapshot { devices, profiles, settings, conn: ConnState, current: Option<String>, apo: Option<ApoStatus>, speaking: bool }`
  - Tauri commands (TS names identical, args camelCase): `get_snapshot`, `update_device(device)`, `forget_device(id)`, `update_profile(profile)`, `search_autoeq(query, limit)`, `apply_autoeq(deviceId, index)`, `announce(deviceId)`, `preview_voice(voice, text)`, `set_settings(settings)`, `set_bypass(on)`, `hearing_start(deviceId)`, `hearing_answer(heard)`, `hearing_stop()`, `hearing_apply(deviceId)`, `ab_start(deviceId, a, b)`, `ab_listen(side)`, `ab_pick(side)`, `hw_command(deviceId, command)`, `choose_device(id)`, `export_store()` returning `String` (JSON), `import_store(json)`, `update_autoeq_db()`.
  - `pub enum HwCommand { Noise { mode: AmbientSoundMode, anc: AncMode }, Eq { gains_tenths_db: [i16; 8] }, Gaming(bool), Touch { side: EarbudSide, press: ButtonPressKind, action: ButtonAction } }`, built into packets with the kept `a3959::build_*` functions and returning `CommandResult`. Every mutating command returns `Snapshot` or `Result<Snapshot, String>`.
  - Events: `snapshot` (Snapshot), `conn-state` (ConnState), `speaking` (bool), `hearing-step` (HtStep).

- [ ] **Step 1:** Write the failing tests against a `FakePlatform` that records calls:
  - `new_output_applies_flat_and_does_not_announce`.
  - `known_output_applies_profile_then_announces_after_1500ms`.
  - `update_profile_on_current_device_reapplies`.
  - `hearing_start_sets_bypass_and_stop_clears_it` (Review Focus 1).
  - `ab_listen_applies_the_mapped_profile`.
  - `forget_current_is_refused`.
  - `voice_failure_still_shows_state`: a failing `Synth` leaves `conn` as `Recognized`, emits `speaking: false`, and returns no error.
  - `import_rejects_invalid_json_and_keeps_store`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "cargo test core" -Dir app/src-tauri`. Expected: FAIL.
- [ ] **Step 3:** Implement `core.rs` (the state lives in `Mutex<Core>`) and the Windows `Platform` (APO plus `PlaySound` of the WAV on the default endpoint). `update_autoeq_db` downloads the AutoEq index release asset over HTTPS. It is the only network code, and it runs only when called.
- [ ] **Step 4:** Run the tests. Expected: 8 passed. Then run `pwsh scripts/stage.ps1 "cargo test" -Dir app/src-tauri` for the full suite.
- [ ] **Step 5:** Commit with the message `feat: core service, Tauri commands and events`.

### Task 13: UI shell, tokens, theme and motion

**Files:**
- Create: `app/src/styles/tokens.css` (copied from the mockup's two token blocks), `app/src/styles/base.css`, `app/src/api.ts` (typed `invoke` and `listen` wrappers), `app/src/types.ts`, `app/src/store.ts` (zustand), `app/src/App.tsx`, `app/src/components/Nav.tsx`
- Test: `app/src/__tests__/shell.test.tsx`

**Interfaces:**
- Consumes: Task 12 commands and events
- Produces: `useApp()` zustand store `{ snap: Snapshot | null, route: Route, setRoute(r) }`, `type Route = 'now' | 'devices' | 'tune' | 'tests' | 'settings'`, and a `<Nav>` that shows the bottom bar under 700px container width and the rail from 700px up (container query, as in the mockup).

- [ ] **Step 1:** Write the failing tests (with `invoke` mocked):
  - `theme_dark_sets_data_theme`.
  - `theme_system_follows_media_query`.
  - `motion_off_sets_data_motion_off`.
  - `nav_has_five_named_buttons_and_marks_current`: `aria-current="page"` is on the active button only.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "npm test" -Dir app`. Expected: FAIL.
- [ ] **Step 3:** Implement, porting the mockup's CSS for the nav, sliding indicator, rail tooltips and segmented control.
- [ ] **Step 4:** Run the tests. Expected: 4 passed.
- [ ] **Step 5:** Commit with the message `feat: UI shell, tokens, theme and motion setting`.

### Task 14: Now screen

**Files:**
- Create: `app/src/screens/Now.tsx`, `app/src/components/DeviceCard.tsx`, `app/src/components/Curve.tsx` (`gainsPath(gains: number[]): string`, ported from the mockup)
- Test: `app/src/__tests__/now.test.tsx`

- [ ] **Step 1:** Write the failing tests:
  - `searching_shows_checking_outputs`.
  - `recognized_shows_name_and_tap_to_hear`.
  - `new_shows_setup_card_that_routes_to_devices`.
  - `error_shows_cause`.
  - `battery_hidden_when_absent`.
  - `card_click_calls_announce`.
  - `noise_control_only_for_a3959`.
  - `choose_lists_wired_sets_and_calls_choose_device`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "npm test" -Dir app`. Expected: FAIL.
- [ ] **Step 3:** Implement from the mockup, including the connect sequence motion. Rings animate only in `searching`. The noise control shows sent, confirmed or failed from `hw_command`.
- [ ] **Step 4:** Run the tests. Expected: 8 passed.
- [ ] **Step 5:** Commit with the message `feat: Now screen with connection states`.

### Task 15: Devices screen

**Files:**
- Create: `app/src/screens/Devices.tsx`
- Test: `app/src/__tests__/devices.test.tsx`

- [ ] **Step 1:** Write the failing tests:
  - `rename_calls_update_device`.
  - `empty_name_saves_as_unnamed_device`.
  - `hue_radio_updates_and_is_checked`.
  - `forget_opens_dialog_and_confirm_calls_forget`.
  - `forget_disabled_for_current`.
  - `new_device_shows_save_this_device`.
  - `a3959_device_shows_earbuds_section`: the hardware EQ (8 bands, 100 Hz to 12.8 kHz), gaming mode and touch controls each call `hw_command` and show its result. Other devices have no such section.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "npm test" -Dir app`. Expected: FAIL.
- [ ] **Step 3:** Implement. The Forget confirm is handled on the button click, not the dialog `close` event (see the mockup's bug note). The earbuds section is new UI. Follow the Settings panel pattern from the mockup.
- [ ] **Step 4:** Run the tests. Expected: 7 passed.
- [ ] **Step 5:** Commit with the message `feat: Devices screen`.

### Task 16: Tune screen

**Files:**
- Create: `app/src/screens/Tune.tsx`, `app/src/components/EqSliders.tsx`
- Test: `app/src/__tests__/tune.test.tsx`

- [ ] **Step 1:** Write the failing tests:
  - `slider_input_updates_profile_and_marks_edited`.
  - `reset_to_flat`.
  - `preamp_text_matches_rule` ("Preamp -3.8 dB, so the loudest boost cannot clip." and "No preamp needed. Nothing is boosted.").
  - `autoeq_search_and_apply`.
  - `autoeq_empty_state_text`.
  - `crossfeed_hidden_on_android`.
  - `apo_missing_shows_install_prompt`.
  - `android_surround_shows_spatializer_state`: unavailable shows "This phone has no spatial audio". Available but off shows a button calling `open_spatializer_settings`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "npm test" -Dir app`. Expected: FAIL.
- [ ] **Step 3:** Implement. Slider input updates locally at once and calls `update_profile` throttled to 100 ms.
- [ ] **Step 4:** Run the tests. Expected: 8 passed.
- [ ] **Step 5:** Commit with the message `feat: Tune screen`.

### Task 17: Tests screen

**Files:**
- Create: `app/src/screens/Tests.tsx`, `app/src/audio/tone.ts` (`tone(freq, db, pan, secs)` with WebAudio, ported from the mockup)
- Test: `app/src/__tests__/tests.test.tsx`

- [ ] **Step 1:** Write the failing tests:
  - `hearing_start_calls_hearing_start_and_plays_first_step`.
  - `answers_forward_to_core`.
  - `leaving_screen_mid_test_calls_hearing_stop` (clears the bypass, Review Focus 1).
  - `done_shows_chart_with_two_labeled_lines`.
  - `balance_buttons_move_half_db`.
  - `ab_flow_six_rounds_then_reveal`.
  - `ab_same_profiles_disables_start`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "npm test" -Dir app`. Expected: FAIL.
- [ ] **Step 3:** Implement. Show "Set a comfortable volume first" before the first tone. The result chart is labeled "Relative to your best band".
- [ ] **Step 4:** Run the tests. Expected: 7 passed.
- [ ] **Step 5:** Commit with the message `feat: Tests screen`.

### Task 18: Settings screen

**Files:**
- Create: `app/src/screens/Settings.tsx`
- Test: `app/src/__tests__/settings.test.tsx`

- [ ] **Step 1:** Write the failing tests:
  - `eight_voices_heart_default`.
  - `play_calls_preview_voice`.
  - `template_edit_saves_and_play_it`.
  - `autoeq_update_button_only_network_entry` (asserts no other command in `api.ts` uses the network).
  - `theme_and_motion_controls`.
  - `export_saves_json_and_import_shows_error_on_bad_file`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 "npm test" -Dir app`. Expected: FAIL.
- [ ] **Step 3:** Implement from the mockup's Settings view, and add a "Profiles" section with Export and Import (file save and open through `@tauri-apps/plugin-dialog`).
- [ ] **Step 4:** Run the tests. Expected: 6 passed.
- [ ] **Step 5:** Commit with the message `feat: Settings screen`.

### Task 19: Android platform

**Files:**
- Modify: `app/plugins/android-audio/android/src/main/java/com/jhems/soundbettrcore/audio/AudioPlugin.kt`
- Create: `AudioService.kt` (ported from v0.1 `SoundBettrService.kt`: foreground service, `DynamicsProcessing` on session 0, -0.3 dBFS limiter), `OutputMapper.kt`, `A3959Socket.kt`, `app/src-tauri/src/android.rs` (the `Platform` implementation through the plugin)
- Test: `app/plugins/android-audio/android/src/test/java/com/jhems/soundbettrcore/audio/OutputMapperTest.kt`

**Interfaces:**
- Produces: plugin events `output` (`{ kind, address?, key, name }`, the Rust `Output` shape) and `spatializer` (`{ available, enabled }`). Plugin commands: `apply(profile, bypass)`, `play(path)`, `a3959_send(bytes)` returning `{ result: 'confirmed' | 'noReply' | 'failed', reply?: number[] }`, and `open_spatializer_settings()`.

- [ ] **Step 1:** Write the failing Kotlin tests:
  - `bluetoothA2dpMapsToBluetoothWithMac`.
  - `wiredHeadsetMapsToSharedWiredKey`.
  - `usbHeadsetKeyUsesProductName`.
  - `balanceMinus2AttenuatesRightInputGain`.
  - `tenBandsMapToPreEqCutoffs`.
- [ ] **Step 2:** Run `pwsh scripts/stage.ps1 ".\gradlew.bat :android-audio:testDebugUnitTest" -Dir app/src-tauri/gen/android`. Expected: FAIL.
- [ ] **Step 3:** Implement. Use `AudioManager.registerAudioDeviceCallback` and the current media route. When session 0 is refused, `apply` returns an error that Tune shows (spec §8). The A3959 socket stays open and reads replies with a 900 ms timeout. Request `BLUETOOTH_CONNECT` at first use.
- [ ] **Step 4:** Run the tests. Expected: 5 passed.
- [ ] **Step 5:** Manual check on a phone: connecting the R50i announces it, the EQ change is audible in Spotify, ANC changes return confirmed, and a wired headset gets its own profile.
- [ ] **Step 6:** Commit with the message `feat: Android platform through audio plugin`.

### Task 20: Release, cleanup and README

**Files:**
- Modify: `scripts/build-release.ps1` (build `app/` for Windows and Android through the stage, embed in `installer/`, version 0.2.0, own target dir), `installer/src/main.rs` (payload name if changed), `README.md`
- Delete: `desktop/`, `android/`, `SoundBettrCore.exe`, `SoundBettrCore.apk`, and v0.1 files in `dist/` (they stay on the v0.1.0 GitHub release)

- [ ] **Step 1:** Update the build script, then run `pwsh scripts/build-release.ps1`. Expected: `dist/SoundBettrCore-Setup-0.2.0.exe`, `dist/SoundBettrCore-0.2.0-portable.exe`, `dist/SoundBettrCore-0.2.0-android.apk` and `SHA256SUMS.txt`, with every test suite passing on the way.
- [ ] **Step 2:** Rewrite the README for v2. Keep only claims the code backs: no "Dolby-style", no "under 15 ms", no 7.1 test bench. Add credits for Kokoro, sherpa-onnx, AutoEq and SADIE II with their licenses, and real screenshots of the new screens.
- [ ] **Step 3:** Delete the v0.1 folders and root binaries.
- [ ] **Step 4:** Run the hardware checklist from spec §9 on Windows and Android, recording pass or fail for each line in the commit message.
- [ ] **Step 5:** Commit with the message `chore: v0.2.0 release build, README, remove v0.1 apps`. Do not push or publish a release without the user's go-ahead.
