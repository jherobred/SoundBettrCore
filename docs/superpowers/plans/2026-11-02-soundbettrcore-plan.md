# SoundBettrCore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and package SoundBettrCore as a Windows desktop app (`SoundBettrCore-Setup-0.1.0.exe` + `SoundBettrCore-0.1.0-portable.exe`) and Android mobile app (`SoundBettrCore-0.1.0-android.apk`) providing host-level Dolby/3D surround/Harman DSP, a built-in media & spatial test player, and direct Bluetooth RFCOMM hardware control for the Anker Soundcore R50i NC (`A3959`).

**Architecture:** Windows (`desktop/`) uses Tauri 2 + Rust (`cpal`, `rustfft`, `windows` WinRT Bluetooth RFCOMM & WASAPI) + React 19 + TypeScript, plus a standalone Rust/Win32 installer (`installer/`). Android (`android/`) uses native Kotlin + Jetpack Compose with a foreground `SoundBettrService` (`DynamicsProcessing` + `Virtualizer` on global audio session `0` + custom float PCM engine for the built-in player/spatial synthesizer) and `R50iBluetoothManager` implementing the identical `A3959` RFCOMM protocol.

**Tech Stack:** Rust 2024, Tauri 2, React 19, TypeScript, Vite, Kotlin 2.0, Jetpack Compose (Material 3 Dark Audiophile Studio / `taste-skill` `data-terminal` + `hallmark` + `antislop`), Gradle 8.

**Spec:** `docs/superpowers/specs/2026-11-02-soundbettrcore-design.md`

## Global Constraints

- Target earbud protocol: Anker Soundcore R50i NC / P30i (`A3959`), outbound prefix `08 EE 00 00 00`, inbound prefix `09 FF 00 00 01`, `sum8` checksum.
- UI/UX governed by `taste-skill` (`data-terminal` seed `1307840498`), `hallmark` (`scope: app`), and `antislop` (`during` mode):
  - All colors and fonts declared as `:root` CSS tokens (`tokens.css` / Compose theme tokens).
  - No `transition: all`, no em dashes in UI copy (R-02), no eyebrow pills (R-09), no purple-blue gradients (R-01), no dead controls (R-26).
  - Minimum 4.5:1 text contrast and 3:1 non-text contrast (R-25), visible `:focus-visible` outlines (R-32), `cursor: pointer` on enabled interactive elements (R-39), `prefers-reduced-motion: reduce` support, and `44x44px` minimum touch targets on mobile (R-03).
- Always output ready-to-use binaries in `dist/`:
  - `dist/SoundBettrCore-Setup-0.1.0.exe`
  - `dist/SoundBettrCore-0.1.0-portable.exe`
  - `dist/SoundBettrCore-0.1.0-android.apk`

## Review Focus

1. **Extreme DSP boost stacking (`+12 dB` EQ + `+100%` Sub-Bass Exciter + `+100%` Vocal Clarity):** Must never clip above `-0.3 dBFS` (`0.966` linear peak) thanks to auto-headroom compensation and brickwall lookahead limiting.
2. **Truncated or corrupted Bluetooth RFCOMM stream frames:** Inbound parser must reject bad `sum8` checksums and handle partial/split packets without panicking.
3. **Old R50i NC firmware (`< 01.60`) or TWS single-earbud mode (`tws_connected == 0`):** State parser must set `gaming_mode = None` when firmware `< 01.60` and mark Single/Triple tap actions disabled when `tws_connected == 0`.
4. **Malformed or exotic EqualizerAPO `.txt` imports:** Parser must gracefully ignore comments/unsupported filter types and clamp out-of-range frequencies (`20..20000 Hz`) and gains (`-15..+15 dB`).
5. **Running on a PC or phone when the R50i NC is not currently connected over Bluetooth:** Host DSP, EqualizerAPO sync, and the built-in spatial/media player must work independently while the R50i NC hardware dock shows a clear disconnected state with scan/connect/demo simulation controls.

---

