// Browser front end for optical-link.
//
// All the real work -- rendering cells, locating markers, sampling, Reed-Solomon,
// RaptorQ -- happens in wasm. This file only moves pixels between the camera, the
// canvas and linear memory. Doing the decode in JS would cap the link an order of
// magnitude below what the codec can carry.

const WASM_URL = "./optical_link.wasm";

let W = null; // wasm exports

// Linear memory can grow on any alloc, which detaches every existing view, so
// these are rebuilt per use rather than cached. It is cheap; a stale view is not.
const u8 = () => new Uint8Array(W.memory.buffer);
const u32 = () => new Uint32Array(W.memory.buffer);

const $ = (id) => document.getElementById(id);
const fmtBytes = (n) =>
  n >= 1 << 20 ? (n / (1 << 20)).toFixed(1) + " MB"
  : n >= 1024 ? (n / 1024).toFixed(0) + " KB"
  : n + " B";

function showError(e) {
  const box = $("err");
  box.textContent = String(e && e.message ? e.message : e);
  box.hidden = false;
}
function clearError() { $("err").hidden = true; }

async function initWasm() {
  const res = await fetch(WASM_URL);
  if (!res.ok) throw new Error(`cannot load ${WASM_URL}: HTTP ${res.status}`);
  const bytes = await res.arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  W = instance.exports;
}

function copyIn(bytes) {
  const ptr = W.alloc(bytes.length);
  u8().set(bytes, ptr);
  return ptr;
}

// ---------------------------------------------------------------------------
// Tabs
// ---------------------------------------------------------------------------

function selectTab(which) {
  const send = which === "send";
  $("tabSend").ariaSelected = String(send);
  $("tabRecv").ariaSelected = String(!send);
  $("send").hidden = !send;
  $("recv").hidden = send;
  if (send) rx.stop(); else tx.stop();
}
$("tabSend").onclick = () => selectTab("send");
$("tabRecv").onclick = () => selectTab("recv");

// ---------------------------------------------------------------------------
// Sender
// ---------------------------------------------------------------------------

const tx = {
  handle: 0,
  payload: null,
  name: "payload.bin",
  raf: 0,
  running: false,
  frames: 0,
  t0: 0,
  fpsT: 0,
  fpsN: 0,
  wakeLock: null,

  async start() {
    clearError();
    if (!this.payload) return;
    this.stop();

    const preset = Number($("txPreset").value);
    const nameBytes = new TextEncoder().encode(this.name);
    const dataPtr = copyIn(this.payload);
    const namePtr = copyIn(nameBytes);

    this.handle = W.sender_new(preset, dataPtr, this.payload.length, namePtr, nameBytes.length);
    W.dealloc(dataPtr, this.payload.length);
    W.dealloc(namePtr, nameBytes.length);
    if (!this.handle) return showError("sender allocation failed");

    this.grid = W.sender_grid(this.handle);
    const symbol = W.sender_symbol_size(this.handle);
    // No back-channel means no true progress bar; a "full pass" is the honest
    // unit -- source symbols plus RaptorQ's ~5% overhead.
    const pass = Math.ceil((this.payload.length / symbol) * 1.05);
    $("txPass").textContent = `${pass} fr · ${(pass / 60).toFixed(1)}s`;

    this.sizeCanvas();
    this.imageData = new ImageData(this.grid, this.grid);
    this.cellCanvas = new OffscreenCanvas(this.grid, this.grid);
    this.cellCtx = this.cellCanvas.getContext("2d");

    this.running = true;
    this.frames = 0;
    this.t0 = performance.now();
    this.fpsT = this.t0;
    this.fpsN = 0;
    $("txStart").disabled = true;
    $("txStop").disabled = false;
    $("txFull").disabled = false;

    try {
      this.wakeLock = await navigator.wakeLock?.request("screen");
    } catch { /* not fatal; the screen may just dim */ }

    this.loop();
  },

  // Snap the canvas to an exact integer number of device pixels per cell.
  // Fractional scaling makes the browser resample the grid, which smears cell
  // boundaries and costs the receiver far more than the few pixels it gains.
  sizeCanvas() {
    const canvas = $("txCanvas");
    const dpr = window.devicePixelRatio || 1;
    const avail = Math.min(window.innerWidth, window.innerHeight) - 8;
    const cell = Math.max(1, Math.floor((avail * dpr) / this.grid));
    const px = cell * this.grid;
    canvas.width = px;
    canvas.height = px;
    canvas.style.width = px / dpr + "px";
    canvas.style.height = px / dpr + "px";
    this.ctx = canvas.getContext("2d");
    this.ctx.imageSmoothingEnabled = false;
  },

  loop() {
    if (!this.running) return;
    this.raf = requestAnimationFrame(() => this.loop());

    const ptr = W.sender_next_frame(this.handle);
    if (!ptr) return;

    const n = this.grid * this.grid * 4;
    this.imageData.data.set(u8().subarray(ptr, ptr + n));
    this.cellCtx.putImageData(this.imageData, 0, 0);

    const c = $("txCanvas");
    this.ctx.imageSmoothingEnabled = false;
    this.ctx.drawImage(this.cellCanvas, 0, 0, c.width, c.height);

    this.frames++;
    this.fpsN++;
    const now = performance.now();
    if (now - this.fpsT >= 500) {
      $("txFps").textContent = (this.fpsN / ((now - this.fpsT) / 1000)).toFixed(0);
      $("txFrames").textContent = this.frames;
      this.fpsT = now;
      this.fpsN = 0;
    }
  },

  stop() {
    this.running = false;
    cancelAnimationFrame(this.raf);
    if (this.handle) { W.sender_free(this.handle); this.handle = 0; }
    this.wakeLock?.release?.().catch(() => {});
    this.wakeLock = null;
    $("txStart").disabled = !this.payload;
    $("txStop").disabled = true;
  },
};

