# SoundBettrCore

Windows (`.exe`) and Android (`.apk`) acoustic correction, 7.1 surround, Dolby-style dynamics, and direct Bluetooth RFCOMM hardware controller for the **Anker Soundcore R50i NC** (`A3959`).

## Ready-to-Run Binaries

- **Windows Setup Installer (EqualizerAPO hook + Start Menu + Autostart):** `dist/SoundBettrCore-Setup-0.1.0.exe`
- **Windows Portable Executable:** `dist/SoundBettrCore-0.1.0-portable.exe` (also at repo root: `SoundBettrCore.exe`)
- **Android APK (Android 10+):** `dist/SoundBettrCore-0.1.0-android.apk` (also at repo root: `SoundBettrCore.apk`)
- **Checksums:** `dist/SHA256SUMS.txt`

## Features

1. **5-Stage DSP Pipeline**
   - **Stage 1 (R50i NC Harman/AutoEq Target Correction):** 10-band parametric biquad cascade cutting the stock `62–250 Hz` upper-bass bloat (`-4.2 dB` at `125 Hz`, `-3.4 dB` at `250 Hz`) and restoring `2–4 kHz` vocal clarity (`+3.2 dB` at `2 kHz`, `+3.8 dB` at `4 kHz`).
   - **Stage 2 (Harmonic Sub-Bass Exciter & Vocal Clarity Enhancer):** Odd/even harmonic sub-bass saturation (`30–90 Hz`) and `2.5–6 kHz` dynamic presence shelf.
   - **Stage 3 (3D Binaural 7.1 Virtual Surround & Crossfeed):** 8-channel (`L, R, C, LFE, LS, RS, LB, RB`) ITD/ILD + pinna notch spatializer with configurable stereo crossfeed (`650 Hz`, `-4.5 dB`).
   - **Stage 4 (Room / Hall Acoustic Ambience):** 4-comb + 2-allpass Schroeder/Freeverb stereo room simulator (`Studio`, `Small Room`, `Cinema`, `Concert Hall`).
   - **Stage 5 (Dolby-Style 3-Band Dynamics & Brickwall Limiter):** Linkwitz-Riley 4th-order crossovers at `200 Hz` and `2500 Hz`, upward/downward loudness leveler, and `-0.3 dBFS` brickwall peak limiter.
2. **Direct R50i NC (`A3959`) Bluetooth RFCOMM Control**
   - **ANC / Transparency / Normal:** Manual `1–5`, Adaptive ANC, Multi-Scene (`Transport`, `Outdoor`, `Indoor`), Vocal Mode, and Wind Noise Suppression (`06 81`).
   - **8-Band Hardware EEPROM EQ + DRC Matrix:** Flashes custom 8-band EQ (`100 Hz` to `12.8 kHz`) and exact `A3959` Dynamic Range Compression (`02 83`) + BassUp directly into the earbuds.
   - **Onboard 3D Surround & Gaming Mode:** Toggles hardware 3D surround (`02 86`) and low-latency gaming mode (`01 87`).
   - **Touch Control Remapping:** Remaps L/R single-tap, double-tap, triple-tap, and long-press actions (`04 81`).
   - **Live L/R Battery & Firmware Telemetry:** Parses `01 01` state frames (`0–100%` L/R battery, charging flags, firmware version, serial number).
3. **Built-In 7.1 Test Bench & Local Audio Player**
   - Synthesizes real-time 7.1 speaker orbit, sub-bass sweep (`25–120 Hz`), vocal presence pulse, and pink noise, or plays local audio files (`WAV`, `MP3`, `FLAC`, `OGG`) with instant `A/B Bypass`.

## Building from Source

Run the unified build script in PowerShell:

```powershell
pwsh -File scripts/build-release.ps1
```
