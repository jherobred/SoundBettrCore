import { useEffect, useRef, useState } from "react";
import { api, balText, fmtHz, type AbResult, type HtStep } from "../api";
import type { Ctx } from "../App";
import { tone } from "../tone";

const HT_FREQS = [250, 500, 1000, 2000, 4000, 8000];

export default function TestsScreen(c: Ctx) {
  const d = c.current;
  return (
    <>
      <div className="page-head">
        <h1 className="page-title">Tests</h1>
        <p>Find the sound that fits your ears and this device. Set a comfortable volume first.</p>
      </div>
      {!d ? (
        <p className="empty">Connect headphones or speakers first. Results are saved to that device.</p>
      ) : (
        <div className="split wide">
          <Hearing c={c} />
          <div className="stack">
            <BalanceCheck c={c} />
            <BlindAb c={c} />
          </div>
        </div>
      )}
    </>
  );
}

function play(s: HtStep | null) {
  if (s?.step === "play") tone(s.freq, s.db, s.ear === "left" ? -1 : 1);
}

function Hearing({ c }: { c: Ctx }) {
  const d = c.current!;
  const [step, setStep] = useState<HtStep | null>(c.snap.hearing);
  const running = useRef(false);
  running.current = step?.step === "play";
  // Leaving the screen mid-test turns the device's EQ back on
  useEffect(() => () => void (running.current && api.hearingStop()), []);

  const answer = async (heard: boolean) => {
    const s = await api.hearingAnswer(heard);
    setStep(s);
    play(s);
  };

  return (
    <section className="card" aria-labelledby="ts-ht">
      <h2 id="ts-ht">Hearing test</h2>
      <div className="ht-stage">
        {!step && (
          <>
            <p className="muted">Short tones play in one ear at a time and get quieter. Answer whether you heard each one. About five minutes in a quiet room. Your EQ is switched off while the test runs.</p>
            <div className="btn-row">
              <button
                className="btn primary"
                type="button"
                onClick={async () => {
                  const s = await api.hearingStart();
                  setStep(s);
                  play(s);
                }}
              >
                Start the test
              </button>
            </div>
          </>
        )}
        {step?.step === "play" && (
          <>
            <div className="progress" aria-hidden="true">
              <i style={{ "--p": step.index / step.total } as React.CSSProperties} />
            </div>
            <div>
              <div className="ht-now">
                {step.ear === "left" ? "Left" : "Right"} ear, {fmtHz(step.freq)}Hz
              </div>
              <p className="muted">
                Frequency {step.index + 1} of {step.total}. Did you hear the tone?
              </p>
            </div>
            <div className="ht-big">
              <button className="btn primary" type="button" onClick={() => answer(true)}>
                I heard it
              </button>
              <button className="btn" type="button" onClick={() => answer(false)}>
                I did not
              </button>
            </div>
            <div className="btn-row">
              <button className="btn" type="button" onClick={() => play(step)}>
                Play again
              </button>
              <button className="btn" type="button" onClick={() => api.hearingStop().then((s) => (c.set(s), setStep(null)))}>
                Stop
              </button>
            </div>
          </>
        )}
        {step?.step === "done" && <Result left={step.left} right={step.right} c={c} onAgain={() => setStep(null)} deviceName={d.name} />}
      </div>
    </section>
  );
}

function Result({ left, right, c, onAgain, deviceName }: { left: number[]; right: number[]; c: Ctx; onAgain: () => void; deviceName: string }) {
  const y = (v: number) => 16 + (-v - 0) * 1.6;
  const x = (i: number) => 36 + i * 52;
  const line = (r: number[]) => r.map((v, i) => `${x(i)},${y(v)}`).join(" ");
  const [msg, setMsg] = useState<string | null>(null);
  return (
    <>
      <p className="muted">The quietest tone you heard, per ear, relative to this device. Lower on the chart is better hearing.</p>
      <svg className="ht-chart" viewBox="0 0 330 160" role="img" aria-label={`Hearing thresholds. Left: ${left.join(", ")} dB. Right: ${right.join(", ")} dB.`}>
        {[-20, -40, -60].map((v) => (
          <g key={v}>
            <line className="grid" x1="28" x2="310" y1={y(v)} y2={y(v)} />
            <text x="0" y={y(v) + 4}>{v}</text>
          </g>
        ))}
        {HT_FREQS.map((f, i) => (
          <text key={f} x={x(i) - 10} y="156">
            {fmtHz(f)}
          </text>
        ))}
        <polyline className="l" points={line(left)} />
        <polyline className="r" points={line(right)} />
        <text x={x(5) + 6} y={y(left[5]) - 6}>Left</text>
        <text x={x(5) + 6} y={y(right[5]) + 14}>Right</text>
      </svg>
      <div className="btn-row">
        <button
          className="btn primary"
          type="button"
          onClick={() =>
            api.hearingApply(c.current!.id).then(
              (s) => (c.set(s), setMsg(`Saved as Hearing fit on ${deviceName}.`)),
              (e) => setMsg(String(e)),
            )
          }
        >
          Apply to {deviceName}
        </button>
        <button className="btn" type="button" onClick={onAgain}>
          Take it again
        </button>
      </div>
      {msg && <p className="saved">{msg}</p>}
    </>
  );
}

