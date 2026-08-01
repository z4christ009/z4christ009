//! Frame geometry: cell layout, rendering, marker localisation and sampling.
//!
//! The receiver's job in this module is to recover the homography that maps grid
//! coordinates to camera pixels. Everything downstream assumes that mapping is
//! right, so this is where a real implementation earns or loses its throughput.

use crate::config::{LinkConfig, MARKER};
use crate::palette;

// ---------------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Image {
    pub w: usize,
    pub h: usize,
    /// Interleaved RGB, 3 bytes per pixel.
    pub px: Vec<u8>,
}

impl Image {
    pub fn new(w: usize, h: usize, fill: [u8; 3]) -> Self {
        let mut px = Vec::with_capacity(w * h * 3);
        for _ in 0..w * h {
            px.extend_from_slice(&fill);
        }
        Self { w, h, px }
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, rgb: [u8; 3]) {
        let i = (y * self.w + x) * 3;
        self.px[i..i + 3].copy_from_slice(&rgb);
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> [u8; 3] {
        let i = (y * self.w + x) * 3;
        [self.px[i], self.px[i + 1], self.px[i + 2]]
    }

    /// Bilinear sample in pixel coordinates. Out-of-bounds reads clamp to the edge.
    pub fn sample_bilinear(&self, x: f64, y: f64) -> [f32; 3] {
        let x = x.clamp(0.0, self.w as f64 - 1.001);
        let y = y.clamp(0.0, self.h as f64 - 1.001);
        let (x0, y0) = (x.floor() as usize, y.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.w - 1), (y0 + 1).min(self.h - 1));
        let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);

        let (p00, p10, p01, p11) = (
            self.get(x0, y0),
            self.get(x1, y0),
            self.get(x0, y1),
            self.get(x1, y1),
        );

        let mut out = [0f32; 3];
        for c in 0..3 {
            let top = p00[c] as f32 * (1.0 - fx) + p10[c] as f32 * fx;
            let bot = p01[c] as f32 * (1.0 - fx) + p11[c] as f32 * fx;
            out[c] = top * (1.0 - fy) + bot * fy;
        }
        out
    }

    pub fn luma(&self) -> Vec<u8> {
        (0..self.w * self.h)
            .map(|i| {
                let p = &self.px[i * 3..i * 3 + 3];
                ((p[0] as u32 * 77 + p[1] as u32 * 150 + p[2] as u32 * 29) >> 8) as u8
            })
            .collect()
    }

    /// Adopt a browser `ImageData` buffer, dropping alpha.
    pub fn from_rgba(rgba: &[u8], w: usize, h: usize) -> Self {
        let mut px = Vec::with_capacity(w * h * 3);
        for p in rgba.chunks_exact(4).take(w * h) {
            px.extend_from_slice(&p[..3]);
        }
        px.resize(w * h * 3, 0);
        Self { w, h, px }
    }

    /// Box-average by an integer factor.
    pub fn downscale(&self, factor: usize) -> Image {
        if factor <= 1 {
            return self.clone();
        }
        let (w, h) = (self.w / factor, self.h / factor);
        let mut px = vec![0u8; w * h * 3];
        let n = (factor * factor) as u32;
        for y in 0..h {
            for x in 0..w {
                let mut acc = [0u32; 3];
                for dy in 0..factor {
                    for dx in 0..factor {
                        let s = ((y * factor + dy) * self.w + (x * factor + dx)) * 3;
                        for c in 0..3 {
                            acc[c] += self.px[s + c] as u32;
                        }
                    }
                }
                let d = (y * w + x) * 3;
                for c in 0..3 {
                    px[d + c] = (acc[c] / n) as u8;
                }
            }
        }
        Image { w, h, px }
    }
}

// ---------------------------------------------------------------------------
// Cell layout
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellKind {
    Marker,
    Calibration,
    Payload,
}

pub fn cell_kind(row: usize, col: usize, grid: usize) -> CellKind {
    let in_marker = |r: usize, c: usize| {
        let top = r < MARKER;
        let bottom = r >= grid - MARKER;
        let left = c < MARKER;
        let right = c >= grid - MARKER;
        (top && left) || (top && right) || (bottom && left) || (bottom && right)
    };

    if in_marker(row, col) {
        CellKind::Marker
    } else if row == 0 {
        CellKind::Calibration
    } else {
        CellKind::Payload
    }
}

