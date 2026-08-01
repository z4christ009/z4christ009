// Loopback test for the wasm ABI: sender -> RGBA frames -> receiver, no browser.
//
// This is the cheapest possible check that the JS boundary is wired correctly --
// pointer handling, memory-growth invalidation, stats layout and the result
// handoff. If this passes and the browser still fails, the bug is in the camera
// path, not the ABI.
//
//   node web/loopback-test.mjs

import { readFileSync } from "node:fs";

const WASM = "target/wasm32-unknown-unknown/release/optical_link.wasm";
const UPSCALE = 4; // pixels per cell, mimicking a camera with room to spare
const PAYLOAD = 200_000;
const PRESETS = [
  [0, "robust"],
  [1, "balanced"],
  [2, "fast"],
];

const { instance } = await WebAssembly.instantiate(readFileSync(WASM), {});
const w = instance.exports;

// Memory views must be rebuilt after any alloc: growing linear memory detaches
// every existing ArrayBuffer view.
const u8 = () => new Uint8Array(w.memory.buffer);
const u32 = () => new Uint32Array(w.memory.buffer);

function copyIn(bytes) {
  const ptr = w.alloc(bytes.length);
  u8().set(bytes, ptr);
  return ptr;
}

const payload = new Uint8Array(PAYLOAD);
for (let i = 0; i < payload.length; i++) payload[i] = (i * 97 + (i >> 8)) & 0xff;

function run(presetId, label) {
  const name = new TextEncoder().encode("clip.mp4");
  const dataPtr = copyIn(payload);
  const namePtr = copyIn(name);

  const tx = w.sender_new(presetId, dataPtr, payload.length, namePtr, name.length);
  const rx = w.receiver_new(presetId);
  w.dealloc(dataPtr, payload.length);
  w.dealloc(namePtr, name.length);
  if (!tx || !rx) throw new Error("handle allocation failed");

  const grid = w.sender_grid(tx);
  const capW = grid * UPSCALE;
  const frameRgba = w.alloc(capW * capW * 4);
  const statsPtr = w.alloc(11 * 4);

  let done = false;
  let frames = 0;

  for (let n = 0; n < 400 && !done; n++) {
    // Render at 1 px/cell, then nearest-neighbour upscale into the buffer the
    // receiver sees -- exactly what the canvas does on the phone.
    const srcPtr = w.sender_next_frame(tx);
    frames++;

    const mem = u8();
    for (let y = 0; y < capW; y++) {
      const sy = (y / UPSCALE) | 0;
      for (let x = 0; x < capW; x++) {
        const sx = (x / UPSCALE) | 0;
        const s = srcPtr + (sy * grid + sx) * 4;
        const d = frameRgba + (y * capW + x) * 4;
        mem[d] = mem[s];
        mem[d + 1] = mem[s + 1];
        mem[d + 2] = mem[s + 2];
        mem[d + 3] = 255;
      }
    }

    if (w.receiver_push(rx, frameRgba, capW, capW) & 4) done = true;
  }

  w.receiver_stats(rx, statsPtr);
  const s = Array.from(u32().subarray(statsPtr >> 2, (statsPtr >> 2) + 11));

  let ok = false;
  let gotName = "";
  let len = 0;

  if (done) {
    len = w.receiver_result_len(rx);
    const ptr = w.receiver_result_ptr(rx);
    const out = u8().slice(ptr, ptr + len);

    const nameLen = w.receiver_name_len(rx);
    gotName = new TextDecoder().decode(
      u8().slice(w.receiver_name_ptr(rx), w.receiver_name_ptr(rx) + nameLen)
    );

    ok = len === payload.length;
    if (ok) {
      for (let i = 0; i < len; i++) {
        if (out[i] !== payload[i]) { ok = false; break; }
      }
    }
    ok = ok && gotName === "clip.mp4";
  }

  console.log(
    `${label.padEnd(9)} grid=${String(grid).padStart(3)} cap=${capW}  ` +
    `frames=${String(frames).padStart(3)}  seen=${s[0]} locked=${s[1]} valid=${s[2]} rejected=${s[3]}  ` +
    `${done ? `${len} B "${gotName}"` : "INCOMPLETE"}  ${ok ? "PASS" : "FAIL"}`
  );

  w.sender_free(tx);
  w.receiver_free(rx);
  w.dealloc(frameRgba, capW * capW * 4);
  w.dealloc(statsPtr, 11 * 4);
  return ok;
}

console.log(`payload ${PAYLOAD} B, ${UPSCALE} px/cell\n`);
const allOk = PRESETS.map(([id, label]) => run(id, label)).every(Boolean);
console.log(allOk ? "\nPASS: all presets byte-identical" : "\nFAIL");
process.exit(allOk ? 0 : 1);
