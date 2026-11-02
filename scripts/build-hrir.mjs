// Splits SADIE II (Apache 2.0) stereo HRIRs for virtual speakers at +30 and -30 degrees into the four
// mono impulse responses EqualizerAPO's 2x2 convolution matrix uses.
// Usage: node scripts/build-hrir.mjs <folder with azi_30,0_ele_0,0.wav and azi_330,0_ele_0,0.wav>
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = process.argv[2];
const out = new URL("../app/src-tauri/resources/hrir/", import.meta.url);

function readWav(path) {
  const b = readFileSync(path);
  let off = 12, fmt, data;
  while (off < b.length) {
    const id = b.toString("ascii", off, off + 4), size = b.readUInt32LE(off + 4);
    if (id === "fmt ") fmt = { channels: b.readUInt16LE(off + 10), rate: b.readUInt32LE(off + 12), bits: b.readUInt16LE(off + 22) };
    if (id === "data") data = b.subarray(off + 8, off + 8 + size);
    off += 8 + size + (size % 2);
  }
  return { ...fmt, data };
}

function writeMono(name, w, ch) {
  const bytes = w.bits / 8, frames = w.data.length / (bytes * w.channels);
  const data = Buffer.alloc(frames * bytes);
  for (let i = 0; i < frames; i++) w.data.copy(data, i * bytes, (i * w.channels + ch) * bytes, (i * w.channels + ch + 1) * bytes);
  const h = Buffer.alloc(44);
  h.write("RIFF", 0); h.writeUInt32LE(36 + data.length, 4); h.write("WAVEfmt ", 8);
  h.writeUInt32LE(16, 16); h.writeUInt16LE(1, 20); h.writeUInt16LE(1, 22); h.writeUInt32LE(w.rate, 24);
  h.writeUInt32LE(w.rate * bytes, 28); h.writeUInt16LE(bytes, 32); h.writeUInt16LE(w.bits, 34);
  h.write("data", 36); h.writeUInt32LE(data.length, 40);
  writeFileSync(new URL(name, out), Buffer.concat([h, data]));
}

// SADIE azimuth runs counterclockwise: 30 is front-left, 330 is front-right. Channel 0 is the left ear.
const left = readWav(join(dir, "azi_30,0_ele_0,0.wav"));
const right = readWav(join(dir, "azi_330,0_ele_0,0.wav"));
if (left.channels !== 2 || right.channels !== 2) throw new Error("expected stereo HRIRs");
writeMono("l30_left.wav", left, 0);
writeMono("l30_right.wav", left, 1);
writeMono("r30_left.wav", right, 0);
writeMono("r30_right.wav", right, 1);
console.log(`4 HRIRs, ${left.rate} Hz, ${left.bits}-bit, ${left.data.length / (left.bits / 8) / 2} taps`);
