// Headless browser test of the full JS path.
//
// The Node loopback checks the wasm ABI in isolation. This one drives the real
// browser pipeline: ImageData, putImageData, canvas upscaling, and a
// getImageData readback feeding the receiver. That covers the failure this
// project is most exposed to -- the canvas resampling cell edges into mush --
// which no amount of Rust-side testing would catch.
//
//   node web/browser-test.mjs

import { chromium } from "playwright";
import { spawn } from "node:child_process";
import { setTimeout as sleep } from "node:timers/promises";

const PORT = 8791;

const server = spawn("python3", ["-m", "http.server", String(PORT), "--bind", "127.0.0.1"], {
  cwd: new URL(".", import.meta.url).pathname,
  stdio: "ignore",
});
await sleep(700);

let code = 1;
let browser;
try {
  // The sandbox ships a Chromium that may not match the npm playwright build,
  // so point at it explicitly rather than triggering a download.
  browser = await chromium.launch({
    executablePath: process.env.CHROMIUM_PATH || "/opt/pw-browsers/chromium-1194/chrome-linux/chrome",
    args: ["--no-sandbox"],
  });
  const page = await browser.newPage();
  const errors = [];
  page.on("pageerror", (e) => errors.push("pageerror: " + String(e)));
  page.on("response", (r) => {
    if (r.status() >= 400) errors.push(`HTTP ${r.status()} ${r.url()}`);
  });

  await page.goto(`http://127.0.0.1:${PORT}/index.html`);
  await page.waitForFunction(() => window.__ready === true, { timeout: 15000 });
  console.log("wasm loaded, page clean");

  const result = await page.evaluate(async () => {
    const { W, copyIn, u8, u32 } = window.__ol;

    const N = 150_000;
    const payload = new Uint8Array(N);
    for (let i = 0; i < N; i++) payload[i] = (i * 97 + (i >> 8)) & 0xff;

    const nameBytes = new TextEncoder().encode("test.mp4");
    const dp = copyIn(payload);
    const np = copyIn(nameBytes);
    const tx = W.sender_new(1, dp, N, np, nameBytes.length);
    const rx = W.receiver_new(1);
    if (!tx || !rx) return { error: "handle allocation failed" };

    const grid = W.sender_grid(tx);
    const SCALE = 5; // px per cell at the "camera"

    const cell = document.createElement("canvas");
    cell.width = grid; cell.height = grid;
    const cctx = cell.getContext("2d");

    const big = document.createElement("canvas");
    big.width = grid * SCALE; big.height = grid * SCALE;
    const bctx = big.getContext("2d", { willReadFrequently: true });

    const idata = new ImageData(grid, grid);
    const framePtr = W.alloc(big.width * big.height * 4);
    const statsPtr = W.alloc(11 * 4);

    let done = false, frames = 0;
    for (let n = 0; n < 300 && !done; n++) {
      const p = W.sender_next_frame(tx);
      idata.data.set(u8().subarray(p, p + grid * grid * 4));
      cctx.putImageData(idata, 0, 0);

      bctx.imageSmoothingEnabled = false;
      bctx.drawImage(cell, 0, 0, big.width, big.height);

      const img = bctx.getImageData(0, 0, big.width, big.height);
      u8().set(img.data, framePtr);

      const st = W.receiver_push(rx, framePtr, big.width, big.height);
      frames++;
      if (st & 4) done = true;
    }

    W.receiver_stats(rx, statsPtr);
    const s = Array.from(u32().subarray(statsPtr >> 2, (statsPtr >> 2) + 11));

    if (!done) return { error: "incomplete", frames, stats: s };

    const len = W.receiver_result_len(rx);
    const ptr = W.receiver_result_ptr(rx);
    const out = u8().slice(ptr, ptr + len);

    let identical = len === N;
    if (identical) {
      for (let i = 0; i < N; i++) {
        if (out[i] !== payload[i]) { identical = false; break; }
      }
    }

    const nl = W.receiver_name_len(rx);
    const npr = W.receiver_name_ptr(rx);
    const name = new TextDecoder().decode(u8().slice(npr, npr + nl));

    W.sender_free(tx);
    W.receiver_free(rx);
    return { frames, len, identical, name, stats: s, grid, capture: big.width };
  });

  if (result.error) {
    console.error("FAIL:", result.error, JSON.stringify(result.stats));
    throw new Error(result.error);
  }

  const [seen, locked, valid, rejected] = result.stats;
  console.log(`grid ${result.grid}, capture ${result.capture}x${result.capture}`);
  console.log(`frames=${result.frames}  seen=${seen} locked=${locked} valid=${valid} rejected=${rejected}`);
  console.log(`recovered ${result.len} B as "${result.name}"`);

  if (errors.length) {
    console.error("page errors:\n" + errors.join("\n"));
    throw new Error("console/page errors present");
  }
  if (!result.identical) throw new Error("payload mismatch");
  if (result.name !== "test.mp4") throw new Error(`bad name: ${result.name}`);

  console.log("PASS: byte-identical through the real canvas path");
  code = 0;
} catch (e) {
  console.error("FAIL:", e.message);
} finally {
  await browser?.close();
  server.kill();
}

process.exit(code);