/// Is this cell of a marker block dark? `r`/`c` are local to the MARKER x MARKER block.
pub fn marker_is_dark(r: usize, c: usize) -> bool {
    // Outermost ring is the light separator.
    if r == 0 || c == 0 || r == MARKER - 1 || c == MARKER - 1 {
        return false;
    }
    let (r, c) = (r - 1, c - 1); // 0..7 within the finder
    if r == 0 || c == 0 || r == 6 || c == 6 {
        return true; // outer ring of the finder
    }
    (2..=4).contains(&r) && (2..=4).contains(&c) // inner block
}

/// Payload cells in raster order.
pub fn payload_cells(grid: usize) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for r in 0..grid {
        for c in 0..grid {
            if cell_kind(r, c, grid) == CellKind::Payload {
                v.push((r, c));
            }
        }
    }
    v
}

/// Calibration cells in raster order.
pub fn calibration_cells(grid: usize) -> Vec<(usize, usize)> {
    (0..grid)
        .filter(|&c| cell_kind(0, c, grid) == CellKind::Calibration)
        .map(|c| (0, c))
        .collect()
}

/// Marker block centres, in grid coordinates, ordered TL, TR, BL, BR.
pub fn marker_centers(grid: usize) -> [(f64, f64); 4] {
    let m = MARKER as f64 / 2.0;
    let g = grid as f64;
    [(m, m), (g - m, m), (m, g - m), (g - m, g - m)]
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Render one frame. `symbols` supplies the payload cells in raster order.
pub fn render(cfg: &LinkConfig, symbols: &[u8], scale: usize, quiet: usize) -> Image {
    let side = (cfg.grid + 2 * quiet) * scale;
    let mut img = Image::new(side, side, [255, 255, 255]);

    let mut payload_idx = 0usize;
    let mut calib_idx = 0usize;

    for r in 0..cfg.grid {
        for c in 0..cfg.grid {
            let rgb = match cell_kind(r, c, cfg.grid) {
                CellKind::Marker => {
                    // Fold into block-local coordinates. The finder is symmetric
                    // under reflection, so all four corners share one pattern.
                    let lr = if r < MARKER {
                        r
                    } else {
                        r - (cfg.grid - MARKER)
                    };
                    let lc = if c < MARKER {
                        c
                    } else {
                        c - (cfg.grid - MARKER)
                    };
                    if marker_is_dark(lr, lc) {
                        [0, 0, 0]
                    } else {
                        [255, 255, 255]
                    }
                }
                CellKind::Calibration => {
                    let s = palette::calibration_symbol(calib_idx);
                    calib_idx += 1;
                    palette::symbol_to_rgb(s)
                }
                CellKind::Payload => {
                    let s = symbols.get(payload_idx).copied().unwrap_or(0);
                    payload_idx += 1;
                    palette::symbol_to_rgb(s)
                }
            };

            let (x0, y0) = ((quiet + c) * scale, (quiet + r) * scale);
            for y in y0..y0 + scale {
                for x in x0..x0 + scale {
                    img.set(x, y, rgb);
                }
            }
        }
    }

    img
}

// ---------------------------------------------------------------------------
// Homography
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Homography(pub [f64; 9]);

impl Homography {
    #[inline]
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let h = &self.0;
        let w = h[6] * x + h[7] * y + h[8];
        (
            (h[0] * x + h[1] * y + h[2]) / w,
            (h[3] * x + h[4] * y + h[5]) / w,
        )
    }

    /// Direct linear transform from four point correspondences.
    pub fn from_correspondences(src: &[(f64, f64); 4], dst: &[(f64, f64); 4]) -> Option<Self> {
        let mut a = [[0f64; 9]; 8];
        for i in 0..4 {
            let (x, y) = src[i];
            let (u, v) = dst[i];
            a[2 * i] = [x, y, 1.0, 0.0, 0.0, 0.0, -u * x, -u * y, u];
            a[2 * i + 1] = [0.0, 0.0, 0.0, x, y, 1.0, -v * x, -v * y, v];
        }

        // Gaussian elimination with partial pivoting.
        for col in 0..8 {
            let mut pivot = col;
            for r in col + 1..8 {
                if a[r][col].abs() > a[pivot][col].abs() {
                    pivot = r;
                }
            }
            if a[pivot][col].abs() < 1e-12 {
                return None; // degenerate configuration
            }
            a.swap(col, pivot);

            let p = a[col][col];
            for v in a[col].iter_mut().skip(col) {
                *v /= p;
            }
            for r in 0..8 {
                if r != col && a[r][col].abs() > 0.0 {
                    let f = a[r][col];
                    for k in col..9 {
                        a[r][k] -= f * a[col][k];
                    }
                }
            }
        }

        let mut h = [0f64; 9];
        for (i, row) in a.iter().enumerate() {
            h[i] = row[8];
        }
        h[8] = 1.0;
        Some(Homography(h))
    }
}

