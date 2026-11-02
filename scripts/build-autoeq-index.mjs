// Builds app/src-tauri/resources/autoeq/index.json.gz from a local AutoEq clone (MIT, jaakkopasanen/AutoEq).
// Usage: node scripts/build-autoeq-index.mjs <path to AutoEq clone>
import { readdirSync, readFileSync, writeFileSync, statSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { gzipSync } from "node:zlib";

const root = process.argv[2];
if (!root) throw new Error("Pass the path to an AutoEq clone");
const KINDS = { PK: "PK", LSC: "LSC", LS: "LSC", HSC: "HSC", HS: "HSC" };

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) yield* walk(p);
    else if (name.endsWith("ParametricEQ.txt")) yield p;
  }
}

function parse(text) {
  let preamp = 0;
  const filters = [];
  for (const line of text.split(/\r?\n/)) {
    const t = line.trim().split(/\s+/);
    if (t[0] === "Preamp:") preamp = Number(t[1]);
    if (t[0] === "Filter" && t[2] === "ON" && KINDS[t[3]]) {
      const num = (k) => Number(t[t.indexOf(k) + 1]);
      filters.push({ kind: KINDS[t[3]], fc: num("Fc"), gain: num("Gain"), q: num("Q") });
    }
  }
  return { preamp, filters: filters.filter((f) => [f.fc, f.gain, f.q].every(Number.isFinite)) };
}

const entries = [];
for (const file of walk(join(root, "results"))) {
  const parts = relative(join(root, "results"), file).split(sep);
  if (parts.length < 3) continue;
  const model = parts.at(-2);
  const source = parts.slice(0, -2).join(" ");
  const { preamp, filters } = parse(readFileSync(file, "utf8"));
  if (filters.length) entries.push({ m: model, s: source, p: preamp, f: filters });
}
entries.sort((a, b) => a.m.localeCompare(b.m) || a.s.localeCompare(b.s));
const out = new URL("../app/src-tauri/resources/autoeq/index.json.gz", import.meta.url);
const gz = gzipSync(JSON.stringify(entries), { level: 9 });
writeFileSync(out, gz);
console.log(`${entries.length} entries, ${(gz.length / 1048576).toFixed(2)} MB`);
