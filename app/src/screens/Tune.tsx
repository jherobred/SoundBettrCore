import { useEffect, useRef, useState } from "react";
import { api, autoPreamp, balText, BANDS, fmtHz, type Hit, type Profile } from "../api";
import type { Ctx } from "../App";
import { kindIcon } from "../icons";
import { Curve } from "../parts";

function edited(p: Profile): Profile {
  if (p.source === "custom") return p;
  const name = p.source === "flat" ? "Custom" : p.name.endsWith(" (edited)") ? p.name : `${p.name} (edited)`;
  return { ...p, name, source: "custom" };
}

export default function TuneScreen(c: Ctx) {
  const d = c.current;
  const saved = c.profileOf(d);
  const [p, setP] = useState<Profile | null>(saved);
  const [err, setErr] = useState<string | null>(null);
  const timer = useRef<number>(undefined);
  useEffect(() => setP(saved), [saved?.id, saved?.name, saved?.source, saved?.gains.join(), saved?.balance, saved?.surround, saved?.crossfeed]);

  if (!d || !p) {
    return (
      <>
        <div className="page-head">
          <h1 className="page-title">Tune</h1>
        </div>
        <p className="empty">Connect headphones or speakers to tune them. Each device keeps its own sound.</p>
      </>
    );
  }

  // Sliders update the screen at once and save at most every 100 ms
  const change = (next: Profile, now = false) => {
    setP(next);
    window.clearTimeout(timer.current);
    const send = () => api.updateProfile(next).then(c.set, (e) => setErr(String(e)));
    if (now) send();
    else timer.current = window.setTimeout(send, 100);
  };
  const preamp = p.preamp ?? autoPreamp(p.gains);
  const android = c.snap.platform === "android";

  return (
    <>
      <div className="page-head">
        <h1 className="page-title">Tune</h1>
        <p>Changes save to {d.name} as you make them.</p>
      </div>
      {(err || c.snap.applyError) && <p className="note warn">{err ?? c.snap.applyError}</p>}
      <div className="stack" style={{ marginTop: 16 }}>
        <section className="card" aria-labelledby="tn-eq">
          <div className="profile">
            <div>
              <h2 id="tn-eq">Equalizer</h2>
              <div className="profile-name">{p.name}</div>
            </div>
            <span className="dev-chip">
              <b aria-hidden="true">{kindIcon(d.kind)}</b>
              {d.name}
            </span>
          </div>
          <Curve gains={p.gains} label={`EQ curve for ${d.name}`} />
          <div className="eq">
            {BANDS.map((f, i) => (
              <label className="band" key={f}>
                <output>{p.gains[i] > 0 ? `+${p.gains[i]}` : p.gains[i]}</output>
                <input
                  type="range"
                  min={-12}
                  max={12}
                  step={0.5}
                  value={p.gains[i]}
                  aria-label={`${fmtHz(f)}Hz gain`}
                  onChange={(e) => change(edited({ ...p, gains: p.gains.map((g, j) => (j === i ? Number(e.target.value) : g)) }))}
                />
                <span>{fmtHz(f)}</span>
              </label>
            ))}
          </div>
          <div className="btn-row" style={{ marginTop: 12 }}>
            <button className="btn" type="button" onClick={() => change({ ...p, gains: p.gains.map(() => 0), preamp: null, name: "Flat", source: "flat", autoeqModel: null }, true)}>
              Reset to flat
            </button>
          </div>
          <p className="muted" style={{ marginTop: 8 }}>
            {preamp < 0 ? `Preamp ${preamp} dB, so the loudest boost cannot clip.` : "No preamp needed. Nothing is boosted."}
          </p>
        </section>
        <div className="split two">
          <AutoEq c={c} deviceId={d.id} />
          <section className="card" aria-labelledby="tn-bal">
            <h2 id="tn-bal">Balance</h2>
            <div className="balance">
              <span>L</span>
              <input className="range" type="range" min={-6} max={6} step={0.5} value={p.balance} aria-label="Left and right balance" onChange={(e) => change({ ...p, balance: Number(e.target.value) })} />
              <span>R</span>
            </div>
            <p className="muted">{balText(p.balance)}</p>
            <div className="btn-row" style={{ marginTop: 8 }}>
              <button className="btn" type="button" onClick={() => c.go("tests")}>
                Check by ear
              </button>
            </div>
            <h3>Space</h3>
            <label className="switch-row">
              <span>
                <strong>Virtual surround</strong>
                <small>{android ? "Uses the phone's own surround effect where it allows apps to" : "Places two virtual speakers in front of you, through EqualizerAPO"}</small>
              </span>
              <input className="switch" type="checkbox" role="switch" checked={p.surround} onChange={(e) => change({ ...p, surround: e.target.checked }, true)} />
            </label>
            {!android && (
              <label className="switch-row">
                <span>
                  <strong>Crossfeed</strong>
                  <small>Softens hard left and right panning on headphones</small>
                </span>
                <input className="switch" type="checkbox" role="switch" checked={p.crossfeed} onChange={(e) => change({ ...p, crossfeed: e.target.checked }, true)} />
              </label>
            )}
          </section>
        </div>
      </div>
    </>
  );
}

function AutoEq({ c, deviceId }: { c: Ctx; deviceId: string }) {
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<Hit[]>([]);
  useEffect(() => {
    const t = window.setTimeout(() => api.searchAutoeq(q).then(setHits, () => setHits([])), 150);
    return () => window.clearTimeout(t);
  }, [q]);
  return (
    <section className="card" aria-labelledby="tn-ae">
      <h2 id="tn-ae">AutoEq match</h2>
      <p className="muted">Measured corrections for {c.snap.autoeqEntries.toLocaleString()} headphone measurements, bundled with the app.</p>
      <label className="sr" htmlFor="aeq">
        Search AutoEq by model
      </label>
      <input className="field" id="aeq" type="search" placeholder="Model name, like Soundcore P40i" value={q} onChange={(e) => setQ(e.target.value)} style={{ marginTop: 12 }} />
      <ul className="results">
        {q.trim() && hits.length === 0 && <li className="empty">No AutoEq measurement matches “{q}”. Try fewer words, or tune by hand above.</li>}
        {hits.map((h) => (
          <li key={h.index}>
            <button className="result" type="button" onClick={() => api.applyAutoeq(deviceId, h.index).then(c.set)}>
              <span>
                <strong>{h.model}</strong>
                <small>{h.source}</small>
              </span>
              <em>Apply</em>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
