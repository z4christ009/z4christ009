//! Link geometry and rate configuration.
//!
//! Everything downstream derives from these numbers, so this is the one place
//! to tune when you're chasing throughput or fighting a flaky camera.

/// Size of a corner marker block, in cells: a 7-cell concentric finder (the
/// classic 1:1:3:1:1 ratio, well-studied and easy to localise under perspective)
/// wrapped in a 1-cell light separator. The separator is not decoration -- without
/// it a dark payload cell touching the finder merges with it during connected-
/// component analysis and the marker stops being findable.
pub const MARKER: usize = 9;

/// Bytes of RS parity per 255-byte codeword. 32 parity corrects up to 16 symbol
/// errors per shard (~6.3% of the shard) at a 12.5% rate cost.
pub const DEFAULT_PARITY: usize = 32;

/// Frame header: magic(2) version(1) flags(1) session(2) len(2) crc(4) = 12,
/// padded to 16 for alignment and future use.
pub const HEADER_LEN: usize = 16;

pub const MAGIC: [u8; 2] = *b"GL";
pub const PROTOCOL_VERSION: u8 = 1;

pub const FLAG_MANIFEST: u8 = 0x01;

#[derive(Clone, Copy, Debug)]
pub struct LinkConfig {
    /// Cells per side of the square grid.
    pub grid: usize,
    /// RS parity bytes per 255-byte codeword.
    pub parity: usize,
    /// Display/capture frame rate, used only for throughput reporting.
    pub fps: u32,
}

impl Default for LinkConfig {
    fn default() -> Self {
        Self::balanced()
    }
}

impl LinkConfig {
    /// 128x128 cells. Widest margins, use when optics are poor or the camera is
    /// only 1080p. ~1.9 Mbps at 60 fps.
    pub fn robust() -> Self {
        Self {
            grid: 128,
            parity: 48,
            fps: 60,
        }
    }

    /// 160x160 cells. The default target: ~3.8 Mbps at 60 fps, ~6 px/cell on a
    /// 1080p capture and ~21 px/cell at 4K.
    pub fn balanced() -> Self {
        Self {
            grid: 160,
            parity: DEFAULT_PARITY,
            fps: 60,
        }
    }

    /// 192x192 cells. Needs 4K capture and good focus. ~5.5 Mbps at 60 fps.
    pub fn fast() -> Self {
        Self {
            grid: 192,
            parity: 24,
            fps: 60,
        }
    }

    /// Cells consumed by the four corner markers.
    pub fn marker_cells(&self) -> usize {
        4 * MARKER * MARKER
    }

    /// Calibration cells: the top row, minus whatever the two top markers cover.
    pub fn calibration_cells(&self) -> usize {
        self.grid.saturating_sub(2 * MARKER)
    }

    /// Cells carrying data.
    pub fn payload_cells(&self) -> usize {
        self.grid * self.grid - self.marker_cells() - self.calibration_cells()
    }

    /// Raw bytes per frame before any coding. 3 bits/cell.
    pub fn raw_bytes(&self) -> usize {
        (self.payload_cells() * 3) / 8
    }

    /// Bytes per RS codeword available for data.
    pub fn shard_data(&self) -> usize {
        255 - self.parity
    }

    /// Number of RS codewords that fit in one frame.
    pub fn shard_count(&self) -> usize {
        self.raw_bytes() / 255
    }

    /// Bytes surviving inner ECC, before the frame header.
    pub fn coded_bytes(&self) -> usize {
        self.shard_count() * self.shard_data()
    }

    /// Bytes of RaptorQ packet carried per frame (header removed).
    pub fn frame_payload(&self) -> usize {
        self.coded_bytes() - HEADER_LEN
    }

    /// RaptorQ symbol size. The 4-byte PayloadId rides in front of each symbol.
    pub fn symbol_size(&self) -> u16 {
        (self.frame_payload() - 4) as u16
    }

    /// Goodput in bits/sec, assuming every frame decodes and ~5% fountain overhead.
    pub fn ideal_bitrate(&self) -> f64 {
        self.symbol_size() as f64 * 8.0 * self.fps as f64 / 1.05
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_hits_target_band() {
        let c = LinkConfig::balanced();
        let mbps = c.ideal_bitrate() / 1.0e6;
        assert!(
            (3.0..4.5).contains(&mbps),
            "balanced preset should land in the 3-4 Mbps band, got {mbps:.2}"
        );
    }

    #[test]
    fn presets_are_internally_consistent() {
        for c in [
            LinkConfig::robust(),
            LinkConfig::balanced(),
            LinkConfig::fast(),
        ] {
            assert!(c.shard_count() > 0);
            assert!(c.symbol_size() > 0);
            assert!(c.coded_bytes() > HEADER_LEN);
            // Inner ECC output must actually fit in the cells we have.
            assert!(c.shard_count() * 255 <= c.raw_bytes());
        }
    }
}
