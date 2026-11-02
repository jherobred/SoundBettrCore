# SoundBettrCore v2: Design Spec

**Date:** 2026-11-02
**Replaces:** `2026-11-02-soundbettrcore-design.md` (v0.1, single device)
**Platforms:** Windows 10/11 (Setup `.exe`, portable `.exe`) and Android 10+ (`.apk`)
**Mockup:** [`assets/2026-11-02-v2-mockup.html`](assets/2026-11-02-v2-mockup.html), open it in a browser. Every screen, both themes, phone and Windows layouts.

## 1. Goal

v0.1 tunes one pair of earbuds (Soundcore R50i NC). v2 tunes every audio output you use, on PC and phone, and remembers each one.

When an output connects, the app recognizes it, loads its own sound profile, shows the connection on screen, and says it out loud ("Connected to Jhems's R50i NC. Harman fix profile on. Battery 78 percent."). The app gives you the tools to find the right sound for each device: EQ, balance, AutoEq correction, a hearing test, a balance check and a blind A/B.

### Success criteria

1. Plugging in or connecting any output (Bluetooth, USB, headphone jack, built-in speakers) switches the system sound to that output's saved profile within 2 seconds, with no manual step.
2. A connection is shown only when it is real. The app never shows a connected or "simulated" state it has not confirmed (v0.1 did).
3. The spoken announcement uses the bundled Kokoro voice, works offline, and starts within 1 second for a device heard before.
4. Every control in the UI changes the sound you hear in other apps (Spotify, YouTube, games), or is clearly labeled as in-app only. v2 has no in-app-only effects.
5. R50i NC hardware controls only appear for an R50i NC (or P30i) that answered a state request, and each command shows whether the earbuds confirmed it.

## 2. Decisions

| Code | Decision |
|---|---|
| D1 | One Tauri 2 app for Windows and Android. One web UI (React, Vite, Tailwind, `motion`, zustand, same stack as KiwiConvert), one Rust core, and a Kotlin Tauri plugin for Android-only APIs. The Compose app in `android/` is retired. |
| D2 | Profiles are per device. Device identity rules are in §4.2. |
| D3 | An unknown device plays flat and gets a setup card: name it, pick its color, match it in AutoEq. |
| D4 | Connection feedback: an on-screen state machine (§5) plus a spoken announcement from an editable template, `Connected to {device}. {profile} profile on. {battery}`, on by default per device. |
| D5 | Voice: Kokoro 82M (Apache 2.0) through sherpa-onnx (Apache 2.0), bundled in both apps. No Kitten, no system voices. 8 English voices are offered, Heart is the default. |
| D6 | Tuning tools: 10-band EQ, preamp, L/R balance, AutoEq match, hearing test, balance check, blind A/B. |
| D7 | AutoEq data is bundled as a compressed index. An optional "Update database" button in Settings is the app's only network call, and it is off until pressed. |
| D8 | Effects: Windows virtual surround is real HRIR convolution through EqualizerAPO. Android uses the system spatializer (Android 13+) or the Virtualizer effect where the phone allows it. Crossfeed stays on Windows. Room reverb, sub-bass exciter and the 7.1 test bench are cut. |
| D9 | Look: light, pale per-device color tiles (from mockup C) with the connect pulse that turns into the device's color (from mockup A). Taste "pastel-consumer" app style, Manrope font. Light, Dark and System themes. |
| D10 | Navigation: bottom bar with labels on phone, icon rail with hover labels on Windows. Five destinations: Now, Devices, Tune, Tests, Settings. |
| D11 | Motion is an in-app setting, on by default, because Jhems's PC reports reduced motion. Turning it off stops every animation and keeps every state readable. |
| D12 | R50i NC (A3959) hardware control stays, rebuilt on a persistent RFCOMM session that reads replies. |

## 3. Scope

**In:** everything in §4 to §9.