// ---------------------------------------------------------------------------
// Marker localisation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct Component {
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
    area: usize,
}

impl Component {
    fn center(&self) -> (f64, f64) {
        (
            (self.min_x + self.max_x) as f64 / 2.0,
            (self.min_y + self.max_y) as f64 / 2.0,
        )
    }
    fn width(&self) -> usize {
        self.max_x - self.min_x + 1
    }
    fn height(&self) -> usize {
        self.max_y - self.min_y + 1
    }
    fn contains(&self, p: (f64, f64)) -> bool {
        p.0 >= self.min_x as f64
            && p.0 <= self.max_x as f64
            && p.1 >= self.min_y as f64
            && p.1 <= self.max_y as f64
    }
}

fn otsu(luma: &[u8]) -> u8 {
    let mut hist = [0u32; 256];
    for &v in luma {
        hist[v as usize] += 1;
    }
    let total = luma.len() as f64;
    let sum: f64 = (0..256).map(|i| i as f64 * hist[i] as f64).sum();

    let (mut sum_b, mut w_b, mut best, mut thresh) = (0f64, 0f64, -1f64, 128u8);
    for t in 0..256 {
        w_b += hist[t] as f64;
        if w_b == 0.0 {
            continue;
        }
        let w_f = total - w_b;
        if w_f == 0.0 {
            break;
        }
        sum_b += t as f64 * hist[t] as f64;
        let m_b = sum_b / w_b;
        let m_f = (sum - sum_b) / w_f;
        let between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if between > best {
            best = between;
            thresh = t as u8;
        }
    }
    thresh
}

fn connected_components(dark: &[bool], w: usize, h: usize) -> Vec<Component> {
    let mut seen = vec![false; w * h];
    let mut out = Vec::new();
    let mut stack: Vec<usize> = Vec::new();

    for start in 0..w * h {
        if !dark[start] || seen[start] {
            continue;
        }
        seen[start] = true;
        stack.push(start);

        let (sx, sy) = (start % w, start / w);
        let mut comp = Component {
            min_x: sx,
            min_y: sy,
            max_x: sx,
            max_y: sy,
            area: 0,
        };

        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            comp.area += 1;
            comp.min_x = comp.min_x.min(x);
            comp.max_x = comp.max_x.max(x);
            comp.min_y = comp.min_y.min(y);
            comp.max_y = comp.max_y.max(y);

            let push = |nx: usize, ny: usize, stack: &mut Vec<usize>, seen: &mut Vec<bool>| {
                let j = ny * w + nx;
                if dark[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                push(x - 1, y, &mut stack, &mut seen);
            }
            if x + 1 < w {
                push(x + 1, y, &mut stack, &mut seen);
            }
            if y > 0 {
                push(x, y - 1, &mut stack, &mut seen);
            }
            if y + 1 < h {
                push(x, y + 1, &mut stack, &mut seen);
            }
        }

        out.push(comp);
    }

    out
}

