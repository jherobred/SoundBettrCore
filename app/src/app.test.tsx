import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import type { Device, Profile, Snapshot } from "./api";

const calls: [string, unknown][] = [];
let snap: Snapshot;
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args: unknown) => {
    calls.push([cmd, args]);
    if (cmd === "search_autoeq") return [];
    if (cmd === "hw_command") return { result: "confirmed" };
    return snap;
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: vi.fn() }));

import App from "./App";

const dev = (over: Partial<Device> = {}): Device => ({
  id: "bt:AA", key: "ep", kind: "bluetooth", name: "Jhems's R50i NC", hue: "cobalt", announce: true,
  profileId: "p1", lastSeen: Date.now(), isNew: false, hardware: null, ...over,
});
const prof = (over: Partial<Profile> = {}): Profile => ({
  id: "p1", name: "Harman fix", source: "autoeq", autoeqModel: null, gains: [2, 1, -4.2, -3.4, -1, 0, 3.2, 3.8, 1, 0],
  preamp: null, balance: 0, surround: false, crossfeed: false, hearing: null, ...over,
});
const base = (over: Partial<Snapshot> = {}): Snapshot => ({
  platform: "windows", devices: [dev()], profiles: [prof()],
  settings: { theme: "light", motion: true, voice: "af_heart", template: "Connected to {device}.", autoeqUpdated: null },
  conn: { state: "recognized", deviceId: "bt:AA" }, current: "bt:AA", applyError: null, apo: { installed: true, hooked: true },
  speaking: false, voiceError: null,
  voices: [["af_heart", "Heart"], ["af_bella", "Bella"], ["bf_emma", "Emma"], ["af_nicole", "Nicole"], ["am_michael", "Michael"], ["am_fenrir", "Fenrir"], ["bm_george", "George"], ["am_puck", "Puck"]].map(([id, name]) => ({ id, name, desc: "" })),
  hardware: null, hearing: null, abRound: null, autoeqEntries: 8850, ...over,
});

beforeEach(() => {
  calls.length = 0;
  snap = base();
  globalThis.ResizeObserver = class { observe() {} disconnect() {} } as unknown as typeof ResizeObserver;
  window.matchMedia = vi.fn(() => ({ matches: false, addEventListener() {}, removeEventListener() {} })) as unknown as typeof matchMedia;
});
afterEach(cleanup);

async function mount() {
  await act(async () => void render(<App />));
}
const nav = (name: string) => fireEvent.click(screen.getAllByRole("button").find((b) => b.classList.contains("nav-btn") && b.textContent === name)!);

test("theme and motion settings reach the document", async () => {
  snap = base({ settings: { ...base().settings, theme: "dark", motion: false } });
  await mount();
  expect(document.documentElement.dataset.theme).toBe("dark");
  expect(document.documentElement.dataset.motion).toBe("off");
});

test("nav has five buttons and marks only the current one", async () => {
  await mount();
  const btns = screen.getAllByRole("button").filter((b) => b.classList.contains("nav-btn"));
  expect(btns.map((b) => b.textContent)).toEqual(["Now", "Devices", "Tune", "Tests", "Settings"]);
  expect(btns.filter((b) => b.getAttribute("aria-current") === "page").length).toBe(1);
});

test("searching shows checking outputs", async () => {
  snap = base({ conn: { state: "searching" }, current: null });
  await mount();
  expect(screen.getByText("Checking outputs")).toBeTruthy();
});

test("recognized device card speaks when tapped", async () => {
  await mount();
  expect(screen.getByText("Recognized. Tap to hear it.")).toBeTruthy();
  fireEvent.click(screen.getByText("Recognized. Tap to hear it.").closest("button")!);
  expect(calls).toContainEqual(["announce", { deviceId: "bt:AA" }]);
});

test("new device shows the setup card", async () => {
  snap = base({ devices: [dev({ isNew: true })], conn: { state: "new", deviceId: "bt:AA" } });
  await mount();
  expect(screen.getByText("New output: Jhems's R50i NC")).toBeTruthy();
});

test("noise control appears only when the earbuds answered", async () => {
  await mount();
  expect(screen.queryByText("Noise control")).toBeNull();
});

test("choose lists the saved wired sets", async () => {
  snap = base({ devices: [dev({ id: "wired:w", kind: "wired", name: "Grey set" }), dev({ id: "wired:w#2", kind: "wired", name: "Red set" })], conn: { state: "choose", deviceIds: ["wired:w", "wired:w#2"] }, current: null });
  await mount();
  fireEvent.click(screen.getByRole("button", { name: "Red set" }));
  expect(calls).toContainEqual(["choose_device", { id: "wired:w#2" }]);
});

test("eq slider edits the profile and keeps the preamp honest", async () => {
  vi.useFakeTimers();
  await mount();
  nav("Tune");
  expect(screen.getByText("Preamp -3.8 dB, so the loudest boost cannot clip.")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("1kHz gain"), { target: { value: "6" } });
  await act(async () => void vi.advanceTimersByTime(150));
  const sent = calls.find((c) => c[0] === "update_profile")![1] as { profile: Profile };
  expect(sent.profile.gains[5]).toBe(6);
  expect(sent.profile.name).toBe("Harman fix (edited)");
  vi.useRealTimers();
});

test("forget is disabled for the connected device", async () => {
  await mount();
  nav("Devices");
  expect((screen.getAllByText("Forget").find((b) => b.tagName === "BUTTON") as HTMLButtonElement).disabled).toBe(true);
});

test("settings offers eight voices and previews one", async () => {
  await mount();
  nav("Settings");
  expect(screen.getAllByRole("button", { name: /^Preview / }).length).toBe(8);
  fireEvent.click(screen.getByRole("button", { name: "Preview Emma" }));
  expect(calls).toContainEqual(["preview_voice", { voice: "bf_emma", text: "Connected to Jhems's R50i NC." }]);
});
