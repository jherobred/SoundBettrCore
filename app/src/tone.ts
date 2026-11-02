let ctx: AudioContext | null = null;

/** A sine tone at `db` dBFS, panned from -1 (left) to 1 (right), with short fades so it never clicks. */
export function tone(freq: number, db: number, pan: number, secs = 0.9) {
  ctx ??= new AudioContext();
  const c = ctx, o = c.createOscillator(), g = c.createGain(), p = c.createStereoPanner();
  o.frequency.value = freq;
  p.pan.value = Math.max(-1, Math.min(1, pan));
  const peak = 10 ** (db / 20), t = c.currentTime;
  g.gain.setValueAtTime(0, t);
  g.gain.linearRampToValueAtTime(peak, t + 0.05);
  g.gain.setValueAtTime(peak, t + secs - 0.08);
  g.gain.linearRampToValueAtTime(0, t + secs);
  o.connect(g).connect(p).connect(c.destination);
  o.start(t);
  o.stop(t + secs);
}
