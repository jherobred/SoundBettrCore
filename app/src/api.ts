import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Kind = "bluetooth" | "usb" | "wired" | "builtin";
export type Hue = "cobalt" | "amber" | "rose" | "jade";
export const HUES: [Hue, string][] = [["cobalt", "Blue"], ["amber", "Amber"], ["rose", "Rose"], ["jade", "Jade"]];

export interface Device {
  id: string;
  key: string;
  kind: Kind;
  name: string;
  hue: Hue;
  announce: boolean;
  profileId: string;
  lastSeen: number;
  isNew: boolean;
  hardware: string | null;
}

export interface Profile {
  id: string;
  name: string;
  source: "flat" | "autoeq" | "hearing" | "ab" | "custom";
  autoeqModel: string | null;
  gains: number[];
  preamp: number | null;
  balance: number;
  surround: boolean;
  crossfeed: boolean;
  hearing: { left: number[]; right: number[] } | null;
}

export interface Settings {
  theme: "light" | "dark" | "system";
  motion: boolean;
  voice: string;
  template: string;
  autoeqUpdated: string | null;
}

export type ConnState =
  | { state: "searching" }
  | { state: "recognized"; deviceId: string }
  | { state: "new"; deviceId: string }
  | { state: "choose"; deviceIds: string[] }
  | { state: "error"; cause: string };

export type HtStep =
  | { step: "play"; ear: "left" | "right"; freq: number; db: number; index: number; total: number }
  | { step: "done"; left: number[]; right: number[] };

export type Ambient = "noise_canceling" | "transparency" | "normal";
export interface SoundModes {
  ambient_mode: Ambient;
  manual_anc_level: number;
  adaptive_anc_level: number;
  anc_mode: "manual" | "adaptive" | "multi_scene";
  wind_noise_suppression: boolean;
  adaptive_sensitivity: number;
  multi_scene_anc: "transport" | "outdoor" | "indoor";
}

/** The earbuds' own state, as the A3959 protocol reports it. */
export interface Earbuds {
  left_battery_percent: number;
  right_battery_percent: number;
  left_charging: boolean;
  right_charging: boolean;
  firmware_left: string;
  eq_gains_tenths_db: number[];
  sound_modes: SoundModes;
  gaming_mode: boolean | null;
}

export interface Snapshot {
  platform: "windows" | "android" | "test";
  devices: Device[];
  profiles: Profile[];
  settings: Settings;
  conn: ConnState;
  current: string | null;
  applyError: string | null;
  apo: { installed: boolean; hooked: boolean } | null;
  speaking: boolean;
  voiceError: string | null;
  voices: { id: string; name: string; desc: string }[];
  hardware: Earbuds | null;
  hearing: HtStep | null;
  abRound: number | null;
  autoeqEntries: number;
}

export type CommandResult = { result: "confirmed" } | { result: "noReply" } | { result: "failed"; reason: string };
export type HwCommand =
  | { cmd: "soundModes"; modes: SoundModes }
  | { cmd: "gaming"; on: boolean }
  | { cmd: "eq"; gains_tenths_db: number[] }
  | { cmd: "refresh" };
export interface Hit { index: number; model: string; source: string }
export interface AbResult { winner: string; wins: number; rounds: number }

export const api = {
  snapshot: () => invoke<Snapshot>("get_snapshot"),
  updateDevice: (device: Device) => invoke<Snapshot>("update_device", { device }),
  forgetDevice: (id: string) => invoke<Snapshot>("forget_device", { id }),
  updateProfile: (profile: Profile) => invoke<Snapshot>("update_profile", { profile }),
  useProfile: (deviceId: string, from: string) => invoke<Snapshot>("use_profile", { deviceId, from }),
  searchAutoeq: (query: string, limit = 20) => invoke<Hit[]>("search_autoeq", { query, limit }),
  applyAutoeq: (deviceId: string, index: number) => invoke<Snapshot>("apply_autoeq", { deviceId, index }),
  announce: (deviceId: string) => invoke<void>("announce", { deviceId }),
  previewVoice: (voice: string, text: string) => invoke<void>("preview_voice", { voice, text }),
  setSettings: (settings: Settings) => invoke<Snapshot>("set_settings", { settings }),
  chooseDevice: (id: string | null) => invoke<Snapshot>("choose_device", { id }),
  notThisSet: () => invoke<Snapshot>("not_this_set"),
  hearingStart: () => invoke<HtStep>("hearing_start"),
  hearingAnswer: (heard: boolean) => invoke<HtStep | null>("hearing_answer", { heard }),
  hearingStop: () => invoke<Snapshot>("hearing_stop"),
  hearingApply: (deviceId: string) => invoke<Snapshot>("hearing_apply", { deviceId }),
  abStart: (deviceId: string, a: string, b: string) => invoke<Snapshot>("ab_start", { deviceId, a, b }),
  abListen: (side: "x" | "y") => invoke<Snapshot>("ab_listen", { side }),
  abPick: (side: "x" | "y") => invoke<AbResult | null>("ab_pick", { side }),
  abCancel: () => invoke<Snapshot>("ab_cancel"),
  hw: (deviceId: string, command: HwCommand) => invoke<CommandResult>("hw_command", { deviceId, command }),
  exportStore: (path: string) => invoke<void>("export_store", { path }),
  importStore: (path: string) => invoke<Snapshot>("import_store", { path }),
  updateAutoeq: () => invoke<Snapshot>("update_autoeq_db"),
  connectApo: () => invoke<void>("connect_apo"),
};

export function onSnapshot(fn: (s: Snapshot) => void): Promise<UnlistenFn> {
  return listen<Snapshot>("snapshot", (e) => fn(e.payload));
}

export function onSpeaking(fn: (on: boolean) => void): Promise<UnlistenFn> {
  return listen<boolean>("speaking", (e) => fn(e.payload));
}

export const BANDS = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
export const HW_BANDS = [100, 200, 400, 800, 1600, 3200, 6400, 12800];
export const fmtHz = (f: number) => (f >= 1000 ? `${f / 1000}k` : `${f}`);
export const autoPreamp = (g: number[]) => -Math.max(0, ...g);
export const balText = (b: number) => (b === 0 ? "Balance centered" : `Balance ${Math.abs(b)} dB toward ${b < 0 ? "left" : "right"}`);

/** Smooth curve through the 10 band gains, in a 300 x 96 box (from the mockup). */
export function gainsPath(g: number[]): string {
  const pts = g.map((v, i) => [(i / (g.length - 1)) * 300, 48 - Math.max(-12, Math.min(12, v)) * 3.6]);
  let d = `M${pts[0][0]} ${pts[0][1]}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i - 1] || pts[i], p1 = pts[i], p2 = pts[i + 1], p3 = pts[i + 2] || p2;
    d += ` C${p1[0] + (p2[0] - p0[0]) / 6} ${p1[1] + (p2[1] - p0[1]) / 6} ${p2[0] - (p3[0] - p1[0]) / 6} ${p2[1] - (p3[1] - p1[1]) / 6} ${p2[0]} ${p2[1]}`;
  }
  return d;
}