**Out:**
- Per-app EQ (different sound for Spotify and Discord).
- Cloud sync of profiles between PC and phone. Profiles export and import as JSON instead.
- Hardware control for Soundcore models other than A3959 (R50i NC, P30i).
- Telling two wired headsets apart automatically on Android. The OS reports them identically (§4.2).
- iOS and macOS.

## 4. Architecture

### 4.1 Layout

```
app/                    Tauri 2 project (replaces desktop/ and android/)
  src/                  React UI
  src-tauri/
    src/
      main.rs           commands, events, tray (Windows)
      device.rs         device identity and the connection state machine
      store.rs          devices and profiles, JSON in the app data folder
      eq.rs             biquad design, response curve, preamp
      apo.rs            EqualizerAPO config writer (Windows)
      surround.rs       HRIR convolution config for EqualizerAPO (Windows)
      autoeq.rs         bundled index, search, parse ParametricEQ
      voice.rs          sherpa-onnx Kokoro, announcement render and cache
      a3959.rs          kept from v0.1: packets, checksum, DRC matrix, state parse
      rfcomm.rs         persistent RFCOMM session (Windows)
      tests_audio.rs    hearing test staircase, A/B round state
    resources/
      kokoro/           model and voices (about 86 MB)
      autoeq/           compressed index
      hrir/             HRIR set for surround
    gen/android/        Tauri Android project
  plugins/android-audio/  Kotlin plugin: device callback, DynamicsProcessing, spatializer, RFCOMM
installer/              kept: Windows Setup, now embeds the Tauri build
```

Kept from v0.1 as-is: `a3959.rs` (packet builders, checksum, DRC matrix, state parser, with its tests), `installer/`, and `scripts/build-release.ps1` (updated for the new paths). Rewritten: `apo.rs` (per-device output), Bluetooth code. Deleted: `dsp.rs`, `presets.rs`, the WebAudio UI, and the Compose app.

### 4.2 Device identity

| Output | Windows identity | Android identity |
|---|---|---|
| Bluetooth | Bluetooth address, read from the audio endpoint's container ID | Bluetooth MAC from `AudioDeviceInfo.getAddress()` |
| USB audio | Endpoint ID (stable per device and port) | `TYPE_USB_HEADSET` + `productName` |
| Headphone jack | Endpoint ID | One shared "wired" identity. The setup card asks which wired set this is when more than one is saved. |
| Built-in | Endpoint ID | One "phone speaker" identity |

Windows watches `IMMNotificationClient` for default endpoint changes. Android uses `AudioManager.registerAudioDeviceCallback` and follows the current media route.

A device is an R50i NC only when its name matches `R50i`, `P30i` or `A3959` **and** it answers the A3959 state request (`01 01`). A name containing "soundcore" is no longer enough.

### 4.3 Applying a profile

- **Windows:** `apo.rs` writes one block per device to `C:\ProgramData\SoundBettrCore\live_apo.txt` under an EqualizerAPO `Device:` selector, so EqualizerAPO switches the filters itself when the output changes. Contents: `Preamp`, ten `Filter PK` lines, balance as per-channel gain, `Copy` crossfeed, and `Convolution` with the HRIR file when surround is on. Without EqualizerAPO, Tune shows "System sound needs EqualizerAPO" with an install link, and the announcement, devices and R50i controls still work.
- **Android:** the plugin's foreground service keeps v0.1's `DynamicsProcessing` on session 0: pre-EQ with 10 bands, a limiter at -0.3 dBFS, and balance as per-channel input gain. If session 0 is refused, Tune says so, and the R50i hardware EQ is offered as the fallback for that device.

### 4.4 Data

```ts
Device  { id, kind: 'bluetooth'|'usb'|'wired'|'builtin', name, hue: 'cobalt'|'amber'|'rose'|'jade',
          announce: boolean, profileId, lastSeen, isNew, hardware?: 'a3959' }
Profile { id, name, source: 'flat'|'autoeq'|'hearing'|'ab'|'custom', autoeqModel?,
          gains: number[10], preamp, balance, surround, crossfeed, hearing?: { left: number[], right: number[] } }
Settings { theme: 'light'|'dark'|'system', motion: boolean, voice: string, template: string }
```