/// Locate the four corner markers and derive the grid-to-pixel homography.
///
/// Returns `None` when fewer than four plausible finders survive filtering --
/// which is the correct outcome for a blurred or half-occluded capture. The
/// caller drops the frame and the fountain code absorbs the loss.
pub fn locate(img: &Image, grid: usize) -> Option<Homography> {
    let luma = img.luma();
    let t = otsu(&luma);
    let dark: Vec<bool> = luma.iter().map(|&v| v < t).collect();
    let comps = connected_components(&dark, img.w, img.h);

    // A finder's outer ring fills roughly (7^2 - 5^2)/7^2 ~= 49% of its bounding
    // box, is close to square, and encloses the separate inner block. Those three
    // together are enough to reject payload blobs without any template matching.
    let min_area = (img.w * img.h) / 100_000 + 8;
    let candidates: Vec<Component> = comps
        .iter()
        .copied()
        .filter(|c| {
            if c.area < min_area {
                return false;
            }
            let (w, h) = (c.width() as f64, c.height() as f64);
            let aspect = w / h;
            if !(0.55..=1.8).contains(&aspect) {
                return false;
            }
            let fill = c.area as f64 / (w * h);
            if !(0.25..=0.80).contains(&fill) {
                return false;
            }
            // Must enclose a distinctly smaller dark component near its centre.
            let center = c.center();
            comps.iter().any(|o| {
                o.area < c.area / 2
                    && o.area >= 1
                    && o.contains(center)
                    && o.width() < c.width()
                    && o.height() < c.height()
            })
        })
        .collect();

    if candidates.len() < 4 {
        return None;
    }

    let corners = [
        (0.0, 0.0),
        (img.w as f64, 0.0),
        (0.0, img.h as f64),
        (img.w as f64, img.h as f64),
    ];

    let mut picked: Vec<(f64, f64)> = Vec::with_capacity(4);
    let mut used: Vec<(f64, f64)> = Vec::with_capacity(4);
    for corner in corners {
        let best = candidates
            .iter()
            .map(|c| c.center())
            .filter(|p| !used.iter().any(|u| u == p))
            .min_by(|a, b| {
                let da = (a.0 - corner.0).powi(2) + (a.1 - corner.1).powi(2);
                let db = (b.0 - corner.0).powi(2) + (b.1 - corner.1).powi(2);
                da.partial_cmp(&db).unwrap()
            })?;
        used.push(best);
        picked.push(best);
    }

    let dst: [(f64, f64); 4] = [picked[0], picked[1], picked[2], picked[3]];
    Homography::from_correspondences(&marker_centers(grid), &dst)
}

/// Downscale factor that keeps marker localisation affordable on a phone.
///
/// Connected-component analysis over a full 1080p frame is far too slow for a
/// real-time loop, and finders survive downsampling comfortably. Cell *sampling*
/// still runs at full resolution -- only detection is cheapened.
pub fn detect_factor(w: usize, h: usize) -> usize {
    (w.min(h) / 480).max(1)
}

/// Localise on a downscaled copy, then lift the homography back to full scale.
pub fn locate_downscaled(img: &Image, grid: usize, factor: usize) -> Option<Homography> {
    if factor <= 1 {
        return locate(img, grid);
    }
    let small = img.downscale(factor);
    let h = locate(&small, grid)?;

    // Box downsampling maps small-pixel centres to full-pixel centres exactly by
    // `factor`, so lifting is just scaling the output rows of the homography.
    let f = factor as f64;
    let mut m = h.0;
    for v in m.iter_mut().take(6) {
        *v *= f;
    }
    Some(Homography(m))
}

/// Render one frame at exactly one pixel per cell, as RGBA.
///
/// The browser scales this up on the canvas with smoothing disabled, which is
/// both faster and sharper than rasterising full-size cells here.
pub fn render_rgba(cfg: &LinkConfig, symbols: &[u8], out: &mut [u8]) {
    let grid = cfg.grid;
    debug_assert!(out.len() >= grid * grid * 4);

    let mut payload_idx = 0usize;
    let mut calib_idx = 0usize;

    for r in 0..grid {
        for c in 0..grid {
            let rgb = match cell_kind(r, c, grid) {
                CellKind::Marker => {
                    let lr = if r < MARKER { r } else { r - (grid - MARKER) };
                    let lc = if c < MARKER { c } else { c - (grid - MARKER) };
                    if marker_is_dark(lr, lc) {
                        [0, 0, 0]
                    } else {
                        [255, 255, 255]
                    }
                }
                CellKind::Calibration => {
                    let s = palette::calibration_symbol(calib_idx);
                    calib_idx += 1;
                    palette::symbol_to_rgb(s)
                }
                CellKind::Payload => {
                    let s = symbols.get(payload_idx).copied().unwrap_or(0);
                    payload_idx += 1;
                    palette::symbol_to_rgb(s)
                }
            };
            let i = (r * grid + c) * 4;
            out[i..i + 3].copy_from_slice(&rgb);
            out[i + 3] = 255;
        }
    }
}

// ---------------------------------------------------------------------------
// Sampling
// ---------------------------------------------------------------------------