$("file").onchange = async (e) => {
  const f = e.target.files?.[0];
  if (!f) return;
  clearError();
  tx.name = f.name;
  tx.payload = new Uint8Array(await f.arrayBuffer());
  $("txSize").textContent = fmtBytes(tx.payload.length);
  $("txStart").disabled = false;
};
$("txStart").onclick = () => tx.start().catch(showError);
$("txStop").onclick = () => tx.stop();
$("txFull").onclick = () => {
  const stage = $("txStage");
  if (document.fullscreenElement) document.exitFullscreen();
  else stage.requestFullscreen?.().catch(() => {});
};
window.addEventListener("resize", () => { if (tx.running) tx.sizeCanvas(); });

// ---------------------------------------------------------------------------
// Receiver
// ---------------------------------------------------------------------------

const rx = {
  handle: 0,
  stream: null,
  raf: 0,
  running: false,
  facing: "environment",
  statsPtr: 0,
  framePtr: 0,
  frameCap: 0,
  capN: 0, decN: 0, fpsT: 0,
  t0: 0,

  async start() {
    clearError();
    this.stop();

    const want = Number($("rxRes").value);
    const video = $("rxVideo");

    try {
      this.stream = await navigator.mediaDevices.getUserMedia({
        video: {
          facingMode: this.facing,
          width: { ideal: want },
          height: { ideal: Math.round((want * 9) / 16) },
          frameRate: { ideal: 60 },
        },
        audio: false,
      });
    } catch (e) {
      if (!window.isSecureContext) {
        throw new Error(
          "Camera needs a secure context. Open this page over HTTPS " +
          "(serve.py sets that up) — plain http:// on a LAN address will always be refused."
        );
      }
      throw e;
    }

    video.srcObject = this.stream;
    await video.play().catch(() => {});

    this.handle = W.receiver_new(Number($("rxPreset").value));
    if (!this.handle) return showError("receiver allocation failed");
    this.statsPtr = W.alloc(11 * 4);

    this.canvas = new OffscreenCanvas(2, 2);
    // getImageData every frame is the whole point of this context; without the
    // hint browsers keep it GPU-backed and each readback stalls the pipeline.
    this.ctx = this.canvas.getContext("2d", { willReadFrequently: true });

    this.running = true;
    this.capN = this.decN = 0;
    this.t0 = this.fpsT = performance.now();
    $("rxStart").disabled = true;
    $("rxStop").disabled = false;
    $("result").hidden = true;
    $("rxMsg").textContent = "Searching for the grid…";

    this.loop();
  },

  loop() {
    if (!this.running) return;
    this.raf = requestAnimationFrame(() => this.loop());

    const video = $("rxVideo");
    const vw = video.videoWidth, vh = video.videoHeight;
    if (!vw || !vh) return;

    if (this.canvas.width !== vw || this.canvas.height !== vh) {
      this.canvas.width = vw;
      this.canvas.height = vh;
    }
    this.ctx.drawImage(video, 0, 0, vw, vh);
    const img = this.ctx.getImageData(0, 0, vw, vh);
    this.capN++;

    const need = vw * vh * 4;
    if (this.frameCap < need) {
      if (this.framePtr) W.dealloc(this.framePtr, this.frameCap);
      this.framePtr = W.alloc(need);
      this.frameCap = need;
    }
    u8().set(img.data, this.framePtr);

    const status = W.receiver_push(this.handle, this.framePtr, vw, vh);
    this.decN++;

    if (status & 4) { this.finish(); return; }
    this.render();
  },

  render() {
    const now = performance.now();
    if (now - this.fpsT < 400) return;
    const dt = (now - this.fpsT) / 1000;

    W.receiver_stats(this.handle, this.statsPtr);
    const s = u32().subarray(this.statsPtr >> 2, (this.statsPtr >> 2) + 11);
    const [seen, locked, valid, rejected, eccFix, , , syms, total, needed] = s;

    $("rxCap").textContent = (this.capN / dt).toFixed(0);
    $("rxDec").textContent = (this.decN / dt).toFixed(0);
    this.capN = this.decN = 0;
    this.fpsT = now;

    const lockPct = seen ? (locked / seen) * 100 : 0;
    const yieldPct = seen ? (valid / seen) * 100 : 0;
    const lockEl = $("rxLock"), yieldEl = $("rxYield");
    lockEl.textContent = lockPct.toFixed(0) + "%";
    yieldEl.textContent = yieldPct.toFixed(0) + "%";
    lockEl.className = "v " + (lockPct > 70 ? "ok" : lockPct > 30 ? "hi" : "bad");
    yieldEl.className = "v " + (yieldPct > 60 ? "ok" : yieldPct > 20 ? "hi" : "bad");

    $("rxRej").textContent = rejected;
    $("rxEcc").textContent = eccFix;
    $("rxSyms").textContent = needed ? `${syms}/${needed}` : String(syms);

    const elapsed = (now - this.t0) / 1000;
    const gotBytes = syms * (needed && total ? total / needed : 0);
    $("rxRate").textContent = elapsed > 0.5
      ? ((gotBytes * 8) / elapsed / 1e6).toFixed(2) + " Mbps" : "—";

    $("rxBar").style.width = needed ? Math.min(100, (syms / needed) * 100) + "%" : "0%";

    if (total) {
      $("rxMsg").textContent = `Locked on ${fmtBytes(total)} — hold steady.`;
    } else if (locked) {
      $("rxMsg").textContent = "Grid found, waiting for a manifest frame…";
    }
  },

  finish() {
    const len = W.receiver_result_len(this.handle);
    const ptr = W.receiver_result_ptr(this.handle);
    const bytes = u8().slice(ptr, ptr + len);

    const nLen = W.receiver_name_len(this.handle);
    const nPtr = W.receiver_name_ptr(this.handle);
    const name = new TextDecoder().decode(u8().slice(nPtr, nPtr + nLen)) || "received.bin";

    const elapsed = (performance.now() - this.t0) / 1000;
    this.stopCamera();
    this.running = false;
    cancelAnimationFrame(this.raf);

    $("rxBar").style.width = "100%";
    $("rxMsg").textContent = "Transfer complete.";
    $("rxDone").textContent =
      `${name} · ${fmtBytes(len)} in ${elapsed.toFixed(1)}s ` +
      `(${((len * 8) / elapsed / 1e6).toFixed(2)} Mbps)`;

    const blob = new Blob([bytes], { type: mimeFor(name) });
    const url = URL.createObjectURL(blob);
    const dl = $("rxDownload");
    dl.href = url;
    dl.download = name;

    const prev = $("preview");
    prev.innerHTML = "";
    const type = mimeFor(name);
    if (type.startsWith("video/")) {
      const v = document.createElement("video");
      v.src = url; v.controls = true; v.playsInline = true;
      prev.appendChild(v);
    } else if (type.startsWith("image/")) {
      const i = document.createElement("img");
      i.src = url;
      prev.appendChild(i);
    }

    $("result").hidden = false;
    $("rxStart").disabled = false;
    $("rxStop").disabled = true;
  },

  stopCamera() {
    this.stream?.getTracks().forEach((t) => t.stop());
    this.stream = null;
    $("rxVideo").srcObject = null;
  },

  stop() {
    this.running = false;
    cancelAnimationFrame(this.raf);
    this.stopCamera();
    if (this.handle) { W.receiver_free(this.handle); this.handle = 0; }
    if (this.framePtr) { W.dealloc(this.framePtr, this.frameCap); this.framePtr = 0; this.frameCap = 0; }
    $("rxStart").disabled = false;
    $("rxStop").disabled = true;
  },
};

function mimeFor(name) {
  const ext = name.split(".").pop().toLowerCase();
  return {
    mp4: "video/mp4", mov: "video/quicktime", webm: "video/webm", m4v: "video/x-m4v",
    jpg: "image/jpeg", jpeg: "image/jpeg", png: "image/png", gif: "image/gif", webp: "image/webp",
    pdf: "application/pdf", txt: "text/plain", json: "application/json",
  }[ext] || "application/octet-stream";
}

$("rxStart").onclick = () => rx.start().catch(showError);
$("rxStop").onclick = () => rx.stop();
$("rxFlip").onclick = () => {
  rx.facing = rx.facing === "environment" ? "user" : "environment";
  if (rx.running) rx.start().catch(showError);
};

// ---------------------------------------------------------------------------

initWasm().then(
  () => { window.__ready = true; },
  (e) => showError(e)
);

// Exposed for the headless browser test in tests/browser-loopback.mjs.
window.__ol = { get W() { return W; }, tx, rx, copyIn, u8, u32 };