The 10 bands are 31, 62, 125, 250, 500 Hz and 1, 2, 4, 8, 16 kHz. The preamp is -max(boost) unless the user sets it. Balance is in dB toward one side, from -6 to +6. Everything is saved as JSON in the app data folder, and nothing leaves the device.

## 5. Connection feedback

States: `searching` → `recognized` | `new` | `error`.

1. The OS reports an output change. The UI shows `searching`: rings pulse around the device mark (they run only while the app is actually searching).
2. Identity resolves (§4.2). A known device loads its profile and applies it (§4.3). An unknown device gets a flat profile and the `new` state with a setup card.
3. On `recognized`, the rings snap in, the card takes the device's color, the name rises in, battery bars fill (when the device reports battery), and the EQ curve draws.
4. If announcements are on for the device, `voice.rs` fills the template and plays it through the new output. Rendered audio is cached by `(device, profile, voice, template)` text hash, so a repeat connection plays immediately. Tapping the device card replays it.
5. `error` covers a failed profile apply or a lost device, with the cause and the next action shown ("EqualizerAPO is not installed. Install it to tune system sound.").

## 6. Tools

- **Equalizer:** 10 vertical sliders from -12 to +12 dB in 0.5 dB steps, a live curve, "Reset to flat", and the automatic preamp note. Editing renames the profile "(edited)", or "Custom" when it was Flat.
- **AutoEq match:** search the bundled index by model name, then Apply. AutoEq's ParametricEQ curves are fitted to the 10 bands, with the original filters kept when they don't fit (Windows APO can take any `Filter PK`). Empty state: "No AutoEq entry matches. Check the model name, or tune by hand above."
- **Hearing test:** per ear at 250 Hz, 500 Hz, 1, 2, 4 and 8 kHz, using a down 10 dB / up 5 dB staircase (Hughson-Westlake style), with "I heard it" and "I did not". Result: a per-ear threshold chart. "Apply" boosts each band by a third of its threshold above the best band, capped at +6 dB, and sets balance from the left/right difference. The result is relative (consumer devices have no calibration) and the screen says so.
- **Balance check:** a 1 kHz tone at the current balance. "Leans left" or "Leans right" moves it 0.5 dB and replays it, and "Centered" saves.
- **Blind A/B:** pick two profiles, then 6 rounds. Each round randomly maps X and Y to the two profiles and switches the live system sound. The user picks a favorite each round, the result is revealed at the end, and the winner can be applied to the device.
- **Surround and crossfeed:** switches in Tune. Windows uses the HRIR convolution (D8). Android shows the spatializer state, and opens the system setting when the app can't turn it on itself. Crossfeed is Windows only, and Android hides its switch.
- **Mockup note:** the mockup's hearing test uses 4 frequencies and a simpler descending method to keep the demo short. This section is the target.
- **R50i NC hardware:** noise control (Normal, Aware, ANC), battery per earbud, hardware EQ, gaming mode, touch remapping. Each command shows sent, confirmed or failed from the earbuds' reply.

## 7. UI

- **Screens:** Now (device card, sound card, noise control for the R50i, remembered devices), Devices (list plus a detail panel to rename, pick a color, toggle the announcement, Tune, and Forget with confirmation), Tune (§6), Tests (§6), Settings (appearance, motion, announcement voice with Play previews, announcement wording).
- **Tokens:** the dealt pastel-consumer set from the mockup. Ground `#f3f5f7`, surface `#fcfdff`, text `#1b1d1f`, muted `#5c6064`. Device hues: cobalt, amber, rose and jade, at least 60° apart, each with soft, ink and mark values for light and dark. Card radius 20, inner radius 10. No shadows on resting elements, no gradients, no glass.
- **Layout:** one column on phone. On Windows, two columns from 700px wide. Settings uses a label column beside a panel.
- **Motion:** only on real state changes: the connect sequence, the nav indicator sliding (spring), the segmented controls, and the view enter (8px rise, 320ms). Transforms and opacity only.
- **Accessibility:** 44px minimum targets, visible focus, every control has a name, keyboard reachable, WCAG AA contrast in both themes. The mockup passes Hallmark sloplint and taste lint and inspect with 0 FAIL.

