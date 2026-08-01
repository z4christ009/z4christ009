//! A synthetic screen-to-camera channel.
//!
//! The point of this module is to make the hard part cheap to iterate on. Every
//! degradation a real capture suffers is here and independently dialable, so you
//! can find the exact parameter that breaks the link instead of waving a phone
//! around and guessing. Get the codec surviving realistic numbers here first;
//! only then go fight optics.

use crate::geometry::{Homography, Image};

// ---------------------------------------------------------------------------
// Deterministic PRNG -- reproducible runs matter more than statistical purity
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    /// Approximately standard normal (Irwin-Hall, n=4).
    pub fn gauss(&mut self) -> f64 {
        ((0..4).map(|_| self.unit()).sum::<f64>() - 2.0) * 1.732
    }
}

// ---------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct ChannelParams {
    /// Camera output resolution (square).
    pub capture: usize,
    /// Corner jitter as a fraction of frame size. 0.03 is a phone propped at a
    /// slight angle; 0.10 is sloppy handheld.
    pub warp: f64,
    /// Defocus radius in output pixels.
    pub blur: f64,
    /// Sensor noise standard deviation, in levels.
    pub noise: f64,
    /// Per-channel gain and offset -- auto white balance and black crush.
    pub gain: [f64; 3],
    pub offset: [f64; 3],
    /// Corner falloff, 0 = none, 1 = severe.
    pub vignette: f64,
    /// Inter-channel bleed. Real sensors have overlapping colour filter response
    /// and this is the term that actually limits how many colours you can use.
    pub crosstalk: f64,
    /// Probability a frame is lost outright (occlusion, exposure hiccup).
    pub drop_rate: f64,
    /// Probability of a rolling-shutter tear: the capture straddles a display
    /// refresh and the top of the frame belongs to the previous image.
    pub tear_rate: f64,
}

impl Default for ChannelParams {
    fn default() -> Self {
        Self::realistic()
    }
}

impl ChannelParams {
    /// A clean tripod-mounted capture with a decent camera.
    pub fn good() -> Self {
        Self {
            capture: 1400,
            warp: 0.015,
            blur: 0.6,
            noise: 3.0,
            gain: [0.98, 0.95, 0.90],
            offset: [4.0, 5.0, 8.0],
            vignette: 0.10,
            crosstalk: 0.04,
            drop_rate: 0.02,
            tear_rate: 0.05,
        }
    }

    /// What a propped-up phone on a desk actually delivers.
    pub fn realistic() -> Self {
        Self {
            capture: 1200,
            warp: 0.035,
            blur: 1.1,
            noise: 7.0,
            gain: [0.95, 0.88, 0.80],
            offset: [8.0, 10.0, 16.0],
            vignette: 0.22,
            crosstalk: 0.10,
            drop_rate: 0.06,
            tear_rate: 0.12,
        }
    }

    /// Handheld, dim room, cheap webcam. If it survives this, it will survive
    /// your desk.
    pub fn harsh() -> Self {
        Self {
            capture: 1000,
            warp: 0.07,
            blur: 2.0,
            noise: 14.0,
            gain: [0.88, 0.78, 0.66],
            offset: [16.0, 20.0, 30.0],
            vignette: 0.38,
            crosstalk: 0.18,
            drop_rate: 0.12,
            tear_rate: 0.22,
        }
    }
}

// ---------------------------------------------------------------------------
// The channel
// ---------------------------------------------------------------------------

pub struct Channel {
    pub params: ChannelParams,
    rng: Rng,
    prev: Option<Image>,
    pub dropped: u64,
    pub torn: u64,
}

impl Channel {
    pub fn new(params: ChannelParams, seed: u64) -> Self {
        Self {
            params,
            rng: Rng::new(seed),
            prev: None,
            dropped: 0,
            torn: 0,
        }
    }

    /// Capture one displayed frame. `None` means the camera got nothing usable.
    pub fn capture(&mut self, screen: &Image) -> Option<Image> {
        let p = self.params;

        let mut source = screen.clone();

        // Rolling shutter: the sensor scanned past a refresh boundary, so the top
        // band still holds the previous image. Nothing recovers this frame -- it
        // just fails its CRC and the fountain code eats the loss.
        if let Some(prev) = &self.prev {
            if self.rng.unit() < p.tear_rate {
                let split = (self.rng.range(0.15, 0.85) * screen.h as f64) as usize;
                let bytes = split * screen.w * 3;
                source.px[..bytes].copy_from_slice(&prev.px[..bytes]);
                self.torn += 1;
            }
        }
        self.prev = Some(screen.clone());

        if self.rng.unit() < p.drop_rate {
            self.dropped += 1;
            return None;
        }

        let n = p.capture;
        let mut buf = self.warp(&source, n);
        box_blur(&mut buf, n, n, p.blur);
        apply_crosstalk(&mut buf, p.crosstalk);
        self.shade(&mut buf, n);
        self.add_noise(&mut buf, p.noise);

        Some(to_image(&buf, n, n))
    }

