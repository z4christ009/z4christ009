//! Color coding: 3 bits per cell, carried as three *independent* binary channels.
//!
//! The eight symbols are the corners of the RGB cube, so bit 0 rides on red,
//! bit 1 on green, bit 2 on blue. That choice matters more than it looks:
//! classification collapses to three separate one-bit thresholds instead of a
//! nearest-neighbour search in 3-space, which is trivially parallel on a GPU and
//! lets each channel carry its own threshold. Displays and sensors do not agree
//! on what "red" is, and they disagree differently per channel.

/// Cell drive levels. Full swing maximises separation; back these off if your
/// camera blooms on saturated colour.
pub const LOW: u8 = 0;
pub const HIGH: u8 = 255;

pub const BITS_PER_CELL: usize = 3;
pub const SYMBOL_COUNT: usize = 1 << BITS_PER_CELL;

/// Symbol (0..8) to RGB.
#[inline]
pub fn symbol_to_rgb(sym: u8) -> [u8; 3] {
    [
        if sym & 0b001 != 0 { HIGH } else { LOW },
        if sym & 0b010 != 0 { HIGH } else { LOW },
        if sym & 0b100 != 0 { HIGH } else { LOW },
    ]
}

/// Per-channel decision thresholds, measured from the calibration strip.
///
/// Fixed thresholds do not survive contact with a real camera: auto white
/// balance, display gamma and ambient colour temperature all shift the levels,
/// and they shift by different amounts per channel. Re-measuring every frame is
/// what makes colour viable at all.
#[derive(Clone, Copy, Debug)]
pub struct Calibration {
    pub lo: [f32; 3],
    pub hi: [f32; 3],
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            lo: [LOW as f32; 3],
            hi: [HIGH as f32; 3],
        }
    }
}

impl Calibration {
    #[inline]
    pub fn threshold(&self, ch: usize) -> f32 {
        (self.lo[ch] + self.hi[ch]) * 0.5
    }

    /// Channel separation in raw levels. Below ~30 the link is about to fall
    /// apart and it is worth surfacing that rather than silently decoding mush.
    pub fn min_separation(&self) -> f32 {
        (0..3)
            .map(|c| self.hi[c] - self.lo[c])
            .fold(f32::INFINITY, f32::min)
    }

    #[inline]
    pub fn classify(&self, px: [f32; 3]) -> u8 {
        let mut sym = 0u8;
        for c in 0..3 {
            if px[c] > self.threshold(c) {
                sym |= 1 << c;
            }
        }
        sym
    }

    /// Build a calibration from observed cells whose true symbols are known.
    pub fn from_samples(samples: &[([f32; 3], u8)]) -> Self {
        let mut lo_sum = [0f32; 3];
        let mut lo_n = [0u32; 3];
        let mut hi_sum = [0f32; 3];
        let mut hi_n = [0u32; 3];

        for (px, sym) in samples {
            for c in 0..3 {
                if sym & (1 << c) != 0 {
                    hi_sum[c] += px[c];
                    hi_n[c] += 1;
                } else {
                    lo_sum[c] += px[c];
                    lo_n[c] += 1;
                }
            }
        }

        let mut cal = Calibration::default();
        for c in 0..3 {
            if lo_n[c] > 0 {
                cal.lo[c] = lo_sum[c] / lo_n[c] as f32;
            }
            if hi_n[c] > 0 {
                cal.hi[c] = hi_sum[c] / hi_n[c] as f32;
            }
        }
        cal
    }
}

/// The pattern written into the calibration strip. Cycling all eight symbols
/// guarantees every channel sees both levels regardless of what the payload
/// happens to look like.
#[inline]
pub fn calibration_symbol(index: usize) -> u8 {
    (index % SYMBOL_COUNT) as u8
}

/// Filler for cells left over after the coded payload. Keeping it varied avoids
/// large flat regions that would otherwise pull the camera's auto-exposure around.
#[inline]
pub fn filler_symbol(index: usize) -> u8 {
    ((index.wrapping_mul(2654435761) >> 13) % SYMBOL_COUNT as usize) as u8
}

