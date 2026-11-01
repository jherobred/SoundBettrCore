# SoundBettrCore — Design Specification

**Date:** 2026-11-02  
**Target Hardware:** Anker Soundcore R50i NC (Model `A3959`, identical hardware/protocol to Soundcore P30i)  
**Target Platforms:** Windows 10/11 (`.exe` Setup + Portable) and Android 10+ (`.apk`)

---

## 1. Intent and Why

The Soundcore R50i NC (`A3959`) has a budget V-shaped acoustic tuning: heavy mid-bass bloat around 80 to 150 Hz, scooped upper-mids around 2 to 4 kHz, and limited soundstage width. Furthermore, its official companion app is mobile-only, lacks true spatial/Dolby-style host DSP, and does not provide a unified desktop + mobile audio studio.

**SoundBettrCore** solves both problems in one cohesive desktop (`.exe`) and mobile (`.apk`) studio:
1. Fixes the R50i NC's acoustic frequency response toward a clean, detailed Harman/AutoEq target while adding 3D Binaural Surround (Virtual 7.1), Dolby-style Multiband Compression & Loudness Leveling, Vocal/Dialog Clarity, Harmonic Sub-Bass Excitation, and Room/Hall Reverb + Crossfeed.
2. Communicates directly with the R50i NC (`A3959`) over Bluetooth RFCOMM to control onboard ANC/Transparency/Normal modes (Adaptive, Manual 1-5, Multi-Scene, Wind Noise Suppression), flash custom 8-band + DRC curves directly to the earbud EEPROM, toggle onboard 3D Surround and Low-Latency Gaming Mode, remap touch controls, and monitor Left/Right earbud + charging case battery levels.
3. Always produces ready-to-run build artifacts in `dist/`:
   - `SoundBettrCore-Setup-<version>.exe` (Windows installer with zero-latency APO integration)
   - `SoundBettrCore-<version>-portable.exe` (Windows standalone portable binary)
   - `SoundBettrCore-<version>-android.apk` (Android arm64/universal APK)

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **G1 — Dual-Layer Audio Enhancement (Host DSP + Earbud Hardware DSP):** Process all system audio on Windows and Android in real time, plus offer a built-in media/test player with instant A/B bypass and spatial/sub-bass test generators.
- **G2 — Full Soundcore R50i NC (`A3959`) Bluetooth RFCOMM Control:** Native Bluetooth RFCOMM client on both Windows and Android implementing the verified `A3959` packet protocol (`08:ee:00:00:00` outbound, `09:ff:00:00:01` inbound, `sum8` checksum).
- **G3 — Simple & Pro Modes in a Dark Audiophile Studio UI:**
  - **Simple Mode:** One-tap presets (*R50i NC Harman Fix*, *Dolby Cinema 7.1*, *Music Clarity*, *BassUp Punch*, *Gaming Footsteps*, *Podcast/Vocal*) + 4 tactile rotary macro dials (*Surround Width*, *Sub-Bass*, *Vocal Clarity*, *Room Space*) + R50i NC ANC/Battery bar.
  - **Pro Rack Mode:** 10-band Parametric/Graphic EQ (`31 Hz` to `16 kHz`) + separate 8-band Hardware Chip EQ flasher (`100 Hz` to `12.8 kHz`), 3-band Dolby-style dynamic compressor/limiter controls, binaural HRTF 7.1 virtual speaker radar, crossfeed/reverb controls, and live 60fps FFT spectrum analyzer.
- **G4 — Always-Available `.exe` and `.apk` Artifacts:** Reproducible build script (`scripts/build-release.ps1`) that compiles and outputs the Windows Setup `.exe`, Windows Portable `.exe`, and Android `.apk` into `dist/`.

### Non-Goals
- Cloud accounts, telemetry, subscriptions, or remote servers. Everything runs 100% locally and offline.
- Supporting proprietary Dolby Labs licensed bitstreams (AC-3/E-AC-3 decoding). "Dolby-style" refers to our clean-room DSP implementation of binaural virtual surround, multiband dynamic loudness leveling, dialog enhancement, and harmonic bass excitation.
- Supporting every historical Soundcore headphone model in v1 (protocol is tailored to `A3959` / R50i NC / P30i, with clean fallback if another Bluetooth headphone is used for host DSP).