### Task 1: Core Rust DSP Engine, EqualizerAPO Bridge, & Soundcore `A3959` Protocol (`desktop/src-tauri`)

**Files:**
- Create: `desktop/src-tauri/Cargo.toml`
- Create: `desktop/src-tauri/build.rs`
- Create: `desktop/src-tauri/tauri.conf.json`
- Create: `desktop/src-tauri/src/lib.rs`
- Create: `desktop/src-tauri/src/main.rs`
- Create: `desktop/src-tauri/src/a3959.rs`
- Create: `desktop/src-tauri/src/dsp.rs`
- Create: `desktop/src-tauri/src/apo.rs`
- Create: `desktop/src-tauri/src/audio_engine.rs`
- Create: `desktop/src-tauri/src/bt_rfcomm.rs`
- Create: `desktop/src-tauri/src/presets.rs`

**Interfaces:**
- Produces:
  - `a3959::encode_outbound(cmd: [u8; 2], body: &[u8]) -> Vec<u8>`
  - `a3959::decode_inbound(bytes: &[u8]) -> Result<( [u8; 2], Vec<u8> ), String>`
  - `a3959::apply_drc_8bands(gains_tenths_db: [i16; 8]) -> [i16; 10]`
  - `a3959::encode_eq_with_drc(preset_id: u16, gains_tenths_db: [i16; 8]) -> Vec<u8>`
  - `a3959::A3959State::parse(body: &[u8]) -> Result<A3959State, String>`
  - `dsp::DspChain::process_stereo_frame(&mut self, left: f32, right: f32) -> (f32, f32)`
  - `apo::export_equalizer_apo_txt(profile: &DspProfile) -> String`
  - `apo::import_equalizer_apo_txt(txt: &str) -> Result<DspProfile, String>`
  - Tauri commands: `get_app_state`, `set_dsp_profile`, `select_preset`, `set_ab_bypass`, `set_test_generator`, `scan_r50i_devices`, `connect_r50i`, `set_r50i_sound_mode`, `set_r50i_hardware_eq`, `set_r50i_Flags`, `set_r50i_button`, `export_apo_text`, `import_apo_text`, `sync_system_apo`.

- [ ] **Step 1: Write unit tests in `a3959.rs`, `dsp.rs`, and `apo.rs`** covering the OpenSCQ30 issue #149 91-byte `A3959` state packet, `02 83` DRC matrix bytes, `06 81` sound mode bytes, `04 81` button action bytes, brickwall limiter ceiling `<= 0.967` under `+12 dB` stack, and EqualizerAPO `.txt` round-trip.
- [ ] **Step 2: Implement `a3959.rs`, `dsp.rs`, `apo.rs`, `presets.rs`, `audio_engine.rs`, `bt_rfcomm.rs`, and Tauri commands in `lib.rs`.**
- [ ] **Step 3: Run `cargo test` inside `desktop/src-tauri` and verify all unit tests pass.**

---

### Task 2: Windows Desktop UI (`desktop/src`) with `taste-skill` + `hallmark` + `antislop` Verification

**Files:**
- Create: `desktop/package.json`
- Create: `desktop/tsconfig.json`
- Create: `desktop/vite.config.ts`
- Create: `desktop/index.html`
- Create: `desktop/src/tokens.css`
- Create: `desktop/src/app.css`
- Create: `desktop/src/types.ts`
- Create: `desktop/src/main.tsx`
- Create: `desktop/src/App.tsx`
- Create: `desktop/src/components/RotaryKnob.tsx`
- Create: `desktop/src/components/SpectrumAndStage.tsx`
- Create: `desktop/src/components/R50iHardwarePanel.tsx`
- Create: `desktop/src/components/TestPlayerBar.tsx`

