import { useState } from "react";
import { api, balText, type Ambient, type CommandResult, type Device, type Kind } from "../api";
import type { Ctx } from "../App";
import { kindIcon, Buds } from "../icons";
import { Curve, profileNote, ResultTag, Seg } from "../parts";

const ROUTE: Record<Kind, string> = {
  bluetooth: "Playing through Bluetooth",
  wired: "Playing through the headphone jack",
  usb: "Playing through USB",
  builtin: "Playing through the built-in speakers",
};

export default function Now(c: Ctx) {
  const { snap, current: d } = c;
  const conn = snap.conn;
  const ready = conn.state === "recognized" || conn.state === "new";
  const p = c.profileOf(d);
  const hw = ready ? snap.hardware : null;
  const [err, setErr] = useState<string | null>(null);

  let name = "Checking outputs";
  let state = "Bluetooth, USB and the headphone jack";
  if (ready && d) {
    name = d.name;
    state = snap.speaking ? "Speaking" : d.isNew ? "New device. Set it up to keep its sound." : "Recognized. Tap to hear it.";
  } else if (conn.state === "choose") {
    name = "Which wired set is this?";
    state = "The jack cannot tell them apart. Pick one below.";
  } else if (conn.state === "error") {
    name = "Could not switch the sound";
    state = conn.cause;
  }

  return (
    <>
      <p className="route">{ready && d ? ROUTE[d.kind] : "Looking for an output"}</p>
      <div className="cols">
        <div className="cols" style={{ gridTemplateColumns: "minmax(0, 1fr)" }}>
          <div className="device">
            <button className="device-btn" type="button" disabled={!ready || !d} onClick={() => d && api.announce(d.id).catch((e) => setErr(String(e)))}>
              <span className="mark" aria-hidden="true">
                <span className="ring" />
                <span className="ring" />
                <span className="ring" />
                <span className="core">{d && ready ? kindIcon(d.kind) : <Buds />}</span>
              </span>
              <span className="device-text" key={ready && d ? d.id : "none"}>
                <span className="device-name">{name}</span>
                <span className="device-state">
                  <span className="bars" aria-hidden="true">
                    <i />
                    <i />
                    <i />
                  </span>
                  <span aria-live="polite">{state}</span>
                </span>
              </span>
            </button>
            {ready && d && (
              <div className="battery">
                {hw ? (
                  (["Left", "Right"] as const).map((s) => {
                    const v = s === "Left" ? hw.left_battery_percent : hw.right_battery_percent;
                    return (
                      <div className="batt" key={s}>
                        {s}
                        <b>{v}%</b>
                        <div className="meter">
                          <i style={{ "--v": v / 100 } as React.CSSProperties} />
                        </div>
                      </div>
                    );
                  })
                ) : (
                  <div className="battery-none">{batteryNote(d)}</div>
                )}
              </div>
            )}
          </div>

          {conn.state === "choose" && <Choose ids={conn.deviceIds} c={c} />}
          {(err || snap.voiceError) && <p className="note warn">{err ?? `The voice is off: ${snap.voiceError}`}</p>}
          <ApoNote c={c} />

          {ready && d?.isNew && (
            <div className="setup-card">
              <strong>New output: {d.name}</strong>
              <span>Name it and pick a sound. Until then it plays flat.</span>
              <div>
                <button className="btn primary" type="button" onClick={() => c.go("devices", d.id)}>
                  Set it up
                </button>
              </div>
            </div>
          )}

          {ready && d && p && (
            <section className="card" aria-labelledby="snd">
              <h2 id="snd">Sound for this device</h2>
              <div className="profile">
                <div>
                  <div className="profile-name">{p.name}</div>
                  <div className="profile-note">{profileNote(p)}</div>
                </div>
                <button className="link-btn" type="button" onClick={() => c.go("tune")}>
                  Tune
                </button>
              </div>
              <Curve gains={p.gains} label={`EQ curve for ${d.name}`} />
              <div className="balance">
                <span>L</span>
                <span className="track" aria-hidden="true">
                  <i style={{ "--bal": `${p.balance * 7}px` } as React.CSSProperties} />
                </span>
                <span>R</span>
              </div>
              <div className="balance-val">{balText(p.balance)}</div>
              {d.kind === "wired" && snap.platform === "android" && (
                <button className="btn" type="button" style={{ marginTop: 12 }} onClick={() => api.notThisSet().then(c.set)}>
                  This is a different wired set
                </button>
              )}
            </section>
          )}
        </div>

        <div className="cols" style={{ gridTemplateColumns: "minmax(0, 1fr)" }}>
          {hw && d && <NoiseControl c={c} device={d} />}
          <section className="card" aria-labelledby="rem">
            <h2 id="rem">Remembered devices</h2>
            {snap.devices.length === 0 ? (
              <p className="empty">Nothing yet. Every output you play through shows up here.</p>
            ) : (
              <ul className="devlist">
                {snap.devices.map((x) => (
                  <li key={x.id}>
                    <button className="devrow" type="button" onClick={() => c.go("devices", x.id)}>
                      <span className="tile" style={{ background: `var(--${x.hue}-soft)`, color: `var(--${x.hue}-ink)` }} aria-hidden="true">
                        {x.name.replace(/^.*'s /, "")[0]}
                      </span>
                      <span>
                        <strong>{x.name}</strong>
                        <span>{x.isNew ? "Not set up" : c.profileOf(x)?.name}</span>
                      </span>
                      {x.isNew ? <span className="new-tag">New</span> : x.id === snap.current ? <span className="now">Connected</span> : <span>{lastSeen(x.lastSeen)}</span>}
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      </div>
    </>
  );
}

function batteryNote(d: Device) {
  if (d.kind === "bluetooth") return "This device does not report its battery.";
  return d.kind === "builtin" ? "Built-in speakers." : d.kind === "usb" ? "USB audio. No battery to report." : "Headphone jack. No battery to report.";
}

function lastSeen(ms: number) {
  if (!ms) return "Never";
  const days = Math.floor((Date.now() - ms) / 86_400_000);
  if (days < 1) return "Today";
  if (days < 2) return "Yesterday";
  return new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

function Choose({ ids, c }: { ids: string[]; c: Ctx }) {
  return (
    <section className="card choose" aria-label="Pick the wired set">
      {ids.map((id) => {
        const d = c.snap.devices.find((x) => x.id === id);
        return (
          <button key={id} className="btn" type="button" onClick={() => api.chooseDevice(id).then(c.set)}>
            {d?.name ?? id}
          </button>
        );
      })}
      <button className="btn primary" type="button" onClick={() => api.chooseDevice(null).then(c.set)}>
        A different set
      </button>
    </section>
  );
}

function ApoNote({ c }: { c: Ctx }) {
  const { snap } = c;
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  if (!snap.applyError) return null;
  const canHook = snap.platform === "windows" && snap.apo?.installed && !snap.apo.hooked;
  return (
    <div className="note warn" role="status">
      {msg ?? snap.applyError}
      {canHook && (
        <div>
          <button
            className="btn primary"
            type="button"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              try {
                await api.connectApo();
                c.set(await api.snapshot());
              } catch (e) {
                setMsg(String(e));
              }
              setBusy(false);
            }}
          >
            Connect EqualizerAPO
          </button>
        </div>
      )}
    </div>
  );
}

const AMBIENT: [Ambient, string][] = [
  ["normal", "Normal"],
  ["transparency", "Aware"],
  ["noise_canceling", "ANC"],
];

function NoiseControl({ c, device }: { c: Ctx; device: Device }) {
  const hw = c.snap.hardware!;
  const [r, setR] = useState<CommandResult | "sending" | null>(null);
  return (
    <section className="card" aria-labelledby="nc">
      <h2 id="nc">Noise control</h2>
      <Seg
        label="Noise control mode"
        options={AMBIENT}
        value={hw.sound_modes.ambient_mode}
        onChange={async (v) => {
          setR("sending");
          setR(await api.hw(device.id, { cmd: "soundModes", modes: { ...hw.sound_modes, ambient_mode: v } }).catch((e) => ({ result: "failed" as const, reason: String(e) })));
        }}
      />
      <ResultTag r={r} />
    </section>
  );
}
