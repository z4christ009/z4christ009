# optical-link

A screen-to-camera data link. One device displays a colour grid at 60 fps, another
films it, and a file crosses the gap with no Wi-Fi, no Bluetooth and no network of
any kind.

The codec core is Rust; the phone app is that same core compiled to WebAssembly
behind a small web page. No App Store, no Xcode — open a URL on both phones.

## Run it on two phones

```
python3 web/serve.py
```

It prints a `https://<your-lan-ip>:8443/` URL. Open it on **both** phones, accept
the certificate warning, then **Send** on one and **Receive** on the other.

Three things about that, all load-bearing:

- **HTTPS is not optional.** `getUserMedia` only works in a secure context, and a
  plain `http://` LAN address never qualifies — the camera is refused on iOS and
  Android regardless of what permissions you grant. `serve.py` generates a
  self-signed cert for exactly this reason. The browser warning is that cert;
  accept it and continue.
- **Both phones need to be propped.** At ~6 px/cell, handheld does not hold focus.
- **Sender screen to maximum brightness.**

`web/optical_link.wasm` is committed so the app runs with only Python installed.
After changing anything under `src/`, re-stage it with `./build-web.sh`.

### What the app does

Send mode takes any file — the picker accepts video, so you can pull a clip
straight from the camera roll — and loops it out as colour frames forever, since
there is no back-channel telling it when to stop. Receive mode runs the camera,
shows a live HUD (capture/decode FPS, lock rate, frame yield, goodput, symbol
progress), and on completion offers the file for download and plays it inline if
it is video or an image. Both ends expose the preset selector; the receiver also
picks capture resolution, because that is the constraint that actually binds at
60 fps.

Everything heavy — cell rendering, marker localisation, sampling, Reed-Solomon,
RaptorQ — runs in wasm. Decoding a 160×160 grid in JavaScript would cap the link
an order of magnitude below what the codec can carry.

---

Below is **Phase 1**: the codec core plus a synthetic camera, so the stack can be
measured and tuned without any hardware in the loop.

## Why not just animate QR codes

A QR version 40 at error-correction level L carries 2953 bytes, so a 60 fps stream
of them tops out near 1.1 Mbps. That is fine for a text file and hopeless for
video — a one-minute 1080p clip would take about ten minutes.

Four changes buy roughly 3–5x, all of which mean leaving the QR spec behind:

| Change | Gain | Why |
|---|---|---|
| Colour cells, 3 bits each | ~4x | QR yields ~0.75 bits/module after EC and overhead |
| Denser grid | ~2x | QR's module sizing is conservative by spec |
| Parity matched to the channel | ~1.2x | screen-to-camera has no coffee stains to survive |
| Rateless outer code | — | turns frame loss into latency instead of failure |

## Coding stack

Three layers, each doing something the others cannot:

1. **Colour cells** (`palette.rs`) — 3 bits per cell carried as three *independent*
   binary channels, one per RGB primary. Classification collapses to three separate
   one-bit thresholds instead of a nearest-neighbour search in 3-space: trivially
   parallel on a GPU, and each channel gets its own threshold. That last part
   matters, because displays and sensors disagree about colour, and they disagree
   differently per channel. A calibration strip in every frame re-measures the
   levels, so auto white balance and gamma drift stop being fatal.

2. **Reed-Solomon** (`ecc.rs`) — corrects misread cells *within* a frame. Colour
   coding produces symbol errors, not clean erasures, so the fountain layer alone
   is not enough. Codewords are interleaved across the grid, so glare — which hits
   physically contiguous cells — damages many codewords lightly instead of blowing
   through one codeword's budget.

3. **RaptorQ** (`fountain.rs`) — corrects whole frames going missing, with no
   back-channel. The sender never learns what the camera dropped; it just emits an
   endless stream of distinct symbols, and the receiver reconstructs once it holds
   any ~1.05x the source count. Dropped frames cost time, never correctness.

## Measured results

1 MB payload, balanced preset (160×160 cells), 60 fps, via `simulate`:

| Channel | px/cell | Frame yield | Goodput | vs budget |
|---|---|---|---|---|
| codec only (no optics) | — | 100% | 3.66 Mbps | 100% |
| `good` (tripod, decent camera) | 8.4 | 95.6% | 3.43 Mbps | 94% |
| `realistic` (propped phone on a desk) | 6.8 | 93.0% | 3.14 Mbps | 86% |
| `harsh` (handheld, dim room, cheap webcam) | 5.6 | 17.3% | 0.53 Mbps | 15% |