- [ ] **Step 1: Build the React Studio UI** implementing Simple Mode (6 factory presets, 4 tactile rotary knobs, R50i NC ANC/Transparency/Battery strip, instant A/B bypass), Pro Rack Mode (10-band Parametric Host EQ + 8-band Hardware EEPROM EQ flasher, 3-band Dolby compressor, 7.1 binaural spatial radar, crossfeed/reverb controls), Touch Control Remapper, APO/AutoEq `.txt` import/export, and the built-in Spatial 7.1 + Local Audio File Player.
- [ ] **Step 2: Run `npm run build` in `desktop/` and run `taste-skill` / `hallmark` lint checks.**
- [ ] **Step 3: Build the Tauri Windows release executable (`cargo build --release` in `desktop/src-tauri`).**

---

### Task 3: Windows Setup Installer (`installer/`)

**Files:**
- Create: `installer/Cargo.toml`
- Create: `installer/build.rs`
- Create: `installer/src/main.rs`

- [ ] **Step 1: Implement self-contained Windows Setup `.exe` installer** that embeds `SoundBettrCore.exe`, installs to `%LOCALAPPDATA%\Programs\SoundBettrCore`, configures the EqualizerAPO `config.txt` include hook (`C:\ProgramData\SoundBettrCore\live_apo.txt`) when present, creates Start Menu and Desktop shortcuts, and supports `--uninstall` and `--silent`.
- [ ] **Step 2: Compile `installer` in release mode and verify `SoundBettrCore-Setup-0.1.0.exe` runs `--verify-payload` cleanly.**

---

### Task 4: Android Native Kotlin + Jetpack Compose App (`android/`)

**Files:**
- Create: `android/settings.gradle.kts`
- Create: `android/build.gradle.kts`
- Create: `android/gradle.properties`
- Create: `android/local.properties`
- Create: `android/app/build.gradle.kts`
- Create: `android/app/proguard-rules.pro`
- Create: `android/app/src/main/AndroidManifest.xml`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/protocol/A3959Protocol.kt`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/dsp/DspEngine.kt`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/bluetooth/R50iBluetoothManager.kt`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/service/SoundBettrService.kt`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/service/SoundBettrTileService.kt`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/ui/Theme.kt`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/ui/MainScreen.kt`
- Create: `android/app/src/main/java/com/jherobred/soundbettrcore/MainActivity.kt`
- Create: `android/app/src/test/java/com/jherobred/soundbettrcore/A3959AndDspTest.kt`

- [ ] **Step 1: Write JUnit tests in `A3959AndDspTest.kt`** verifying identical `A3959` packet encoding/decoding, DRC matrix math, EqualizerAPO `.txt` import/export, and DSP limiter headroom on Android.
- [ ] **Step 2: Implement `A3959Protocol.kt`, `DspEngine.kt`, `R50iBluetoothManager.kt`, `SoundBettrService.kt` (`DynamicsProcessing` + `Virtualizer` on global session `0` and media sessions + procedural 7.1/sub-bass test player), `SoundBettrTileService.kt`, and the Jetpack Compose UI (`MainScreen.kt`) with responsive 44dp+ touch targets.**
- [ ] **Step 3: Run `./gradlew testDebugUnitTest assembleRelease` and verify the signed APK builds cleanly.**

---

### Task 5: Unified Release Packaging (`scripts/build-release.ps1`) & Verification

**Files:**
- Create: `scripts/build-release.ps1`
- Create: `README.md`
- Create: `.gitignore`

- [ ] **Step 1: Create `scripts/build-release.ps1`** to build and copy `SoundBettrCore-Setup-0.1.0.exe`, `SoundBettrCore-0.1.0-portable.exe`, `SoundBettrCore-0.1.0-android.apk`, and `SHA256SUMS.txt` into `dist/` (and root copies `SoundBettrCore.exe` and `SoundBettrCore.apk` for instant access).
- [ ] **Step 2: Run full test suites (`cargo test`, `./gradlew testDebugUnitTest`, `taste-skill` / `hallmark` UI inspection) and verify all `.exe` and `.apk` binaries exist and pass integrity checks.**
- [ ] **Step 3: Update `C:\Users\J30N\dev-context` (`REPOS.md` and `projects/soundbettrcore.md`) per global rules.**