### Anti-Goals
- Clipping or muddy bass when multiple enhancements are stacked. The DSP chain must end with a brickwall lookahead soft-limiter (`-0.3 dBFS` ceiling) and automatic pre-amp gain compensation.
- Requiring root on Android or forcing manual registry/config edits on Windows.

---

## 3. Technical Constraints

1. **Windows System Audio Interception (`F4`):**
   - **APO Mode (Setup `.exe`, Builder Call):** Windows Audio Processing Objects run inside `audiodg.exe`. Rather than injecting an unsigned third-party C++ `.dll` into `audiodg.exe`, SoundBettrCore writes native EqualizerAPO configuration directives (`Preamp`, 10-band biquad `Filter` lines, vocal/exciter shelf filters, and `Copy`/`Delay`/`Channel` matrices for 7.1 ITD/ILD surround and crossfeed) to `C:\ProgramData\SoundBettrCore\live_apo.txt` (and hooks `C:\Program Files\EqualizerAPO\config\config.txt` when EqualizerAPO is installed).
   - **WASAPI Loopback + Render / Built-in Player Mode (Setup & Portable `.exe`):** The Rust audio engine captures system loopback audio (`wasapi` shared loopback) for real-time 60fps FFT spectrum visualization and processes audio through the full 7-stage non-linear Rust DSP pipeline ($\tanh$ harmonic sub-bass exciter, dynamic vocal enhancer, 7.1 HRTF spatializer, Schroeder reverb, 3-band compressor, brickwall limiter) into any selected secondary/virtual output device, and runs the built-in local audio file player & spatial 7.1 / sub-bass test synthesizer directly through the primary output.
2. **Android System Audio Interception (`F3`):**
   - Unrooted Android 10+ allows attaching `android.media.audiofx.DynamicsProcessing` (pre-EQ, multi-band compressor, post-EQ, limiter) and `Virtualizer` / `BassBoost` to audio session `0` (global output mix) and dynamically to active app `AudioSession` IDs broadcast via `ACTION_OPEN_AUDIO_EFFECT_CONTROL_SESSION`.
   - A foreground service (`SoundBettrAudioService`, `foregroundServiceType="mediaPlayback|connectedDevice"`) keeps the DSP chain alive, listens for Bluetooth `ACL_CONNECTED` events for the R50i NC, and exposes a Quick Settings Tile and notification preset switcher.
   - The built-in Android test/media player runs the complete custom Kotlin/PCM float DSP pipeline (biquad 10-band EQ, HRTF 7.1 binaural spatializer, multiband compressor, harmonic exciter, Freeverb room model, limiter) so every effect can be auditioned at full precision.
