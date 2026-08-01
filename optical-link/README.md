# optical-link

A screen-to-camera data link. One device displays a colour grid at 60 fps, another
films it, and a file crosses the gap with no Wi-Fi, no Bluetooth and no network of
any kind.

This is **Phase 1**: the codec core plus a synthetic camera, so the whole stack can
be measured and tuned without any hardware in the loop.

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
cargo test --release
cargo run --release --bin simulate -- --channel realistic --size 1000000
```

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
config.rs     grid geometry and rate budget — tune throughput here
palette.rs    colour coding, bit packing, per-channel calibration
geometry.rs   rendering, marker localisation, homography, cell sampling
ecc.rs        interleaved Reed-Solomon
frame.rs      header, CRC32
fountain.rs   RaptorQ wrapper
sender.rs     object in, cell symbols out
receiver.rs   camera frames in, object out
channel.rs    synthetic camera: warp, blur, noise, crosstalk, tearing, drops
```

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