## 8. Errors and edge cases

| Case | Behavior |
|---|---|
| EqualizerAPO missing (Windows) | Tune shows the install prompt. Everything else works. |
| Session 0 refused (Android) | Tune says so. The R50i hardware EQ is offered for R50i devices. |
| Bluetooth permission denied | The R50i panel shows "Allow Nearby devices to control the earbuds" with a button to the permission prompt. |
| R50i does not answer | Hardware controls stay hidden. The device still works as a normal output with its profile. |
| Voice fails to load | The announcement is skipped, the on-screen state still shows, and Settings shows the error. |
| Device forgotten while connected | Not allowed. The Forget button is disabled with "Disconnect it first". |
| Two wired sets on Android | The setup card asks which one this is, and the choice is remembered until unplug. |
| AutoEq update fails | The bundled index stays. Settings shows the failure and the last update date. |

## 9. Testing

- **Rust unit tests:** device identity mapping, the state machine transitions, biquad coefficients against reference values, response curve, preamp rule, APO render for a two-device config, AutoEq parse and the 10-band fit, hearing staircase and the apply rule, A/B round mapping, template fill, and the kept A3959 tests.
- **Kotlin unit tests:** device callback to identity, DynamicsProcessing band mapping.
- **UI tests (vitest):** the connection state machine rendering, device rename and color, EQ slider to curve, settings persistence, theme switch.
- **Manual checklist on hardware** (R50i NC, a wired headset, laptop speakers, a phone): connect and disconnect each one, hear the announcement, confirm the profile change is audible in Spotify, run the hearing test and the A/B, and confirm R50i ANC changes with a reply.

## 10. Sizes

Kokoro adds about 86 MB. Expected sizes: Windows Setup about 95 MB, portable about 95 MB, APK about 110 MB.

## 11. Risks and open items

| Code | Risk | Plan |
|---|---|---|
| R1 | HRIR licensing. SADIE II is listed as Apache 2.0 with a citation requirement. A noncommercial set (CC BY-NC-SA) cannot be used. | Confirm the license file in the downloaded SADIE II package before bundling, and credit it in the README. |
| R2 | sherpa-onnx inside Tauri on both platforms. The official Rust crate's Android support is unconfirmed. | Spike first: Rust crate on Windows, Kotlin AAR in the Android plugin, both speaking one line. |
| R3 | Android OEMs restricting `DynamicsProcessing` on session 0. | Known from v0.1. Use the R50i hardware EQ fallback and say so in the UI. |
| R4 | AutoEq output license. The project is MIT, but the license of the generated `results/` files is not stated separately. | Check the upstream LICENSE before bundling, and credit AutoEq in the app and README. |
| R5 | Bluetooth address from a Windows audio endpoint. The container ID mapping can differ between drivers. | Fall back to the endpoint ID as identity, matched by name to the Bluetooth device list. |
| R6 | Announcement playing over the earbuds' own "connected" prompt. | Delay the announcement about 1.5 s after the route is active. Tune it on the R50i. |

Sources checked on 2026-11-02: [AutoEq license (ecosyste.ms)](https://awesome.ecosyste.ms/projects/github.com%2Fjaakkopasanen%2FAutoEq), [SADIE II (Zenodo)](https://zenodo.org/records/12092466), [SADIE database page](https://www.york.ac.uk/sadie-project/database.html), [sherpa-onnx Rust API](https://k2-fsa.github.io/sherpa/onnx/rust-api/index.html), [kokoro-js](https://www.npmjs.com/package/kokoro-js), [Picovoice on-device TTS benchmark](https://picovoice.ai/blog/on-device-tts/).
