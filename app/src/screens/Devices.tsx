import { useEffect, useRef, useState } from "react";
import { api, fmtHz, HUES, HW_BANDS, type CommandResult, type Device } from "../api";
import type { Ctx } from "../App";
import { profileNote, ResultTag } from "../parts";

const KIND: Record<Device["kind"], string> = { bluetooth: "Bluetooth", usb: "USB audio", wired: "Headphone jack", builtin: "Built-in speakers" };

export default function DevicesScreen(c: Ctx) {
  const { snap } = c;
  const [sel, setSel] = useState(c.selected);
  const d = snap.devices.find((x) => x.id === sel) ?? snap.devices[0];
  return (
    <>
      <div className="page-head">
        <h1 className="page-title">Devices</h1>
        <p>Every output you have connected. Each one keeps its own name, color and sound.</p>
      </div>
      {!d ? (
        <p className="empty">No devices yet. Play something through headphones or speakers and they appear here.</p>
      ) : (
        <div className="split two">
          <section className="card" aria-labelledby="dv-list">
            <h2 id="dv-list">Remembered</h2>
            <ul className="devlist">
              {snap.devices.map((x) => (
                <li key={x.id}>
                  <button className="devrow" type="button" aria-current={x.id === d.id} onClick={() => setSel(x.id)}>
                    <span className="tile" style={{ background: `var(--${x.hue}-soft)`, color: `var(--${x.hue}-ink)` }} aria-hidden="true">
                      {x.name.replace(/^.*'s /, "")[0]}
                    </span>
                    <span>
                      <strong>{x.name}</strong>
                      <span>{x.isNew ? "Not set up" : c.profileOf(x)?.name}</span>
                    </span>
                    {x.isNew ? <span className="new-tag">New</span> : x.id === snap.current ? <span className="now">Connected</span> : <span />}
                  </button>
                </li>
              ))}
            </ul>
          </section>
          <Detail key={d.id} c={c} d={d} />
        </div>
      )}
    </>
  );
}

function Detail({ c, d }: { c: Ctx; d: Device }) {
  const { snap } = c;
  const isCurrent = d.id === snap.current;
  const [name, setName] = useState(d.name);
  const [err, setErr] = useState<string | null>(null);
  const dialog = useRef<HTMLDialogElement>(null);
  const p = c.profileOf(d);
  const save = (patch: Partial<Device>) =>
    api
      .updateDevice({ ...d, name, ...patch })
      .then(c.set)
      .catch((e) => setErr(String(e)));
  useEffect(() => setName(d.name), [d.name]);

  return (
    <section className="card detail" aria-labelledby="dv-detail" style={{ "--dev-soft": `var(--${d.hue}-soft)`, "--dev-ink": `var(--${d.hue}-ink)`, "--dev-mark": `var(--${d.hue}-mark)` } as React.CSSProperties}>
      <h2 id="dv-detail">{d.isNew ? "Set up this device" : d.name}</h2>
      <p className="muted">
        {KIND[d.kind]}. {d.isNew ? "Found just now. It plays flat until you finish." : isCurrent ? "Connected now." : ""}
      </p>
      <label className="field-label" htmlFor="rn">
        Name
      </label>
      <input
        className="field"
        id="rn"
        value={name}
        maxLength={60}
        onChange={(e) => setName(e.target.value)}
        onBlur={() => name !== d.name && save({})}
        onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
      />
      <div className="field-label" id="hl">
        Color
      </div>
      <div className="swatches" role="radiogroup" aria-labelledby="hl">
        {HUES.map(([h, label]) => (
          <button key={h} type="button" className="swatch" role="radio" aria-checked={d.hue === h} aria-label={label} style={{ background: `var(--${h}-mark)`, color: `var(--${h}-mark)` }} onClick={() => save({ hue: h })} />
        ))}
      </div>
      <div className="field-label">Sound</div>
      <p className="muted">{d.isNew ? "Find its model in AutoEq on the Tune screen, or keep it flat." : p ? `${p.name}. ${profileNote(p)}.` : ""}</p>
      <label className="switch-row">
        <span>
          <strong>Announce when connected</strong>
          <small>Speaks its name and sound profile</small>
        </span>
        <input className="switch" type="checkbox" role="switch" checked={d.announce} onChange={(e) => save({ announce: e.target.checked })} />
      </label>
      {isCurrent && d.hardware === "a3959" && snap.hardware && <Earbuds c={c} d={d} />}
      <div className="btn-row">
        {d.isNew && (
          <button className="btn primary" type="button" onClick={() => save({ isNew: false })}>
            Save this device
          </button>
        )}
        <button className="btn" type="button" disabled={!isCurrent} title={isCurrent ? undefined : "Connect this device to tune it"} onClick={() => c.go("tune")}>
          Tune
        </button>
        <button className="btn danger" type="button" disabled={isCurrent} title={isCurrent ? "Disconnect it first" : undefined} onClick={() => dialog.current?.showModal()}>
          Forget
        </button>
      </div>
      {!isCurrent && <p className="muted">Connect this device to tune it.</p>}
      {err && <p className="note warn">{err}</p>}
      <dialog ref={dialog} aria-labelledby="fg">
        <h2 id="fg">Forget this device?</h2>
        <p className="muted">{d.name} and its sound settings will be removed. It is set up again the next time it connects.</p>
        <div className="btn-row" style={{ marginTop: 16 }}>
          <button className="btn" type="button" onClick={() => dialog.current?.close()}>
            Keep it
          </button>
          <button
            className="btn danger"
            type="button"
            onClick={() => {
              dialog.current?.close();
              api.forgetDevice(d.id).then(c.set, (e) => setErr(String(e)));
            }}
          >
            Forget
          </button>
        </div>
      </dialog>
    </section>
  );
}

/** R50i NC hardware: the 8-band EQ stored in the earbuds, and gaming mode. */
function Earbuds({ c, d }: { c: Ctx; d: Device }) {
  const hw = c.snap.hardware!;
  const [eq, setEq] = useState(hw.eq_gains_tenths_db.map((v) => v / 10));
  const [r, setR] = useState<CommandResult | "sending" | null>(null);
  const send = async (cmd: Parameters<typeof api.hw>[1]) => {
    setR("sending");
    setR(await api.hw(d.id, cmd).catch((e) => ({ result: "failed" as const, reason: String(e) })));
  };
  return (
    <>
      <h3>Earbuds</h3>
      <p className="muted">
        Firmware {hw.firmware_left}. This EQ is stored in the earbuds, so it follows them to every phone and PC.
      </p>
      <div className="eq" style={{ gridTemplateColumns: "repeat(8, minmax(0, 1fr))" }}>
        {HW_BANDS.map((f, i) => (
          <label className="band" key={f}>
            <output>{eq[i] > 0 ? `+${eq[i]}` : eq[i]}</output>
            <input type="range" min={-12} max={12} step={0.5} value={eq[i]} aria-label={`Earbud ${fmtHz(f)}Hz gain`} onChange={(e) => setEq(eq.map((v, j) => (j === i ? Number(e.target.value) : v)))} />
            <span>{fmtHz(f)}</span>
          </label>
        ))}
      </div>
      <div className="btn-row">
        <button className="btn primary" type="button" onClick={() => send({ cmd: "eq", gains_tenths_db: eq.map((v) => Math.round(v * 10)) })}>
          Send EQ to the earbuds
        </button>
      </div>
      {hw.gaming_mode !== null && (
        <label className="switch-row">
          <span>
            <strong>Gaming mode</strong>
            <small>Lower latency on the earbuds</small>
          </span>
          <input className="switch" type="checkbox" role="switch" checked={hw.gaming_mode} onChange={(e) => send({ cmd: "gaming", on: e.target.checked })} />
        </label>
      )}
      <ResultTag r={r} />
    </>
  );
}