All four reconstruct **byte-identical**. That is the property worth protecting: the
link degrades in throughput, never in correctness. `harsh` still completes — it just
takes 15 seconds instead of 2.5.

The `fast` preset (192×192) reaches 4.65 Mbps on the realistic channel.

### What the numbers say

Under `good` and `realistic`, Reed-Solomon corrected **zero** shards — every
uncorrectable shard came from a torn frame, not from a misread cell. So in those
conditions the link is **loss-limited, not error-limited**, and the parity is close
to pure overhead. Raising capture resolution from 1200 to 2400 px did not help
either, for the same reason.

Which tuning lever to reach for depends on the regime:

- **Loss-limited** (yield below ~95%, few RS corrections): parity is wasted. Cut it
  or push the grid denser. Frames are being lost whole, so attack tearing.
- **Error-limited** (many RS corrections): raise parity, enlarge cells, or improve
  focus and lighting. `harsh` is the only preset that gets here, correcting ~13k
  shards.

## Caveat on the channel model

The synthetic channel's degradation parameters are set by judgement, not calibrated
against real captures. Real optics at 5.6 px/cell are very likely worse than modelled
— the fact that colour classification stayed error-free at that density is the
result I trust least here.

So: Phase 1 establishes that the codec is **correct** and that the plumbing holds up
under loss, warping, blur, noise, crosstalk and rolling-shutter tearing. The absolute
Mbps figures are provisional until a real camera is in the loop. Phase 2 is what
turns them into measurements.

## Running it

```
cargo test --release                     # 26 tests, codec + geometry + end-to-end
node web/loopback-test.mjs               # wasm ABI, all three presets
node web/browser-test.mjs                # real canvas path, headless Chromium
cargo run --release --bin simulate -- --channel realistic --size 1000000
```

The three test layers exist because they fail differently. `cargo test` covers the
codec. The Node loopback covers the wasm boundary — pointer handling, memory-growth
invalidation, the stats layout. The browser test covers what neither can see: the
canvas resampling cell edges into mush. That last one is why the sender snaps its
canvas to an integer number of device pixels per cell instead of just filling the
viewport.

Useful flags:

```
--preset <robust|balanced|fast>       160×160 by default
--channel <good|realistic|harsh|perfect>
--capture <px>                        camera resolution
--file <path>                         send a real file
--perfect-geometry                    skip optics, isolate the codec
```

`--perfect-geometry` exists so codec bugs and geometry bugs never have to be debugged
at the same time. When something breaks, run it first: if it still fails, the problem
is in the coding stack, not the optics.

## Layout

```
src/
  config.rs     grid geometry and rate budget — tune throughput here
  palette.rs    colour coding, bit packing, per-channel calibration
  geometry.rs   rendering, marker localisation, homography, cell sampling
  ecc.rs        interleaved Reed-Solomon
  frame.rs      header, CRC32
  fountain.rs   RaptorQ wrapper
  sender.rs     object in, cell symbols out
  receiver.rs   camera frames in, object out
  channel.rs    synthetic camera: warp, blur, noise, crosstalk, tearing, drops
  wasm.rs       raw C ABI for the browser build
web/
  index.html    two modes, minimal chrome
  app.js        camera, canvas, wasm glue — no codec logic
  serve.py      HTTPS static server (required for camera access)
```

`wasm.rs` deliberately avoids wasm-bindgen. The surface is small enough that a
plain `extern "C"` boundary over linear memory is less machinery than a codegen
dependency, and it keeps everything crossing into JS visible in one file.

## What's next

**Phase 2 — laptop sender, webcam receiver.** Both ends on one machine so iteration
stays a single keystroke. This is what calibrates the channel model against reality.

**Phase 3 — phone receiver.** Locked-format 60 fps capture, cell classification in a
Metal or Vulkan compute shader. At 60 fps there are ~16 ms per frame, so the
per-cell work has to leave the CPU.

**Phase 4 — phone sender**, and it's phone-to-phone.

**Phase 5 — streaming.** Transcode to fit the measured link rate, chunk GOP-aligned,
play with a small jitter buffer. Above ~3 Mbps that is watchable 720p live, which is
a better demo than a progress bar and sidesteps the problem that an untranscoded
video file is simply a lot of bytes.

### Practical notes for when hardware shows up

- Clamp both devices. At 6 px/cell, handheld is hopeless.
- Sender screen at maximum brightness — at 60 fps the exposure budget is short.
- Lock focus and exposure before the transfer starts. Autofocus hunting mid-stream
  will wreck a run.
- Don't fight tearing first. A torn frame fails its CRC and the fountain code
  absorbs it. Only build tear detection if measured yield stays below ~60%.
