import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api, type Settings } from "../api";
import type { Ctx } from "../App";
import { Seg } from "../parts";

export default function SettingsScreen(c: Ctx) {
  const s = c.snap.settings;
  const [template, setTemplate] = useState(s.template);
  const [msg, setMsg] = useState<Record<string, string>>({});
  const [busy, setBusy] = useState(false);
  const set = (patch: Partial<Settings>) => api.setSettings({ ...s, ...patch }).then(c.set);
  const say = (k: string, v: string) => setMsg((m) => ({ ...m, [k]: v }));
  const sample = c.current ? `Connected to ${c.current.name}.` : "Connected to your headphones.";

  return (
    <>
      <h1 className="page-title">Settings</h1>
      <section className="set-row" aria-labelledby="s-look">
        <div className="set-label">
          <h2 id="s-look">Appearance</h2>
          <p>System follows the Windows or Android setting.</p>
        </div>
        <div className="set-panel">
          <Seg label="Theme" options={[["light", "Light"], ["dark", "Dark"], ["system", "System"]]} value={s.theme} onChange={(theme) => set({ theme })} />
          <label className="switch-row">
            <span>
              <strong>Motion</strong>
              <small>Connect animation and screen transitions</small>
            </span>
            <input className="switch" type="checkbox" role="switch" checked={s.motion} onChange={(e) => set({ motion: e.target.checked })} />
          </label>
        </div>
      </section>

      <section className="set-row" aria-labelledby="s-voice">
        <div className="set-label">
          <h2 id="s-voice">Announcement voice</h2>
          <p>Kokoro voices, built into the app. They run on this device, offline.</p>
        </div>
        <div className="set-panel">
          {c.snap.voiceError && <p className="note warn">The voice could not load: {c.snap.voiceError}</p>}
          <ul className="voices">
            {c.snap.voices.map((v) => (
              <li className="voice" key={v.id} aria-current={v.id === s.voice}>
                <button className="voice-pick" type="button" onClick={() => set({ voice: v.id })}>
                  <strong>{v.name}</strong>
                  <span>{v.desc}</span>
                </button>
                <button className="play-btn" type="button" aria-label={`Preview ${v.name}`} disabled={c.snap.speaking} onClick={() => api.previewVoice(v.id, sample)}>
                  Play
                </button>
              </li>
            ))}
          </ul>
        </div>
      </section>

      <section className="set-row" aria-labelledby="s-words">
        <div className="set-label">
          <h2 id="s-words">What it says</h2>
          <p>{"{device}, {profile} and {battery} fill in when a device connects."}</p>
        </div>
        <div className="set-panel">
          <label className="field-label" htmlFor="words">
            Announcement
          </label>
          <input className="field" id="words" value={template} maxLength={200} onChange={(e) => setTemplate(e.target.value)} onBlur={() => template !== s.template && set({ template })} />
          <div className="btn-row">
            <button
              className="link-btn"
              type="button"
              disabled={!c.current || c.snap.speaking}
              onClick={async () => {
                if (template !== s.template) await set({ template });
                if (c.current) api.announce(c.current.id);
              }}
            >
              Play it
            </button>
          </div>
          {!c.current && <p className="muted">Connect a device to hear it with real values.</p>}
        </div>
      </section>

      <section className="set-row" aria-labelledby="s-aeq">
        <div className="set-label">
          <h2 id="s-aeq">AutoEq database</h2>
          <p>The app's only internet use, and only when you press Update.</p>
        </div>
        <div className="set-panel">
          <p className="muted">
            {c.snap.autoeqEntries.toLocaleString()} measurements. {s.autoeqUpdated ? `Updated ${s.autoeqUpdated}.` : "The copy that came with the app."}
          </p>
          <div className="btn-row">
            <button
              className="btn"
              type="button"
              disabled={busy}
              onClick={async () => {
                setBusy(true);
                say("aeq", "Downloading");
                try {
                  c.set(await api.updateAutoeq());
                  say("aeq", "Updated.");
                } catch (e) {
                  say("aeq", String(e));
                }
                setBusy(false);
              }}
            >
              Update
            </button>
          </div>
          <p className="result-tag" aria-live="polite">{msg.aeq}</p>
        </div>
      </section>

      <section className="set-row" aria-labelledby="s-prof">
        <div className="set-label">
          <h2 id="s-prof">Devices and profiles</h2>
          <p>Move your devices and their sound to another PC or phone.</p>
        </div>
        <div className="set-panel">
          <div className="btn-row">
            <button
              className="btn"
              type="button"
              onClick={async () => {
                const path = await save({ defaultPath: "soundbettrcore-devices.json", filters: [{ name: "JSON", extensions: ["json"] }] });
                if (path) api.exportStore(path).then(() => say("prof", "Saved."), (e) => say("prof", String(e)));
              }}
            >
              Export
            </button>
            <button
              className="btn"
              type="button"
              onClick={async () => {
                const path = await open({ filters: [{ name: "JSON", extensions: ["json"] }] });
                if (typeof path === "string") api.importStore(path).then((n) => (c.set(n), say("prof", "Imported.")), (e) => say("prof", String(e)));
              }}
            >
              Import
            </button>
          </div>
          <p className="result-tag" aria-live="polite">{msg.prof}</p>
        </div>
      </section>

      <section className="set-row" aria-labelledby="s-credits">
        <div className="set-label">
          <h2 id="s-credits">Credits</h2>
          <p>Open work this app is built on.</p>
        </div>
        <div className="set-panel">
          <p className="muted">
            Voice: Kokoro 82M by hexgrad, run with sherpa-onnx (both Apache 2.0). Headphone corrections: AutoEq by Jaakko Pasanen (MIT). Surround: SADIE II head-related impulse responses, University of York (Apache 2.0).
          </p>
        </div>
      </section>
    </>
  );
}