3. **Soundcore R50i NC (`A3959`) Bluetooth RFCOMM Protocol (`F2`):**
   - **Service Discovery:** Connect via RFCOMM to the Soundcore vendor UUID matching mask `0cf12d31-fac3-4553-bd80-d6832e700000` (`ffffffff-ffff-ffff-ffff-fffffff00000`) or standard SPP UUID `00001101-0000-1000-8000-00805f9b34fb`.
   - **Packet Framing:**
     - Outbound header: `08 EE 00 00 00` + `Command (2 bytes)` + `Total Packet Length (u16 LE, including 1-byte checksum)` + `Payload` + `Checksum (sum8 = sum of all preceding bytes mod 256)`.
     - Inbound header: `09 FF 00 00 01` + `Command (2 bytes)` + `Total Packet Length (u16 LE)` + `Payload` + `Checksum (sum8)`.
   - **Commands for `A3959`:**
     - `01 01` (**Request State**): Empty payload. Returns 91-byte `A3959StateUpdatePacket` body with exact offsets:
       - `0..2` (`2B`): `tws_status` (`[host_device, tws_connected_bool]`).
       - `2..6` (`4B`): `dual_battery` (`[left_level (0..=10), right_level (0..=10), left_charging (0/1/255), right_charging (0/1/255)]`). *Builder Call:* The `A3959` firmware does not transmit a separate charging-case battery percentage in the 91-byte state packet; the UI displays exact Left/Right earbud battery (`0–100%` in `10%` steps) + Left/Right charging indicators, and marks Case status as `In Case / Charging` when `charging == 1`.
       - `6..16` (`10B`): `dual_firmware_version` (5-byte ASCII Left e.g. `"01.64"`, 5-byte ASCII Right e.g. `"01.64"`).
       - `16..32` (`16B`): `serial_number` (16-byte ASCII).
       - `32..44` (`12B`): `equalizer_configuration` (`preset_id: u16 LE` + 10 band bytes where `120 = 0.0 dB`).
       - `44..55` (`11B`): Reserved (`10B` + `1B`).
       - `55..64` (`9B`): `button_configuration` — byte `55` is count/header (`10`), followed by 8 action bytes in order `[LeftSinglePress, RightSinglePress, LeftDoublePress, RightDoublePress, LeftTriplePress, RightTriplePress, LeftLongPress, RightLongPress]`. Low nibble (`byte & 0x0F`) is `Action` (`0=VolumeUp, 1=VolumeDown, 2=PreviousSong, 3=NextSong, 4=AmbientSoundMode, 5=VoiceAssistant, 6=PlayPause, 9=GameMode`); `0xFF` means disabled. When `tws_connected == 0`, Single and Triple press report disabled.
       - `64..65` (`1B`): `ambient_sound_mode_cycle` bitmask (`bit 0 = NoiseCanceling, bit 1 = Transparency, bit 2 = Normal`).
       - `65..72` (`7B`): `sound_modes` (`[ambient_mode, (adaptive_level << 4) | manual_level, ambient_mode_dup, anc_mode, wind_noise, adaptive_sensitivity, multi_scene_anc]`).
       - `72..73` (`1B`): Reserved (`0`).
       - `73..74` (`1B`): `touch_tone` (`0/1`).
       - `74..75` (`1B`): `dual_connections_enabled` (`0/1`).
       - `75..76` (`1B`): `surround_sound` (`0/1`).
       - `76..78` (`2B`): `auto_power_off` (`[enabled_bool, duration_index (0=10m, 1=20m, 2=30m, 3=60m)]`).
       - `78..79` (`1B`): `low_battery_prompt` (`0/1`).
       - `79..80` (`1B`): `gaming_mode` (`0/1`, active on firmware `>= 01.60`).
       - `80..91` (`11B`–`12B`): Trailing reserved bytes.
     - `06 81` (**Set Sound Modes**): 7-byte payload `[ambient_mode, (adaptive_level << 4) | manual_level, ambient_mode, anc_mode, wind_noise_byte, adaptive_sensitivity, multi_scene_mode]`.
       - `ambient_mode`: `0 = NoiseCanceling`, `1 = Transparency`, `2 = Normal`.
       - `anc_mode`: `0 = Manual (1..=5)`, `1 = Adaptive`, `2 = MultiScene (0=Transport, 1=Outdoor, 2=Indoor)`.
     - `02 83` (**Set Equalizer with DRC**): 22-byte payload:
       - Bytes `0..2`: Preset ID (`0xFEFE` for custom, `0x0000` Soundcore Signature, `0x0001` Acoustic, `0x0002` Bass Booster / BassUp, `0x0003` Bass Reducer, `0x0004` Classical, `0x0005` Podcast, `0x0006` Dance, `0x0007` Deep, `0x0008` Electronic, `0x0009` Flat, `0x000A` Hip-Hop, `0x000B` Jazz, `0x000C` Latin, `0x000D` Lounge, `0x000E` Piano, `0x000F` Pop, `0x0010` RnB, `0x0011` Rock, `0x0012` Small Speakers, `0x0013` Spoken Word, `0x0014` Treble Booster, `0x0015` Treble Reducer).
       - Bytes `2..12`: 10 band values (`u8`, where `120` = `0.0 dB`, `0` = `-12.0 dB`, `240` = `+12.0 dB`; first 8 bands active at `100, 200, 400, 800, 1600, 3200, 6400, 12800 Hz`, bands 9-10 fixed at `120, 120`).
       - Bytes `12..22`: 10 DRC-compensated band bytes (`u8 = (drc_tenths + 120)` for bands `0..8`, followed by fixed `[120, 0]` for bands 9-10). For input adjustments $a_0 \dots a_7$ in dB ($a_i = \frac{\text{byte}_i - 120}{10.0}$), with $S = 0.85$, $L = 0.95$, $\text{low\_sub} = a_2 \times 0.81 \times S$, $\text{high\_sub} = a_5 \times 0.81 \times S$, and $8 \times 8$ matrix $C$:
         - Row 0: $[1.26, -0.71S, 0.177, -0.0494, 0.0345, -0.0197, 0.0075, -0.00217]$
         - Row 1: $[-0.71S, 1.73L, 0, 0.204, -0.068, 0.045, -0.0235, 0.0075]$ (with $-\text{low\_sub}$ at $j=2$)
         - Row 2: $[0.177, -0.81S, 1.73L, -0.81S, 0.208, -0.07, 0.045, -0.0197]$
         - Row 3: $[-0.0494, 0.204, 0, 1.73L, -0.82S, 0.208, -0.068, 0.0345]$ (with $-\text{low\_sub}$ at $j=2$)
         - Row 4: $[0.0345, -0.068, 0.208, -0.82S, 1.73L, 0, 0.204, -0.0494]$ (with $-\text{high\_sub}$ at $j=5$)
         - Row 5: $[-0.0197, 0.045, -0.07, 0.208, -0.81S, 1.73L, -0.81S, 0.177]$
         - Row 6: $[0.0075, -0.0235, 0.045, -0.068, 0.204, 0, 1.83L, -0.71S]$ (with $-\text{high\_sub}$ at $j=5$)
         - Row 7: $[-0.00217, 0.0075, -0.0197, 0.0345, -0.0494, 0.177, -0.71S, 1.5]$
         - Each output $\text{drc\_tenths}_r = \text{round}\left(\left(\sum_{j=0}^{7} \text{term}_{r,j}\right) / 10.0 \times 10.0\right)$.
     - `02 86` (**Set Onboard Surround Sound**): 1-byte payload `[0]` or `[1]`.
     - `01 87` (**Set Gaming Mode**): 1-byte payload `[0]` or `[1]`.
     - `01 83` (**Set Touch Tone**): 1-byte payload `[0]` or `[1]`.
     - `04 81` (**Set Button Action**): 3-byte payload `[earbud_side (0=Left, 1=Right), button_id (2=Single, 0=Double, 5=Triple, 1=Long), action_byte]`.
       - `action_byte`: `0xF0 | action_id` when enabled, or `0xFF` when disabled (`0=VolumeUp, 1=VolumeDown, 2=PreviousSong, 3=NextSong, 4=AmbientSoundMode, 5=VoiceAssistant, 6=PlayPause, 9=GameMode`).

