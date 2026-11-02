import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { api, onSnapshot, onSpeaking, type Device, type Profile, type Snapshot } from "./api";
import * as Icon from "./icons";
import Now from "./screens/Now";
import DevicesScreen from "./screens/Devices";
import TuneScreen from "./screens/Tune";
import TestsScreen from "./screens/Tests";
import SettingsScreen from "./screens/Settings";

export type Route = "now" | "devices" | "tune" | "tests" | "settings";

export interface Ctx {
  snap: Snapshot;
  set: (s: Snapshot) => void;
  go: (r: Route, select?: string) => void;
  current: Device | null;
  profileOf: (d: Device | null) => Profile | null;
  selected: string | null;
}

const NAV: [Route, string, () => React.JSX.Element][] = [
  ["now", "Now", Icon.Buds],
  ["devices", "Devices", Icon.Devices],
  ["tune", "Tune", Icon.Tune],
  ["tests", "Tests", Icon.Tests],
  ["settings", "Settings", Icon.Gear],
];

function useTheme(snap: Snapshot | null) {
  useEffect(() => {
    if (!snap) return;
    const html = document.documentElement;
    const dark = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const t = snap.settings.theme;
      html.dataset.theme = t === "system" ? (dark.matches ? "dark" : "light") : t;
    };
    apply();
    html.dataset.motion = snap.settings.motion ? "on" : "off";
    dark.addEventListener("change", apply);
    return () => dark.removeEventListener("change", apply);
  }, [snap?.settings.theme, snap?.settings.motion]);
}

export default function App() {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  const [route, setRoute] = useState<Route>("now");
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.snapshot().then(setSnap, (e) => setError(String(e)));
    const un = [onSnapshot(setSnap), onSpeaking((on) => setSnap((s) => (s ? { ...s, speaking: on } : s)))];
    return () => un.forEach((p) => p.then((f) => f()));
  }, []);
  useTheme(snap);

  if (error) return <main className="screen"><p className="note warn">SoundBettrCore could not start: {error}</p></main>;
  if (!snap) return <main className="screen" aria-busy="true"><p className="empty">Starting</p></main>;

  const byId = (id: string | null) => snap.devices.find((d) => d.id === id) ?? null;
  const ctx: Ctx = {
    snap,
    set: setSnap,
    go: (r, select) => {
      if (select) setSelected(select);
      setRoute(r);
    },
    current: byId(snap.current),
    profileOf: (d) => (d ? snap.profiles.find((p) => p.id === d.profileId) ?? null : null),
    selected: selected ?? snap.current ?? snap.devices[0]?.id ?? null,
  };
  const ready = snap.conn.state === "recognized" || snap.conn.state === "new";
  const hue = ready && ctx.current ? ctx.current.hue : null;
  const style = hue ? ({ "--dev-soft": `var(--${hue}-soft)`, "--dev-ink": `var(--${hue}-ink)`, "--dev-mark": `var(--${hue}-mark)` } as React.CSSProperties) : undefined;

  return (
    <div className="app" style={style} data-state={ready ? "ready" : "searching"} data-speaking={snap.speaking}>
      <div className="shell">
        <Nav route={route} go={setRoute} />
        <main className="screen" key={route}>
          <div className="view on">
            {route === "now" && <Now {...ctx} />}
            {route === "devices" && <DevicesScreen {...ctx} />}
            {route === "tune" && <TuneScreen {...ctx} />}
            {route === "tests" && <TestsScreen {...ctx} />}
            {route === "settings" && <SettingsScreen {...ctx} />}
          </div>
        </main>
      </div>
    </div>
  );
}

function Nav({ route, go }: { route: Route; go: (r: Route) => void }) {
  const nav = useRef<HTMLElement>(null);
  const ind = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const place = () => {
      const btn = nav.current?.querySelector<HTMLElement>('[aria-current="page"]');
      if (!btn || !ind.current || !nav.current) return;
      const rail = nav.current.offsetWidth < 100;
      ind.current.style.width = rail ? "" : `${btn.offsetWidth}px`;
      ind.current.style.transform = rail ? `translateY(${btn.offsetTop}px)` : `translateX(${btn.offsetLeft}px)`;
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(nav.current!);
    return () => ro.disconnect();
  }, [route]);
  return (
    <nav className="nav" aria-label="Main" ref={nav}>
      <span className="brand" aria-hidden="true">SB</span>
      <span className="nav-ind" aria-hidden="true" ref={ind} />
      {NAV.map(([r, label, Ico]) => (
        <button key={r} className="nav-btn" type="button" aria-current={route === r ? "page" : undefined} onClick={() => go(r)}>
          <Ico />
          <span className="lbl">{label}</span>
        </button>
      ))}
    </nav>
  );
}
