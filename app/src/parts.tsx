import { useLayoutEffect, useRef } from "react";
import { gainsPath, type CommandResult, type Profile } from "./api";

export function Seg<T extends string>({ label, options, value, onChange }: { label: string; options: [T, string][]; value: T; onChange: (v: T) => void }) {
  const box = useRef<HTMLDivElement>(null);
  const ind = useRef<HTMLSpanElement>(null);
  useLayoutEffect(() => {
    const place = () => {
      const on = box.current?.querySelector<HTMLElement>('[aria-checked="true"]');
      if (!on || !ind.current) return;
      ind.current.style.width = `${on.offsetWidth}px`;
      ind.current.style.transform = `translateX(${on.offsetLeft - 4}px)`;
    };
    place();
    const ro = new ResizeObserver(place);
    ro.observe(box.current!);
    return () => ro.disconnect();
  }, [value]);
  return (
    <div className="seg" role="radiogroup" aria-label={label} ref={box} style={{ gridTemplateColumns: `repeat(${options.length}, 1fr)` }}>
      <span className="seg-ind" aria-hidden="true" ref={ind} />
      {options.map(([v, l]) => (
        <button key={v} type="button" role="radio" aria-checked={v === value} onClick={() => onChange(v)}>
          {l}
        </button>
      ))}
    </div>
  );
}

export function Curve({ gains, label }: { gains: number[]; label: string }) {
  return (
    <>
      <svg className="curve" viewBox="0 0 300 96" preserveAspectRatio="none" role="img" aria-label={label}>
        <line className="zero" x1="0" y1="48" x2="300" y2="48" />
        <path className="line" pathLength={1} d={gainsPath(gains)} />
      </svg>
      <div className="axis" aria-hidden="true">
        <span>31 Hz</span>
        <span>1 kHz</span>
        <span>16 kHz</span>
      </div>
    </>
  );
}

export function profileNote(p: Profile): string {
  switch (p.source) {
    case "flat":
      return "No correction yet";
    case "autoeq":
      return "AutoEq measurement";
    case "hearing":
      return "From your hearing test";
    case "ab":
      return "Chosen in a blind A/B";
    default:
      return "Tuned by hand";
  }
}

export function ResultTag({ r }: { r: CommandResult | "sending" | null }) {
  if (!r) return <p className="result-tag" aria-live="polite" />;
  const [cls, text] =
    r === "sending"
      ? ["", "Sending to the earbuds"]
      : r.result === "confirmed"
        ? ["ok", "Confirmed by the earbuds"]
        : r.result === "noReply"
          ? ["bad", "Sent, but the earbuds did not answer"]
          : ["bad", r.reason];
  return (
    <p className={`result-tag ${cls}`} aria-live="polite">
      {text}
    </p>
  );
}

/** Calls a command and swallows nothing: errors show where the user is looking. */
export async function run<T>(p: Promise<T>, onError: (e: string) => void): Promise<T | undefined> {
  try {
    return await p;
  } catch (e) {
    onError(String(e));
  }
}