function BalanceCheck({ c }: { c: Ctx }) {
  const p = c.profileOf(c.current)!;
  const [msg, setMsg] = useState("");
  const nudge = async (dir: number) => {
    if (dir === 0) return setMsg(`Saved. ${balText(p.balance)}.`);
    const balance = Math.max(-6, Math.min(6, p.balance - dir * 0.5));
    const s = await api.updateProfile({ ...p, balance });
    c.set(s);
    setMsg(`${balText(balance)}. Listen again.`);
    tone(1000, -24, 0);
  };
  return (
    <section className="card" aria-labelledby="ts-bal">
      <h2 id="ts-bal">Balance check</h2>
      <p className="muted" style={{ marginTop: 4 }}>
        A centered tone plays through your balance setting. Say where you hear it.
      </p>
      <div className="btn-row" style={{ marginTop: 12 }}>
        <button className="btn" type="button" onClick={() => tone(1000, -24, 0)}>
          Play the tone
        </button>
      </div>
      <div className="btn-row three" style={{ marginTop: 8 }}>
        <button className="btn" type="button" onClick={() => nudge(-1)}>
          Leans left
        </button>
        <button className="btn" type="button" onClick={() => nudge(0)}>
          Centered
        </button>
        <button className="btn" type="button" onClick={() => nudge(1)}>
          Leans right
        </button>
      </div>
      <p className="saved" aria-live="polite">
        {msg}
      </p>
    </section>
  );
}

function BlindAb({ c }: { c: Ctx }) {
  const d = c.current!;
  const own = c.profileOf(d)!;
  const choices = c.snap.profiles;
  const [a, setA] = useState(own.id);
  const [b, setB] = useState(choices.find((p) => p.id !== own.id)?.id ?? "");
  const [round, setRound] = useState<number | null>(c.snap.abRound);
  const [on, setOn] = useState<"x" | "y">("x");
  const [result, setResult] = useState<AbResult | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const label = (id: string) => {
    const p = c.snap.profiles.find((x) => x.id === id);
    const owner = c.snap.devices.find((x) => x.profileId === id);
    return p ? (owner && owner.id !== d.id ? `${p.name} (${owner.name})` : p.name) : id;
  };
  useEffect(() => () => void (round !== null && !result && api.abCancel()), []);

  if (choices.length < 2) return (
    <section className="card" aria-labelledby="ts-ab">
      <h2 id="ts-ab">Blind A/B</h2>
      <p className="muted">You need two sound profiles to compare. Try an AutoEq match or the hearing test first.</p>
    </section>
  );

  return (
    <section className="card" aria-labelledby="ts-ab">
      <h2 id="ts-ab">Blind A/B</h2>
      <div style={{ display: "grid", gap: 12, marginTop: 8 }}>
        {result ? (
          <>
            <div className="ab-round">You picked {label(result.winner)}</div>
            <p className="muted">
              It won {result.wins} of {result.rounds} rounds.
            </p>
            <div className="btn-row">
              <button className="btn primary" type="button" onClick={() => api.useProfile(d.id, result.winner).then((s) => (c.set(s), setResult(null), setRound(null)))}>
                Use it on {d.name}
              </button>
              <button className="btn" type="button" onClick={() => (setResult(null), setRound(null))}>
                Done
              </button>
            </div>
          </>
        ) : round === null ? (
          <>
            <p className="muted">Play music in any app. X and Y switch your sound between two profiles without saying which is which.</p>
            <div className="ab-pick">
              <label>
                First
                <select className="select" value={a} onChange={(e) => setA(e.target.value)}>
                  {choices.map((p) => (
                    <option key={p.id} value={p.id}>{label(p.id)}</option>
                  ))}
                </select>
              </label>
              <label>
                Second
                <select className="select" value={b} onChange={(e) => setB(e.target.value)}>
                  {choices.map((p) => (
                    <option key={p.id} value={p.id}>{label(p.id)}</option>
                  ))}
                </select>
              </label>
            </div>
            <div className="btn-row">
              <button className="btn primary" type="button" disabled={a === b} onClick={() => api.abStart(d.id, a, b).then((s) => (c.set(s), setRound(0), setOn("x")), (e) => setErr(String(e)))}>
                Start, 6 rounds
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="ab-round">Round {round + 1} of 6</div>
            <div className="ab-pick">
              {(["x", "y"] as const).map((s) => (
                <button key={s} className={`btn ${on === s ? "ab-on" : ""}`} type="button" aria-pressed={on === s} onClick={() => api.abListen(s).then((r) => (c.set(r), setOn(s)))}>
                  Listen to {s.toUpperCase()}
                </button>
              ))}
            </div>
            <div className="ab-pick">
              {(["x", "y"] as const).map((s) => (
                <button
                  key={s}
                  className="btn primary"
                  type="button"
                  onClick={async () => {
                    const r = await api.abPick(s);
                    setOn("x");
                    if (r) setResult(r);
                    else setRound(round + 1);
                  }}
                >
                  I prefer {s.toUpperCase()}
                </button>
              ))}
            </div>
          </>
        )}
        {err && <p className="note warn">{err}</p>}
      </div>
    </section>
  );
}