    /// Backward-map every output pixel through a jittered homography.
    fn warp(&mut self, src: &Image, n: usize) -> Vec<f32> {
        let (sw, sh) = (src.w as f64, src.h as f64);
        let j = self.params.warp * n as f64;
        let margin = 0.05 * n as f64;

        let mut jit = |x: f64, y: f64| (x + self.rng.range(-j, j), y + self.rng.range(-j, j));

        // Where the screen's four corners land in the camera frame.
        let cam = [
            jit(margin, margin),
            jit(n as f64 - margin, margin),
            jit(margin, n as f64 - margin),
            jit(n as f64 - margin, n as f64 - margin),
        ];
        let scr = [
            (0.0, 0.0),
            (sw - 1.0, 0.0),
            (0.0, sh - 1.0),
            (sw - 1.0, sh - 1.0),
        ];

        // Solve camera -> screen so we can pull rather than push.
        let back = Homography::from_correspondences(&cam, &scr)
            .expect("jittered quad should stay non-degenerate");

        // Anything off-screen is desk, not paper.
        let mut buf = vec![28.0f32; n * n * 3];
        for y in 0..n {
            for x in 0..n {
                let (sx, sy) = back.apply(x as f64 + 0.5, y as f64 + 0.5);
                if sx < 0.0 || sy < 0.0 || sx > sw - 1.0 || sy > sh - 1.0 {
                    continue;
                }
                let px = src.sample_bilinear(sx, sy);
                let i = (y * n + x) * 3;
                buf[i..i + 3].copy_from_slice(&px);
            }
        }
        buf
    }

    fn shade(&mut self, buf: &mut [f32], n: usize) {
        let p = self.params;
        let center = n as f64 / 2.0;
        let max_r2 = 2.0 * center * center;

        for y in 0..n {
            for x in 0..n {
                let dx = x as f64 - center;
                let dy = y as f64 - center;
                let fall = 1.0 - p.vignette * ((dx * dx + dy * dy) / max_r2);
                let i = (y * n + x) * 3;
                for c in 0..3 {
                    let v = buf[i + c] as f64 * p.gain[c] * fall + p.offset[c];
                    buf[i + c] = v as f32;
                }
            }
        }
    }

    fn add_noise(&mut self, buf: &mut [f32], sigma: f64) {
        if sigma <= 0.0 {
            return;
        }
        for v in buf.iter_mut() {
            *v += (self.rng.gauss() * sigma) as f32;
        }
    }
}

// ---------------------------------------------------------------------------
// Pixel operations
// ---------------------------------------------------------------------------

/// Two box passes approximate a Gaussian well enough for defocus.
fn box_blur(buf: &mut [f32], w: usize, h: usize, radius: f64) {
    let r = radius.round() as usize;
    if r == 0 {
        return;
    }
    for _ in 0..2 {
        blur_axis(buf, w, h, r, true);
        blur_axis(buf, w, h, r, false);
    }
}

fn blur_axis(buf: &mut [f32], w: usize, h: usize, r: usize, horizontal: bool) {
    let (outer, inner) = if horizontal { (h, w) } else { (w, h) };
    let mut line = vec![0f32; inner * 3];

    for o in 0..outer {
        for i in 0..inner {
            let idx = if horizontal { o * w + i } else { i * w + o } * 3;
            line[i * 3..i * 3 + 3].copy_from_slice(&buf[idx..idx + 3]);
        }

        for i in 0..inner {
            let lo = i.saturating_sub(r);
            let hi = (i + r).min(inner - 1);
            let count = (hi - lo + 1) as f32;
            let mut acc = [0f32; 3];
            for k in lo..=hi {
                for c in 0..3 {
                    acc[c] += line[k * 3 + c];
                }
            }
            let idx = if horizontal { o * w + i } else { i * w + o } * 3;
            for c in 0..3 {
                buf[idx + c] = acc[c] / count;
            }
        }
    }
}

fn apply_crosstalk(buf: &mut [f32], k: f64) {
    if k <= 0.0 {
        return;
    }
    let k = k as f32;
    let keep = 1.0 - k;
    let half = k * 0.5;
    for px in buf.chunks_exact_mut(3) {
        let (r, g, b) = (px[0], px[1], px[2]);
        px[0] = keep * r + half * (g + b);
        px[1] = keep * g + half * (r + b);
        px[2] = keep * b + half * (r + g);
    }
}

fn to_image(buf: &[f32], w: usize, h: usize) -> Image {
    Image {
        w,
        h,
        px: buf.iter().map(|&v| v.clamp(0.0, 255.0) as u8).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LinkConfig;
    use crate::geometry;

    #[test]
    fn rng_is_deterministic() {
        let a: Vec<u64> = (0..8)
            .scan(Rng::new(42), |r, _| Some(r.next_u64()))
            .collect();
        let b: Vec<u64> = (0..8)
            .scan(Rng::new(42), |r, _| Some(r.next_u64()))
            .collect();
        assert_eq!(a, b);
    }

    #[test]
    fn geometry_survives_a_realistic_capture() {
        let cfg = LinkConfig::balanced();
        let syms: Vec<u8> = (0..cfg.payload_cells()).map(|i| (i % 8) as u8).collect();
        let screen = geometry::render(&cfg, &syms, 6, 4);

        let mut ch = Channel::new(
            ChannelParams {
                drop_rate: 0.0,
                tear_rate: 0.0,
                ..ChannelParams::realistic()
            },
            7,
        );
        let cam = ch.capture(&screen).expect("no drops configured");
        assert!(
            geometry::locate(&cam, cfg.grid).is_some(),
            "markers must stay findable through warp, blur and vignetting"
        );
    }
}