/// Average a small cluster around each cell centre. Offsets are expressed in
/// *grid* units so the footprint scales automatically with capture resolution:
/// a wide average at 6 px/cell, a tight one at 21 px/cell.
fn sample_cell(img: &Image, h: &Homography, col: usize, row: usize) -> [f32; 3] {
    const OFF: [(f64, f64); 5] = [
        (0.0, 0.0),
        (-0.18, 0.0),
        (0.18, 0.0),
        (0.0, -0.18),
        (0.0, 0.18),
    ];

    let (cx, cy) = (col as f64 + 0.5, row as f64 + 0.5);
    let mut acc = [0f32; 3];
    for (dx, dy) in OFF {
        let (px, py) = h.apply(cx + dx, cy + dy);
        let s = img.sample_bilinear(px, py);
        for c in 0..3 {
            acc[c] += s[c];
        }
    }
    for c in acc.iter_mut() {
        *c /= OFF.len() as f32;
    }
    acc
}

pub struct SampledFrame {
    pub symbols: Vec<u8>,
    pub calibration: palette::Calibration,
}

/// Sample every cell, calibrate from the known strip, then classify the payload.
pub fn sample(img: &Image, h: &Homography, grid: usize) -> SampledFrame {
    let calib_samples: Vec<([f32; 3], u8)> = calibration_cells(grid)
        .iter()
        .enumerate()
        .map(|(i, &(r, c))| (sample_cell(img, h, c, r), palette::calibration_symbol(i)))
        .collect();

    let calibration = palette::Calibration::from_samples(&calib_samples);

    let symbols = payload_cells(grid)
        .iter()
        .map(|&(r, c)| calibration.classify(sample_cell(img, h, c, r)))
        .collect();

    SampledFrame {
        symbols,
        calibration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_config_arithmetic() {
        let cfg = LinkConfig::balanced();
        assert_eq!(payload_cells(cfg.grid).len(), cfg.payload_cells());
        assert_eq!(calibration_cells(cfg.grid).len(), cfg.calibration_cells());
    }

    #[test]
    fn finder_has_expected_shape() {
        // Separator ring is light all the way round.
        for i in 0..MARKER {
            assert!(!marker_is_dark(0, i));
            assert!(!marker_is_dark(MARKER - 1, i));
            assert!(!marker_is_dark(i, 0));
            assert!(!marker_is_dark(i, MARKER - 1));
        }
        assert!(marker_is_dark(1, 1)); // finder outer ring
        assert!(!marker_is_dark(2, 2)); // finder light ring
        assert!(marker_is_dark(4, 4)); // finder inner block
    }

    #[test]
    fn homography_roundtrips_a_known_warp() {
        let src = [(0.0, 0.0), (10.0, 0.0), (0.0, 10.0), (10.0, 10.0)];
        let dst = [(5.0, 3.0), (95.0, 8.0), (2.0, 88.0), (99.0, 97.0)];
        let h = Homography::from_correspondences(&src, &dst).expect("solvable");
        for i in 0..4 {
            let (u, v) = h.apply(src[i].0, src[i].1);
            assert!((u - dst[i].0).abs() < 1e-6, "x mismatch at {i}");
            assert!((v - dst[i].1).abs() < 1e-6, "y mismatch at {i}");
        }
    }

    #[test]
    fn markers_are_findable_in_a_clean_render() {
        let cfg = LinkConfig::balanced();
        let syms: Vec<u8> = (0..cfg.payload_cells()).map(|i| (i % 8) as u8).collect();
        let img = render(&cfg, &syms, 6, 4);

        let h = locate(&img, cfg.grid).expect("four finders in a clean render");

        // Recovered mapping should put marker centres where we drew them:
        // quiet zone of 4 cells, 6 px per cell.
        let expect = |gx: f64, gy: f64| -> (f64, f64) { ((4.0 + gx) * 6.0, (4.0 + gy) * 6.0) };
        for (i, (gx, gy)) in marker_centers(cfg.grid).iter().enumerate() {
            let (u, v) = h.apply(*gx, *gy);
            let (eu, ev) = expect(*gx, *gy);
            assert!(
                (u - eu).abs() < 2.0 && (v - ev).abs() < 2.0,
                "marker {i} landed at ({u:.1},{v:.1}), expected ~({eu:.1},{ev:.1})"
            );
        }
    }

    #[test]
    fn clean_render_samples_back_to_the_same_symbols() {
        let cfg = LinkConfig::balanced();
        let syms: Vec<u8> = (0..cfg.payload_cells())
            .map(|i| palette::filler_symbol(i))
            .collect();
        let img = render(&cfg, &syms, 6, 4);
        let h = locate(&img, cfg.grid).expect("locate");
        let got = sample(&img, &h, cfg.grid);
        assert_eq!(got.symbols, syms, "noiseless path must be exact");
    }
}
