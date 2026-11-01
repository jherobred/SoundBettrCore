(function () {
  "use strict";

  const HOST_FREQS = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
  const HW_FREQS = [100, 200, 400, 800, 1600, 3200, 6400, 12800];
  const HARMAN_GAINS = [1.5, -2.0, -4.5, -2.5, 0.0, 1.0, 3.0, 3.5, 1.5, 2.5];

  const BUTTON_SLOTS = [
    { key: "left_single", side: "left", press: "single", label: "Left Earbud, Single Tap" },
    { key: "right_single", side: "right", press: "single", label: "Right Earbud, Single Tap" },
    { key: "left_double", side: "left", press: "double", label: "Left Earbud, Double Tap" },
    { key: "right_double", side: "right", press: "double", label: "Right Earbud, Double Tap" },
    { key: "left_triple", side: "left", press: "triple", label: "Left Earbud, Triple Tap" },
    { key: "right_triple", side: "right", press: "triple", label: "Right Earbud, Triple Tap" },
    { key: "left_long", side: "left", press: "long", label: "Left Earbud, Long Press" },
    { key: "right_long", side: "right", press: "long", label: "Right Earbud, Long Press" },
  ];

  const BUTTON_ACTIONS = [
    { value: "volume_up", label: "Volume Up" },
    { value: "volume_down", label: "Volume Down" },
    { value: "previous_song", label: "Previous Track" },
    { value: "next_song", label: "Next Track" },
    { value: "ambient_sound_mode", label: "Cycle ANC / Transparency" },
    { value: "voice_assistant", label: "Voice Assistant" },
    { value: "play_pause", label: "Play / Pause" },
    { value: "game_mode", label: "Toggle Game Mode" },
    { value: "disabled", label: "Disabled" },
  ];

  function makeFallbackPresets() {
    function bandsFor(gains) {
      return HOST_FREQS.map(function (f, idx) {
        return {
          freq_hz: f,
          gain_db: gains[idx],
          q: idx === 0 || idx === 9 ? 0.707 : 1.41,
          kind: idx === 0 ? "low_shelf" : idx === 9 ? "high_shelf" : "peaking",
          enabled: true,
        };
      });
    }
    return [
      {
        id: "r50i_harman_reference",
        name: "R50i NC Harman Reference",
        subtitle: "AutoEq mid-bass mud cut + upper-mid vocal clarity & subtle 3D stage",
        bypass: false,
        auto_headroom: true,
        preamp_db: 0.0,
        harman_correction_enabled: true,
        bands: bandsFor(HARMAN_GAINS),
        surround_width: 0.35,
        crossfeed_amount: 0.22,
        sub_bass_exciter: 0.38,
        vocal_clarity: 0.42,
        room_space: 0.16,
        room_model: "studio_booth",
        dolby_leveler_amount: 0.45,
        limiter_ceiling_db: -0.3,
        hw_eq_tenths_db: [-25, -18, 5, 18, 32, 24, 10, 15],
      },
      {
        id: "dolby_cinema_3d",
        name: "Dolby Cinema 3D",
        subtitle: "Wide 7.1 virtual surround, theater room acoustics, dialog lift & loudness leveler",
        bypass: false,
        auto_headroom: true,
        preamp_db: 0.5,
        harman_correction_enabled: true,
        bands: bandsFor([3.5, 1.0, -3.5, -2.0, 0.5, 2.0, 3.8, 4.0, 2.0, 3.0]),
        surround_width: 0.78,
        crossfeed_amount: 0.18,
        sub_bass_exciter: 0.62,
        vocal_clarity: 0.68,
        room_space: 0.42,
        room_model: "cinema_theater",
        dolby_leveler_amount: 0.72,
        limiter_ceiling_db: -0.3,
        hw_eq_tenths_db: [15, -20, -10, 15, 38, 35, 20, 25],
      },
      {
        id: "dolby_music_studio",
        name: "Dolby Music Studio",
        subtitle: "Mastered Harman target curve, studio crossfeed, warm harmonic sub-bass & natural dynamics",
        bypass: false,
        auto_headroom: true,
        preamp_db: 0.0,
        harman_correction_enabled: true,
        bands: bandsFor([2.5, -1.0, -4.0, -2.0, 0.5, 1.2, 3.2, 3.5, 2.2, 3.2]),
        surround_width: 0.48,
        crossfeed_amount: 0.28,
        sub_bass_exciter: 0.46,
        vocal_clarity: 0.45,
        room_space: 0.20,
        room_model: "studio_booth",
        dolby_leveler_amount: 0.48,
        limiter_ceiling_db: -0.3,
        hw_eq_tenths_db: [-10, -25, 0, 12, 32, 30, 22, 30],
      },
      {
        id: "bassup_ultra_clean",
        name: "BassUp Ultra Clean",
        subtitle: "Deep 31-62 Hz sub-bass slam with 125 Hz anti-mud notch so vocals stay crystal clear",
        bypass: false,
        auto_headroom: true,
        preamp_db: 0.0,
        harman_correction_enabled: true,
        bands: bandsFor([5.5, 3.2, -4.8, -2.5, 0.0, 1.0, 2.8, 3.2, 2.0, 2.8]),
        surround_width: 0.40,
        crossfeed_amount: 0.20,
        sub_bass_exciter: 0.85,
        vocal_clarity: 0.40,
        room_space: 0.14,
        room_model: "studio_booth",
        dolby_leveler_amount: 0.55,
        limiter_ceiling_db: -0.3,
        hw_eq_tenths_db: [35, -30, -15, 10, 28, 30, 20, 25],
      },
      {
        id: "fps_gaming_footsteps",
        name: "FPS Gaming Footsteps",
        subtitle: "Pinpoint 7.1 binaural imaging, 2-6 kHz footstep & reload detail, low-end explosion control",
        bypass: false,
        auto_headroom: true,
        preamp_db: 0.5,
        harman_correction_enabled: true,
        bands: bandsFor([-2.0, -3.5, -5.0, -1.5, 1.0, 2.5, 4.8, 5.2, 3.5, 2.5]),
        surround_width: 0.88,
        crossfeed_amount: 0.10,
        sub_bass_exciter: 0.15,
        vocal_clarity: 0.58,
        room_space: 0.08,
        room_model: "studio_booth",
        dolby_leveler_amount: 0.80,
        limiter_ceiling_db: -0.3,
        hw_eq_tenths_db: [-35, -40, 10, 25, 48, 50, 35, 20],
      },
      {
        id: "podcast_late_night",
        name: "Podcast & Late Night",
        subtitle: "Dialog isolation, sub-bass rumble cut, and strong upward/downward loudness leveling",
        bypass: false,
        auto_headroom: true,
        preamp_db: 1.0,
        harman_correction_enabled: true,
        bands: bandsFor([-4.0, -4.5, -4.5, -1.0, 1.5, 3.2, 4.2, 3.8, 1.0, 0.5]),
        surround_width: 0.20,
        crossfeed_amount: 0.32,
        sub_bass_exciter: 0.05,
        vocal_clarity: 0.88,
        room_space: 0.10,
        room_model: "studio_booth",
        dolby_leveler_amount: 0.90,
        limiter_ceiling_db: -0.3,
        hw_eq_tenths_db: [-45, -35, 15, 32, 42, 38, 10, 5],
      },
    ];
  }

  let studio = {
    profile: makeFallbackPresets()[0],
    presets: makeFallbackPresets(),
    r50i: {
      connected: false,
      simulated: true,
      device_name: "Soundcore R50i NC (A3959)",
      mac_address: "AC:12:2F:39:59:A0",
      left_battery_level: 9,
      right_battery_level: 9,
      left_battery_percent: 90,
      right_battery_percent: 90,
      left_charging: false,
      right_charging: false,
      firmware_left: "01.64",
      firmware_right: "01.64",
      serial_number: "3959685923BF067C",
      eq_preset_id: 0xfefe,
      eq_gains_tenths_db: [-25, -18, 5, 18, 32, 24, 10, 15],
      drc_preview_tenths_db: [-2, -2, 1, 2, 4, 3, 1, 2],
      bass_up_enabled: false,
      buttons: {
        left_single: "volume_down",
        right_single: "volume_up",
        left_double: "play_pause",
        right_double: "next_song",
        left_triple: "voice_assistant",
        right_triple: "game_mode",
        left_long: "ambient_sound_mode",
        right_long: "ambient_sound_mode",
      },
      sound_modes: {
        ambient_mode: "noise_canceling",
        manual_anc_level: 5,
        adaptive_anc_level: 3,
        anc_mode: "adaptive",
        wind_noise_suppression: true,
        adaptive_sensitivity: 5,
        multi_scene_anc: "transport",
      },
      touch_tone: true,
      onboard_surround: false,
      gaming_mode: false,
    },
    apo: {
      live_config_path: "C:\\ProgramData\\SoundBettrCore\\live_apo.txt",
      equalizer_apo_detected: false,
      equalizer_apo_hooked: false,
      last_synced_preset: "R50i NC Harman Reference",
    },
    response_curve: [],
    last_packet_hex: "08 EE 00 00 00 02 83 1C 00 FE FE 5F 66 7D 8A 98 90 82 87 76 76 79 7A 7C 7B 79 7A 4A",
  };

  function invokeTauri(cmd, args) {
    if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
      return window.__TAURI__.core.invoke(cmd, args || {});
    }
    return Promise.reject(new Error("Tauri bridge unavailable"));
  }

  function formatSignedDb(val) {
    const n = Number(val) || 0;
    return (n >= 0 ? "+" : "") + n.toFixed(1) + " dB";
  }

  function formatFreqLabel(f) {
    return f >= 1000 ? (f / 1000).toFixed(f % 1000 === 0 ? 0 : 1) + "k" : String(f);
  }

  function setDialArc(arcId, valId, pct) {
    const clamped = Math.max(0, Math.min(100, Math.round(pct)));
    const arc = document.getElementById(arcId);
    const label = document.getElementById(valId);
    if (arc) {
      const circumference = 188.5;
      const offset = circumference * (1 - clamped / 100);
      arc.setAttribute("stroke-dashoffset", offset.toFixed(1));
    }
    if (label) {
      label.textContent = clamped + "%";
    }
  }

  function switchView(mode) {
    const views = ["simple", "pro", "hw"];
    views.forEach(function (v) {
      const sec = document.getElementById("view-" + v);
      const tab = document.getElementById("tab-" + v);
      const nav = document.getElementById("nav-" + v);
      const active = v === mode;
      if (sec) sec.classList.toggle("hidden-panel", !active);
      if (tab) tab.setAttribute("aria-selected", active ? "true" : "false");
      if (nav) {
        if (active) nav.setAttribute("aria-current", "page");
        else nav.removeAttribute("aria-current");
      }
    });
  }

  function renderPresetTiles() {
    const grid = document.getElementById("preset-grid");
    if (!grid) return;
    grid.innerHTML = "";
    studio.presets.forEach(function (p) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "preset-tile";
      const isSelected = studio.profile.id === p.id;
      btn.setAttribute("aria-pressed", isSelected ? "true" : "false");
      btn.innerHTML =
        '<span class="preset-name"></span><span class="preset-desc"></span>';
      btn.querySelector(".preset-name").textContent = p.name;
      btn.querySelector(".preset-desc").textContent = p.subtitle;
      btn.addEventListener("click", function () {
        selectPresetById(p.id);
      });
      grid.appendChild(btn);
    });
  }

  function renderHostEqRack() {
    const grid = document.getElementById("host-eq-grid");
    if (!grid) return;
    grid.innerHTML = "";
    studio.profile.bands.forEach(function (band, idx) {
      const col = document.createElement("div");
      col.className = "eq-band-col";
      col.innerHTML =
        '<span class="eq-gain-val" id="host-gain-lbl-' + idx + '"></span>' +
        '<div class="v-slider-wrap">' +
        '<input type="range" class="v-slider" min="-12" max="12" step="0.5" aria-label="Host EQ ' + band.freq_hz + ' Hz">' +
        "</div>" +
        '<span class="eq-freq-val">' + formatFreqLabel(band.freq_hz) + "</span>" +
        '<span class="eq-q-val">Q ' + band.q.toFixed(2) + "</span>";
      const lbl = col.querySelector(".eq-gain-val");
      const slider = col.querySelector("input");
      lbl.textContent = formatSignedDb(band.gain_db);
      slider.value = String(band.gain_db);
      slider.addEventListener("input", function () {
        const g = parseFloat(slider.value);
        studio.profile.bands[idx].gain_db = g;
        lbl.textContent = formatSignedDb(g);
        pushProfileUpdate();
      });
      grid.appendChild(col);
    });
  }

  function renderHwEqRack() {
    const grid = document.getElementById("hw-eq-grid");
    if (!grid) return;
    grid.innerHTML = "";
    HW_FREQS.forEach(function (freq, idx) {
      const tenths = studio.r50i.eq_gains_tenths_db[idx] || 0;
      const drcTenths =
        (studio.r50i.drc_preview_tenths_db && studio.r50i.drc_preview_tenths_db[idx]) || 0;
      const col = document.createElement("div");
      col.className = "eq-band-col";
      col.innerHTML =
        '<span class="eq-gain-val" id="hw-gain-lbl-' + idx + '"></span>' +
        '<div class="v-slider-wrap">' +
        '<input type="range" class="v-slider" min="-120" max="120" step="5" aria-label="R50i NC Hardware EQ ' + freq + ' Hz">' +
        "</div>" +
        '<span class="eq-freq-val">' + formatFreqLabel(freq) + "</span>" +
        '<span class="eq-q-val" id="hw-drc-lbl-' + idx + '"></span>';
      const lbl = col.querySelector(".eq-gain-val");
      const drcLbl = col.querySelector(".eq-q-val");
      const slider = col.querySelector("input");
      lbl.textContent = formatSignedDb(tenths / 10);
      drcLbl.textContent = "DRC " + (drcTenths >= 0 ? "+" : "") + (drcTenths / 10).toFixed(1);
      slider.value = String(tenths);
      slider.addEventListener("input", function () {
        const v = parseInt(slider.value, 10);
        studio.r50i.eq_gains_tenths_db[idx] = v;
        studio.profile.hw_eq_tenths_db[idx] = v;
        lbl.textContent = formatSignedDb(v / 10);
      });
      grid.appendChild(col);
    });
  }

  function renderTouchRemapList() {
    const container = document.getElementById("touch-remap-list");
    if (!container) return;
    container.innerHTML = "";
    BUTTON_SLOTS.forEach(function (slot) {
      const row = document.createElement("div");
      row.className = "kv-row";
      const keySpan = document.createElement("span");
      keySpan.className = "kv-key";
      keySpan.textContent = slot.label;

      const sel = document.createElement("select");
      sel.className = "select-box";
      sel.setAttribute("aria-label", slot.label);
      BUTTON_ACTIONS.forEach(function (act) {
        const opt = document.createElement("option");
        opt.value = act.value;
        opt.textContent = act.label;
        if (studio.r50i.buttons[slot.key] === act.value) {
          opt.selected = true;
        }
        sel.appendChild(opt);
      });
      sel.addEventListener("change", function () {
        const nextAction = sel.value;
        studio.r50i.buttons[slot.key] = nextAction;
        invokeTauri("set_r50i_button_action", {
          side: slot.side,
          press: slot.press,
          action: nextAction,
        })
          .then(applySnapshot)
          .catch(function () {
            refreshUiFromState();
          });
      });

      row.appendChild(keySpan);
      row.appendChild(sel);
      container.appendChild(row);
    });
  }

  function refreshUiFromState() {
    const p = studio.profile;
    const r = studio.r50i;
    const apo = studio.apo;

    const activeLbl = document.getElementById("active-preset-label");
    if (activeLbl) {
      activeLbl.textContent = (p.bypass ? "Bypassed: " : "Active: ") + p.name;
    }

    const bypassBtn = document.getElementById("btn-bypass");
    if (bypassBtn) {
      bypassBtn.setAttribute("data-bypass", p.bypass ? "true" : "false");
      bypassBtn.textContent = p.bypass ? "DSP Bypassed (Raw)" : "A/B DSP Active";
    }

    const btPill = document.getElementById("bt-status-pill");
    if (btPill) {
      const modeTag = r.connected ? "RFCOMM Live" : "A3959 Ready";
      btPill.setAttribute("data-tone", r.connected ? "positive" : "warning");
      btPill.textContent =
        "R50i NC (" + modeTag + "): L " + r.left_battery_percent + "%, R " + r.right_battery_percent + "%";
    }

    const apoPill = document.getElementById("apo-status-pill");
    if (apoPill) {
      apoPill.setAttribute("data-tone", apo.equalizer_apo_hooked ? "positive" : "neutral");
      apoPill.textContent = apo.equalizer_apo_hooked
        ? "EqualizerAPO Hooked"
        : "APO Live File Synced";
    }

    setDialArc("arc-surround", "val-surround", p.surround_width * 100);
    setDialArc("arc-subbass", "val-subbass", p.sub_bass_exciter * 100);
    setDialArc("arc-clarity", "val-clarity", p.vocal_clarity * 100);
    setDialArc("arc-space", "val-space", p.room_space * 100);

    const rngSurround = document.getElementById("rng-surround");
    if (rngSurround) rngSurround.value = String(Math.round(p.surround_width * 100));
    const rngSub = document.getElementById("rng-subbass");
    if (rngSub) rngSub.value = String(Math.round(p.sub_bass_exciter * 100));
    const rngClarity = document.getElementById("rng-clarity");
    if (rngClarity) rngClarity.value = String(Math.round(p.vocal_clarity * 100));
    const rngSpace = document.getElementById("rng-space");
    if (rngSpace) rngSpace.value = String(Math.round(p.room_space * 100));

    const rngPreamp = document.getElementById("rng-preamp");
    if (rngPreamp) rngPreamp.value = String(p.preamp_db);
    const valPreamp = document.getElementById("val-preamp");
    if (valPreamp) valPreamp.textContent = formatSignedDb(p.preamp_db);

    const rngDolby = document.getElementById("rng-dolby-leveler");
    if (rngDolby) rngDolby.value = String(Math.round(p.dolby_leveler_amount * 100));
    const valDolby = document.getElementById("val-dolby-leveler");
    if (valDolby) valDolby.textContent = Math.round(p.dolby_leveler_amount * 100) + "%";

    const rngCf = document.getElementById("rng-crossfeed");
    if (rngCf) rngCf.value = String(Math.round(p.crossfeed_amount * 100));
    const valCf = document.getElementById("val-crossfeed");
    if (valCf) valCf.textContent = Math.round(p.crossfeed_amount * 100) + "%";

    const selRoom = document.getElementById("sel-room-model");
    if (selRoom) selRoom.value = p.room_model;

    const spatialMeta = document.getElementById("spatial-meta");
    if (spatialMeta) {
      spatialMeta.textContent =
        "Width " +
        Math.round(p.surround_width * 100) +
        "%, Crossfeed " +
        Math.round(p.crossfeed_amount * 100) +
        "%, Leveler " +
        Math.round(p.dolby_leveler_amount * 100) +
        "%";
    }

    const apoCode = document.getElementById("apo-path-code");
    if (apoCode) apoCode.textContent = apo.live_config_path;

    document.querySelectorAll("#quick-ambient-group .seg-btn").forEach(function (b) {
      b.setAttribute(
        "aria-pressed",
        b.getAttribute("data-ambient") === r.sound_modes.ambient_mode ? "true" : "false"
      );
    });
    document.querySelectorAll("#quick-anc-mode-group .seg-btn").forEach(function (b) {
      b.setAttribute(
        "aria-pressed",
        b.getAttribute("data-ancmode") === r.sound_modes.anc_mode ? "true" : "false"
      );
    });

    const rngAnc = document.getElementById("rng-anc-level");
    if (rngAnc) rngAnc.value = String(r.sound_modes.manual_anc_level);
    const valAnc = document.getElementById("val-anc-level");
    if (valAnc) valAnc.textContent = "Level " + r.sound_modes.manual_anc_level;

    const selScene = document.getElementById("sel-multiscene");
    if (selScene) selScene.value = r.sound_modes.multi_scene_anc;

    const btnWind = document.getElementById("btn-wind-noise");
    if (btnWind) {
      btnWind.setAttribute("aria-pressed", r.sound_modes.wind_noise_suppression ? "true" : "false");
      btnWind.textContent = r.sound_modes.wind_noise_suppression ? "Enabled" : "Off";
    }

    const btnSurr = document.getElementById("btn-hw-surround");
    if (btnSurr) btnSurr.setAttribute("aria-pressed", r.onboard_surround ? "true" : "false");

    const btnGame = document.getElementById("btn-hw-gaming");
    if (btnGame) btnGame.setAttribute("aria-pressed", r.gaming_mode ? "true" : "false");

    const btnBass = document.getElementById("btn-hw-bassup");
    if (btnBass) btnBass.setAttribute("aria-pressed", r.bass_up_enabled ? "true" : "false");

    const btnTone = document.getElementById("btn-touch-tone");
    if (btnTone) {
      btnTone.setAttribute("aria-pressed", r.touch_tone ? "true" : "false");
      btnTone.textContent = r.touch_tone ? "Enabled" : "Muted";
    }

    const fwReadout = document.getElementById("fw-serial-readout");
    if (fwReadout) {
      fwReadout.textContent = "Firmware " + r.firmware_left + ", serial " + r.serial_number.toLowerCase();
    }
    const hwDev = document.getElementById("hw-device-name");
    if (hwDev) hwDev.textContent = r.device_name + " (" + r.mac_address + ")";
    const hwBat = document.getElementById("hw-battery-detail");
    if (hwBat) {
      hwBat.textContent =
        "L " +
        r.left_battery_percent +
        "% (" +
        r.left_battery_level +
        "/10), R " +
        r.right_battery_percent +
        "% (" +
        r.right_battery_level +
        "/10)";
    }
    const hwFw = document.getElementById("hw-fw-detail");
    if (hwFw) hwFw.textContent = r.firmware_left + " / " + r.serial_number.toLowerCase();

    const pktHex = document.getElementById("last-packet-hex");
    if (pktHex) pktHex.textContent = studio.last_packet_hex;

    renderPresetTiles();
    renderHostEqRack();
    renderHwEqRack();
    renderTouchRemapList();
    syncWebAudioGraph();
  }

  function applySnapshot(snap) {
    if (!snap) return;
    studio = snap;
    refreshUiFromState();
  }

  function pushProfileUpdate() {
    invokeTauri("apply_dsp_profile", { profile: studio.profile })
      .then(applySnapshot)
      .catch(function () {
        refreshUiFromState();
      });
  }

  function selectPresetById(id) {
    invokeTauri("select_preset", { presetId: id })
      .then(applySnapshot)
      .catch(function () {
        const found = studio.presets.find(function (p) {
          return p.id === id;
        });
        if (found) {
          studio.profile = JSON.parse(JSON.stringify(found));
          studio.r50i.eq_gains_tenths_db = studio.profile.hw_eq_tenths_db.slice();
          refreshUiFromState();
        }
      });
  }

  function pushSoundModesUpdate() {
    invokeTauri("set_r50i_sound_modes", { modes: studio.r50i.sound_modes })
      .then(applySnapshot)
      .catch(function () {
        refreshUiFromState();
      });
  }

  // WebAudio live DSP chain + analyser for built-in player and test signal auditioning
  let audioCtx = null;
  let activeSourceNode = null;
  let analyserNode = null;
  let biquadNodes = [];
  let preampGainNode = null;
  let compressorNode = null;
  let orbitAngle = 0;

  function ensureAudioContext() {
    if (!audioCtx) {
      const Ctor = window.AudioContext || window.webkitAudioContext;
      audioCtx = new Ctor({ sampleRate: 48000 });
      preampGainNode = audioCtx.createGain();
      analyserNode = audioCtx.createAnalyser();
      analyserNode.fftSize = 1024;
      analyserNode.smoothingTimeConstant = 0.82;
      compressorNode = audioCtx.createDynamicsCompressor();
      compressorNode.threshold.value = -16;
      compressorNode.knee.value = 12;
      compressorNode.ratio.value = 3.5;
      compressorNode.attack.value = 0.005;
      compressorNode.release.value = 0.14;

      biquadNodes = HOST_FREQS.map(function (freq, idx) {
        const f = audioCtx.createBiquadFilter();
        f.type = idx === 0 ? "lowshelf" : idx === 9 ? "highshelf" : "peaking";
        f.frequency.value = freq;
        f.Q.value = idx === 0 || idx === 9 ? 0.707 : 1.41;
        f.gain.value = 0;
        return f;
      });

      let head = preampGainNode;
      biquadNodes.forEach(function (node) {
        head.connect(node);
        head = node;
      });
      head.connect(compressorNode);
      compressorNode.connect(analyserNode);
      analyserNode.connect(audioCtx.destination);
    }
    if (audioCtx.state === "suspended") {
      audioCtx.resume();
    }
    syncWebAudioGraph();
  }

  function syncWebAudioGraph() {
    if (!audioCtx || !preampGainNode) return;
    const p = studio.profile;
    if (p.bypass) {
      preampGainNode.gain.value = 1.0;
      biquadNodes.forEach(function (node) {
        node.gain.value = 0;
      });
      compressorNode.ratio.value = 1.0;
      return;
    }
    preampGainNode.gain.value = Math.pow(10, (p.preamp_db || 0) / 20);
    biquadNodes.forEach(function (node, idx) {
      const band = p.bands[idx];
      let g = band && band.enabled ? band.gain_db : 0;
      if (idx === 1) g += p.sub_bass_exciter * 3.2;
      if (idx === 6 || idx === 7) g += p.vocal_clarity * 2.4;
      node.gain.value = g;
    });
    compressorNode.ratio.value = 1.2 + p.dolby_leveler_amount * 4.5;
  }

  function stopCurrentPlayback() {
    if (activeSourceNode) {
      try {
        activeSourceNode.stop();
      } catch (_) {}
      try {
        activeSourceNode.disconnect();
      } catch (_) {}
      activeSourceNode = null;
    }
    const status = document.getElementById("player-status-text");
    if (status) status.textContent = "Playback stopped";
  }

  function playTestSignal(kind, label) {
    ensureAudioContext();
    stopCurrentPlayback();
    const status = document.getElementById("player-status-text");
    if (status) status.textContent = "Synthesizing " + label + " through 7-stage DSP...";

    invokeTauri("render_test_signal_wav", { kind: kind, durationSecs: 5.0 })
      .then(function (wavBytes) {
        const u8 = new Uint8Array(wavBytes);
        return audioCtx.decodeAudioData(u8.buffer.slice(0));
      })
      .then(function (decoded) {
        const src = audioCtx.createBufferSource();
        src.buffer = decoded;
        src.loop = true;
        // Signal from Rust is already processed, connect through analyser
        src.connect(analyserNode);
        src.start();
        activeSourceNode = src;
        if (status) status.textContent = "Playing: " + label + " (48 kHz Stereo)";
      })
      .catch(function () {
        // Browser fallback synthesizer through WebAudio biquad chain
        const sr = audioCtx.sampleRate;
        const duration = 4.0;
        const frames = sr * duration;
        const buf = audioCtx.createBuffer(2, frames, sr);
        const left = buf.getChannelData(0);
        const right = buf.getChannelData(1);
        for (let i = 0; i < frames; i++) {
          const t = i / sr;
          const angle = 2 * Math.PI * (t / duration);
          const tone = Math.sin(2 * Math.PI * (kind === "sub_bass_sweep" ? 55 : 440) * t) * 0.25;
          left[i] = tone * Math.cos(angle * 0.5);
          right[i] = tone * Math.sin(angle * 0.5);
        }
        const src = audioCtx.createBufferSource();
        src.buffer = buf;
        src.loop = true;
        src.connect(preampGainNode);
        src.start();
        activeSourceNode = src;
        if (status) status.textContent = "Playing: " + label;
      });
  }

  // Canvas visualizers (Spectrum + EQ Response Curve & 7.1 Speaker Radar)
  function drawCanvases() {
    const specCanvas = document.getElementById("spectrum-canvas");
    if (specCanvas) {
      const ctx = specCanvas.getContext("2d");
      const w = specCanvas.width;
      const h = specCanvas.height;
      ctx.fillStyle = "#171719";
      ctx.fillRect(0, 0, w, h);

      // Grid lines
      ctx.strokeStyle = "#222224";
      ctx.lineWidth = 1;
      [0.25, 0.5, 0.75].forEach(function (frac) {
        const y = Math.round(h * frac) + 0.5;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(w, y);
        ctx.stroke();
      });

      // Live FFT bars if playing, or subtle noise floor
      if (analyserNode) {
        const bins = new Uint8Array(analyserNode.frequencyBinCount);
        analyserNode.getByteFrequencyData(bins);
        ctx.fillStyle = "#212023";
        const barCount = 48;
        const step = w / barCount;
        for (let i = 0; i < barCount; i++) {
          const binIdx = Math.min(bins.length - 1, Math.floor(Math.pow(i / barCount, 1.8) * 180));
          const mag = bins[binIdx] / 255.0;
          const barH = Math.max(3, mag * (h - 24));
          ctx.fillRect(i * step + 2, h - barH, step - 3, barH);
        }
      }

      // Composite EQ Response curve
      ctx.strokeStyle = studio.profile.bypass ? "#8a8c92" : "#4a8ad6";
      ctx.lineWidth = 2;
      ctx.beginPath();
      const bands = studio.profile.bands;
      for (let x = 0; x < w; x++) {
        const norm = x / (w - 1);
        const freq = 20 * Math.pow(1000, norm);
        let db = studio.profile.bypass ? 0 : studio.profile.preamp_db;
        if (!studio.profile.bypass) {
          bands.forEach(function (b) {
            const octDist = Math.log2(freq / b.freq_hz);
            db += b.gain_db * Math.exp(-(octDist * octDist) * (b.q * 1.4));
          });
        }
        const y = h * 0.5 - (db / 15.0) * (h * 0.42);
        if (x === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.stroke();

      // Axis labels
      ctx.fillStyle = "#8a8c92";
      ctx.font = "11px sans-serif";
      ctx.fillText("+12 dB", 8, 16);
      ctx.fillText("0 dB", 8, h * 0.5 - 4);
      ctx.fillText("-12 dB", 8, h - 8);
      ctx.fillText("31 Hz", 58, h - 8);
      ctx.fillText("125 Hz (Mud Cut)", 135, h - 8);
      ctx.fillText("1 kHz", 285, h - 8);
      ctx.fillText("4 kHz (Harman Clarity)", 365, h - 8);
    }

    const radarCanvas = document.getElementById("radar-canvas");
    if (radarCanvas) {
      const rctx = radarCanvas.getContext("2d");
      const rw = radarCanvas.width;
      const rh = radarCanvas.height;
      rctx.fillStyle = "#171719";
      rctx.fillRect(0, 0, rw, rh);

      const cx = rw * 0.5;
      const cy = rh * 0.52;
      const radius = Math.min(rw, rh) * 0.38 * (0.65 + studio.profile.surround_width * 0.35);

      rctx.strokeStyle = "#222224";
      rctx.lineWidth = 1;
      [0.5, 1.0].forEach(function (scale) {
        rctx.beginPath();
        rctx.arc(cx, cy, radius * scale, 0, Math.PI * 2);
        rctx.stroke();
      });

      const speakers = [
        { name: "C", deg: -90 },
        { name: "FL", deg: -130 },
        { name: "FR", deg: -50 },
        { name: "SL", deg: 180 },
        { name: "SR", deg: 0 },
        { name: "RL", deg: 135 },
        { name: "RR", deg: 45 },
      ];
      rctx.fillStyle = "#4a8ad6";
      rctx.font = "11px sans-serif";
      speakers.forEach(function (sp) {
        const rad = (sp.deg * Math.PI) / 180;
        const sx = cx + Math.cos(rad) * radius;
        const sy = cy + Math.sin(rad) * radius;
        rctx.beginPath();
        rctx.arc(sx, sy, 4, 0, Math.PI * 2);
        rctx.fill();
        rctx.fillStyle = "#ffffff";
        rctx.fillText(sp.name, sx - 7, sy - 8);
        rctx.fillStyle = "#4a8ad6";
      });

      orbitAngle += 0.025;
      const ox = cx + Math.cos(orbitAngle) * radius * 0.85;
      const oy = cy + Math.sin(orbitAngle) * radius * 0.85;
      rctx.fillStyle = "#5fc39c";
      rctx.beginPath();
      rctx.arc(ox, oy, 5, 0, Math.PI * 2);
      rctx.fill();
    }

    requestAnimationFrame(drawCanvases);
  }

  function wireEvents() {
    ["simple", "pro", "hw"].forEach(function (mode) {
      const tab = document.getElementById("tab-" + mode);
      const nav = document.getElementById("nav-" + mode);
      if (tab) {
        tab.addEventListener("click", function () {
          switchView(mode);
        });
      }
      if (nav) {
        nav.addEventListener("click", function () {
          switchView(mode);
        });
      }
    });

    const bypassBtn = document.getElementById("btn-bypass");
    if (bypassBtn) {
      bypassBtn.addEventListener("click", function () {
        invokeTauri("toggle_bypass")
          .then(applySnapshot)
          .catch(function () {
            studio.profile.bypass = !studio.profile.bypass;
            refreshUiFromState();
          });
      });
    }

    const scanBtn = document.getElementById("btn-scan-bt");
    if (scanBtn) {
      scanBtn.addEventListener("click", function () {
        invokeTauri("scan_and_connect_r50i", {})
          .then(applySnapshot)
          .catch(function () {
            refreshUiFromState();
          });
      });
    }

    function bindMacroRange(id, field, arcId, valId) {
      const el = document.getElementById(id);
      if (!el) return;
      el.addEventListener("input", function () {
        const pct = parseFloat(el.value);
        studio.profile[field] = pct / 100.0;
        setDialArc(arcId, valId, pct);
        pushProfileUpdate();
      });
    }
    bindMacroRange("rng-surround", "surround_width", "arc-surround", "val-surround");
    bindMacroRange("rng-subbass", "sub_bass_exciter", "arc-subbass", "val-subbass");
    bindMacroRange("rng-clarity", "vocal_clarity", "arc-clarity", "val-clarity");
    bindMacroRange("rng-space", "room_space", "arc-space", "val-space");

    const rngPreamp = document.getElementById("rng-preamp");
    if (rngPreamp) {
      rngPreamp.addEventListener("input", function () {
        studio.profile.preamp_db = parseFloat(rngPreamp.value);
        pushProfileUpdate();
      });
    }

    const btnHarman = document.getElementById("btn-reset-harman");
    if (btnHarman) {
      btnHarman.addEventListener("click", function () {
        selectPresetById("r50i_harman_reference");
      });
    }

    const btnFlashHw = document.getElementById("btn-flash-hw-eq");
    if (btnFlashHw) {
      btnFlashHw.addEventListener("click", function () {
        invokeTauri("flash_r50i_hardware_eq", {
          gainsTenthsDb: studio.r50i.eq_gains_tenths_db,
          bassUp: studio.r50i.bass_up_enabled,
        })
          .then(applySnapshot)
          .catch(function () {
            refreshUiFromState();
          });
      });
    }

    const rngDolby = document.getElementById("rng-dolby-leveler");
    if (rngDolby) {
      rngDolby.addEventListener("input", function () {
        studio.profile.dolby_leveler_amount = parseFloat(rngDolby.value) / 100.0;
        pushProfileUpdate();
      });
    }

    const rngCf = document.getElementById("rng-crossfeed");
    if (rngCf) {
      rngCf.addEventListener("input", function () {
        studio.profile.crossfeed_amount = parseFloat(rngCf.value) / 100.0;
        pushProfileUpdate();
      });
    }

    const selRoom = document.getElementById("sel-room-model");
    if (selRoom) {
      selRoom.addEventListener("change", function () {
        studio.profile.room_model = selRoom.value;
        pushProfileUpdate();
      });
    }

    document.querySelectorAll("#quick-ambient-group .seg-btn").forEach(function (btn) {
      btn.addEventListener("click", function () {
        studio.r50i.sound_modes.ambient_mode = btn.getAttribute("data-ambient");
        pushSoundModesUpdate();
      });
    });

    document.querySelectorAll("#quick-anc-mode-group .seg-btn").forEach(function (btn) {
      btn.addEventListener("click", function () {
        studio.r50i.sound_modes.anc_mode = btn.getAttribute("data-ancmode");
        pushSoundModesUpdate();
      });
    });

    const rngAnc = document.getElementById("rng-anc-level");
    if (rngAnc) {
      rngAnc.addEventListener("input", function () {
        studio.r50i.sound_modes.manual_anc_level = parseInt(rngAnc.value, 10);
        pushSoundModesUpdate();
      });
    }

    const selScene = document.getElementById("sel-multiscene");
    if (selScene) {
      selScene.addEventListener("change", function () {
        studio.r50i.sound_modes.multi_scene_anc = selScene.value;
        pushSoundModesUpdate();
      });
    }

    const btnWind = document.getElementById("btn-wind-noise");
    if (btnWind) {
      btnWind.addEventListener("click", function () {
        studio.r50i.sound_modes.wind_noise_suppression =
          !studio.r50i.sound_modes.wind_noise_suppression;
        pushSoundModesUpdate();
      });
    }

    const btnSurr = document.getElementById("btn-hw-surround");
    if (btnSurr) {
      btnSurr.addEventListener("click", function () {
        const next = !studio.r50i.onboard_surround;
        studio.r50i.onboard_surround = next;
        invokeTauri("set_r50i_surround_mode", { enabled: next })
          .then(applySnapshot)
          .catch(function () {
            refreshUiFromState();
          });
      });
    }

    const btnGame = document.getElementById("btn-hw-gaming");
    if (btnGame) {
      btnGame.addEventListener("click", function () {
        const next = !studio.r50i.gaming_mode;
        studio.r50i.gaming_mode = next;
        invokeTauri("set_r50i_gaming_mode", { enabled: next })
          .then(applySnapshot)
          .catch(function () {
            refreshUiFromState();
          });
      });
    }

    const btnBass = document.getElementById("btn-hw-bassup");
    if (btnBass) {
      btnBass.addEventListener("click", function () {
        const next = !studio.r50i.bass_up_enabled;
        studio.r50i.bass_up_enabled = next;
        invokeTauri("flash_r50i_hardware_eq", {
          gainsTenthsDb: studio.r50i.eq_gains_tenths_db,
          bassUp: next,
        })
          .then(applySnapshot)
          .catch(function () {
            refreshUiFromState();
          });
      });
    }

    const btnTone = document.getElementById("btn-touch-tone");
    if (btnTone) {
      btnTone.addEventListener("click", function () {
        const next = !studio.r50i.touch_tone;
        studio.r50i.touch_tone = next;
        invokeTauri("set_r50i_touch_tone", { enabled: next })
          .then(applySnapshot)
          .catch(function () {
            refreshUiFromState();
          });
      });
    }

    const btnOrbit = document.getElementById("btn-test-orbit");
    if (btnOrbit) {
      btnOrbit.addEventListener("click", function () {
        playTestSignal("orbit71", "7.1 Speaker Orbit 360");
      });
    }
    const btnSubSweep = document.getElementById("btn-test-subbass");
    if (btnSubSweep) {
      btnSubSweep.addEventListener("click", function () {
        playTestSignal("sub_bass_sweep", "25-160 Hz Sub-Bass Sweep");
      });
    }
    const btnVocal = document.getElementById("btn-test-vocal");
    if (btnVocal) {
      btnVocal.addEventListener("click", function () {
        playTestSignal("vocal_pulse", "Vocal Clarity Pulse");
      });
    }
    const btnPink = document.getElementById("btn-test-pink");
    if (btnPink) {
      btnPink.addEventListener("click", function () {
        playTestSignal("pink_noise", "Pink Noise Reference");
      });
    }
    const btnStop = document.getElementById("btn-stop-audio");
    if (btnStop) {
      btnStop.addEventListener("click", stopCurrentPlayback);
    }

    const btnLoadAudio = document.getElementById("btn-load-audio");
    const fileAudio = document.getElementById("file-audio-input");
    if (btnLoadAudio && fileAudio) {
      btnLoadAudio.addEventListener("click", function () {
        fileAudio.click();
      });
      fileAudio.addEventListener("change", function () {
        const file = fileAudio.files && fileAudio.files[0];
        if (!file) return;
        ensureAudioContext();
        stopCurrentPlayback();
        const reader = new FileReader();
        reader.onload = function () {
          audioCtx.decodeAudioData(reader.result).then(function (decoded) {
            const src = audioCtx.createBufferSource();
            src.buffer = decoded;
            src.connect(preampGainNode);
            src.start();
            activeSourceNode = src;
            const status = document.getElementById("player-status-text");
            if (status) status.textContent = "Playing file: " + file.name;
          });
        };
        reader.readAsArrayBuffer(file);
      });
    }

    const btnExportApo = document.getElementById("btn-export-apo");
    if (btnExportApo) {
      btnExportApo.addEventListener("click", function () {
        invokeTauri("export_equalizer_apo_text")
          .then(function (txt) {
            const blob = new Blob([txt], { type: "text/plain" });
            const a = document.createElement("a");
            a.href = URL.createObjectURL(blob);
            a.download = studio.profile.id + "_equalizer_apo.txt";
            a.click();
          })
          .catch(function () {});
      });
    }

    const btnExportJson = document.getElementById("btn-export-json");
    if (btnExportJson) {
      btnExportJson.addEventListener("click", function () {
        const json = JSON.stringify(studio.profile, null, 2);
        const blob = new Blob([json], { type: "application/json" });
        const a = document.createElement("a");
        a.href = URL.createObjectURL(blob);
        a.download = studio.profile.id + ".json";
        a.click();
      });
    }

    const btnImport = document.getElementById("btn-import-apo");
    const fileImport = document.getElementById("file-import-config");
    if (btnImport && fileImport) {
      btnImport.addEventListener("click", function () {
        fileImport.click();
      });
      fileImport.addEventListener("change", function () {
        const file = fileImport.files && fileImport.files[0];
        if (!file) return;
        const reader = new FileReader();
        reader.onload = function () {
          const text = String(reader.result || "");
          if (file.name.toLowerCase().endsWith(".json")) {
            invokeTauri("import_preset_json", { jsonStr: text })
              .then(applySnapshot)
              .catch(function () {});
          } else {
            invokeTauri("import_equalizer_apo_text", { content: text })
              .then(applySnapshot)
              .catch(function () {});
          }
        };
        reader.readAsText(file);
      });
    }

    if (window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.listen) {
      window.__TAURI__.event.listen("studio://updated", function (ev) {
        if (ev && ev.payload) applySnapshot(ev.payload);
      });
    }
  }

  document.addEventListener("DOMContentLoaded", function () {
    wireEvents();
    refreshUiFromState();
    requestAnimationFrame(drawCanvases);
    invokeTauri("get_studio_snapshot")
      .then(applySnapshot)
      .catch(function () {});
  });
})();