---

## 4. Approaches Considered

1. **`O1` (Selected) — Tauri 2 + Rust + React on Windows (`.exe`) and Native Kotlin + Jetpack Compose on Android (`.apk`)**
   - **Pros:** Matches existing build infrastructure (`LodiSave`, `KiwiConvert`, `TubbyKeys`). Smallest binaries, direct WinRT Bluetooth RFCOMM & WASAPI access in Rust, and direct `DynamicsProcessing` + `BluetoothSocket` access in Kotlin without WebView overhead on mobile.
   - **Cons:** UI components are written in React (Windows) and Compose (Android), sharing the same design tokens, layout structure, preset curves, and DSP math.
2. **`O2` — Shared React UI on Tauri 2 (`.exe`) and Capacitor (`.apk`)**
   - **Pros:** Single React UI codebase.
   - **Cons:** Higher memory usage on Android when running background audio services and 60fps canvas visualizers inside a WebView.
3. **`O3` — Flutter for Windows and Android**
   - **Pros:** Single Dart UI.
   - **Cons:** Heavier Windows runtime bundle and extra FFI glue for WinRT RFCOMM + WASAPI loopback.

---

## 5. Decided Architecture & DSP Pipeline (`D1`–`D9`)

### 5.1 Host DSP Processing Chain (64-bit / 32-bit Float Stereo @ 48 kHz)