/// Pack bytes into 3-bit symbols, MSB-first.
///
/// A byte length that is not a whole number of 3-bit groups leaves a partial
/// final group, which is zero-padded rather than dropped. The production path
/// never produces one -- shards are 255 bytes and 255*8 divides by 3 exactly --
/// but silently truncating the tail would be a trap for whoever next changes the
/// shard arithmetic.
pub fn pack_symbols(data: &[u8], out: &mut [u8]) {
    let total_bits = data.len() * 8;
    for (i, slot) in out.iter_mut().enumerate() {
        let base = i * BITS_PER_CELL;
        if base >= total_bits {
            *slot = filler_symbol(i);
            continue;
        }
        let mut sym = 0u8;
        for b in 0..BITS_PER_CELL {
            let idx = base + b;
            let val = if idx < total_bits {
                (data[idx >> 3] >> (7 - (idx & 7))) & 1
            } else {
                0
            };
            sym = (sym << 1) | val;
        }
        *slot = sym;
    }
}

/// Inverse of [`pack_symbols`]. Trailing filler symbols and pad bits are ignored.
pub fn unpack_symbols(symbols: &[u8], out: &mut [u8]) {
    out.fill(0);
    let total_bits = out.len() * 8;
    for (i, &sym) in symbols.iter().enumerate() {
        let base = i * BITS_PER_CELL;
        if base >= total_bits {
            break;
        }
        for b in 0..BITS_PER_CELL {
            let idx = base + b;
            if idx >= total_bits {
                break;
            }
            let val = (sym >> (BITS_PER_CELL - 1 - b)) & 1;
            out[idx >> 3] |= val << (7 - (idx & 7));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(len: usize) {
        let data: Vec<u8> = (0..=255u8).cycle().take(len).collect();
        let mut syms = vec![0u8; len * 8 / 3 + 40];
        pack_symbols(&data, &mut syms);
        let mut back = vec![0u8; len];
        unpack_symbols(&syms, &mut back);
        assert!(data == back, "roundtrip failed for {len} bytes");
    }

    #[test]
    fn pack_roundtrips() {
        roundtrip(1000);
    }

    /// Every residue of (len * 8) mod 3, so the partial-final-group path is covered.
    #[test]
    fn pack_roundtrips_at_every_tail_alignment() {
        for len in [1, 2, 3, 254, 255, 256, 997, 998, 999] {
            roundtrip(len);
        }
    }

    /// The size the real pipeline actually uses: whole 255-byte shards, which
    /// pack to exactly 680 symbols each with no padding.
    #[test]
    fn shard_sized_packing_is_exact() {
        assert_eq!(255 * 8 % BITS_PER_CELL, 0);
        roundtrip(36 * 255);
    }

    #[test]
    fn symbols_map_to_distinct_colors() {
        let mut seen = std::collections::HashSet::new();
        for s in 0..SYMBOL_COUNT as u8 {
            assert!(seen.insert(symbol_to_rgb(s)));
        }
    }

    #[test]
    fn calibration_recovers_shifted_levels() {
        // Simulate a camera that crushes blacks and has a warm white balance.
        let samples: Vec<([f32; 3], u8)> = (0..64)
            .map(|i| {
                let s = (i % 8) as u8;
                let rgb = symbol_to_rgb(s);
                let px = [
                    rgb[0] as f32 * 0.9 + 30.0,
                    rgb[1] as f32 * 0.8 + 25.0,
                    rgb[2] as f32 * 0.7 + 20.0,
                ];
                (px, s)
            })
            .collect();

        let cal = Calibration::from_samples(&samples);
        for s in 0..SYMBOL_COUNT as u8 {
            let rgb = symbol_to_rgb(s);
            let px = [
                rgb[0] as f32 * 0.9 + 30.0,
                rgb[1] as f32 * 0.8 + 25.0,
                rgb[2] as f32 * 0.7 + 20.0,
            ];
            assert_eq!(cal.classify(px), s, "symbol {s} misclassified");
        }
    }
}