Audio flows through 7 stages in order, with an instant **A/B Bypass** toggle that crossfades in 15 ms (click-free):

1. **Pre-Amp & Auto-Gain Stage:** Applies user pre-amp (`-12` to `+12 dB`) and automatic headroom compensation based on active EQ/exciter boosts.
2. **R50i NC Corrective AutoEq / Harman Curve + 10-Band Parametric EQ:**
   - Default **R50i NC Harman Fix** profile compensates for the `A3959` driver response:
     - `31 Hz`: `+1.5 dB` (sub-bass extension)
     - `62 Hz`: `-2.0 dB` (trim upper sub-bass bloom)
     - `125 Hz`: `-4.5 dB` (cut R50i NC mid-bass bloat/mud)
     - `250 Hz`: `-2.5 dB` (clean lower-mid warmth bleed)
     - `500 Hz`: `0.0 dB`
     - `1 kHz`: `+1.0 dB`
     - `2 kHz`: `+3.0 dB` (restore recessed upper-mid vocal presence)
     - `4 kHz`: `+3.5 dB` (ear-gain clarity target)
     - `8 kHz`: `+1.5 dB` (crisp detail without sibilance)
     - `16 kHz`: `+2.5 dB` (air & sparkle)
   - Implemented as cascaded RBJ Audio EQ Cookbook biquad filters (`Peaking`, `LowShelf`, `HighShelf` with adjustable `Q` in Pro Mode).
3. **Harmonic Sub-Bass Exciter ("BassUp Studio"):**
   - 4th-order Linkwitz-Riley bandpass (`45–110 Hz`) feeds an asymmetric soft-saturator ($\tanh$ waveshaper) generating 2nd and 3rd harmonics (`90–250 Hz`), mixed back through a low-pass at `180 Hz`. Gives perceived deep sub-bass slam on the 10mm R50i NC drivers without excursion clipping.
4. **Dialog / Vocal Clarity Enhancer:**
   - Dynamic presence shelf (`1.5–4.5 kHz`) + mid-channel extractor that gently lifts center-panned vocals/dialogue relative to wide stereo reverb.
5. **3D Binaural Surround / Virtual 7.1 Spatializer:**
   - Decomposes stereo into Mid ($M = \frac{L+R}{2}$) and Side ($S = \frac{L-R}{2}$) plus virtual Front L/R, Side L/R, and Rear L/R phantom speakers using Interaural Time Difference (ITD Haas delay lines `0.22–0.65 ms`), Interaural Level Difference (ILD head-shadow shelf), and pinna notch/peak cues.
   - Includes adjustable **Binaural Crossfeed** (`15–35%` low-passed opposite-channel blend with `300 µs` delay) to move the soundstage out of the head and eliminate ear fatigue.
6. **Room / Concert Hall Acoustic Virtualizer:**
   - Stereo Schroeder / Freeverb network (8 parallel feedback comb filters + 4 series all-pass diffusers per channel) with configurable Room Size (*Studio Booth*, *Cinema Theater*, *Concert Hall*) and Wet/Dry mix.
7. **Dolby-Style 3-Band Compressor / Loudness Leveler & Brickwall Limiter:**
   - Crossover at `200 Hz` and `2.5 kHz` (Low / Mid / High) with upward + downward compression so quiet details in movies, games, and tracks stay audible without pumping, followed by a `-0.3 dBFS` lookahead soft-knee limiter.

### 5.2 Built-In Media & Spatial Test Player

Both `.exe` and `.apk` include an integrated player bar:
- Load and play local audio files (`.mp3`, `.wav`, `.flac`, `.ogg`, `.m4a`) through the live DSP chain.
- Built-in **Spatial & Acoustic Test Generator** (zero external files needed):
  - *7.1 Speaker Orbit Test* (band-limited pink noise + chime panning smoothly 360° around Front L -> Center -> Front R -> Side R -> Rear R -> Rear L -> Side L).
  - *Sub-Bass Sweep (`25–160 Hz`)* to hear the R50i NC mid-bass mud cleanup via A/B toggle.
  - *Vocal / Dialog Clarity Pulse* & *Pink Noise Reference*.

### 5.3 Presets & Import/Export

- **Built-in Factory Presets:**
  1. `R50i NC Harman Reference` (Default — balanced audiophile tuning + subtle stage)
  2. `Dolby Cinema 3D` (Wide 7.1 surround, dialog boost, loudness leveler, theater room)
  3. `Dolby Music Studio` (Clean Harman EQ, harmonic bass, studio crossfeed, gentle dynamics)
  4. `BassUp Ultra (Clean)` (Heavy sub-bass harmonic punch with 125 Hz mud cut)
  5. `FPS Gaming Footsteps` (Wide spatial radar, compressed dynamic range, 2–6 kHz footstep lift, R50i NC Game Mode ON)
  6. `Podcast & Late Night` (Dialog enhancer + strong loudness leveler + sub-bass roll-off)
- **Import / Export:**
  - Export/Import full SoundBettrCore presets as `.json`.
  - Import/Export standard EqualizerAPO / AutoEq `.txt` files (`Preamp:` and `Filter N: ON PK/LS/HS Fc ... Gain ... Q ...`).

### 5.4 Windows Application (`desktop/` + `installer/`)

- **Stack:** Tauri 2 + Rust (`cpal`, `windows` crate for WinRT Bluetooth RFCOMM & WASAPI loopback, `rustfft` for 60fps spectrum) + React 19 + TypeScript + Vite.
- **System Audio Modes:**
  - Automatically writes active EQ/spatial filter chain to EqualizerAPO config (`C:\Program Files\EqualizerAPO\config\soundbettrcore.txt` and registers `Include: soundbettrcore.txt` if EqualizerAPO is present) AND runs a live WASAPI loopback + output router in Rust so users can either process via APO (0 ms latency) or via the live Rust DSP stream.
- **System Tray:** Clicking tray icon toggles compact quick-switcher; closing window minimizes to tray while keeping DSP and Bluetooth auto-connect active.
- **Installer (`installer/`):** Self-contained Rust/Win32 Setup `.exe` (adapted from proven `LodiSave`/`KiwiConvert` pattern) that installs `SoundBettrCore.exe`, creates Start Menu & Desktop shortcuts, configures APO hook / autostart option, and registers uninstaller.

### 5.5 Android Application (`android/`)

- **Stack:** Kotlin + Jetpack Compose (Material 3 custom Dark Audiophile Studio theme), `minSdk 29` (Android 10), `targetSdk 35`.
- **Background Engine (`SoundBettrService`):**
  - Attaches `DynamicsProcessing` (10-band Pre-EQ, 3-band MBC, 10-band Post-EQ, Limiter) + `Virtualizer` to global output mix (`session 0`) and active media player sessions.
  - Runs custom PCM float DSP engine (`AudioTrack` + `MediaExtractor` / procedural synthesizer) for the built-in player and 360° 7.1 test suite.
  - Runs `R50iBluetoothManager` (`BluetoothAdapter` + `BluetoothSocket` RFCOMM) with auto-connect receiver on `BluetoothDevice.ACTION_ACL_CONNECTED` for devices matching `Soundcore R50i NC`, `Soundcore P30i`, or `A3959` (plus manual device picker).
  - Includes a **Quick Settings Tile** (`SoundBettrTileService`) to cycle presets or toggle DSP from the Android notification shade.

---

## 6. What Is Left to the Builder

- Exact UI spacing, SVG knob geometry, and canvas rendering optimizations for the 60fps FFT spectrum and 3D 7.1 soundstage radar.
- Internal Rust module organization under `desktop/src-tauri/src/` and Kotlin package organization under `android/app/src/main/java/com/jherobred/soundbettrcore/`.
- Unit and integration test suites verifying the DSP biquad math, `A3959` packet encoder/decoder + DRC matrix, EqualizerAPO `.txt` parser, and release build verification.
