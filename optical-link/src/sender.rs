//! Transmit side: object in, cell symbols out.

use crate::config::{LinkConfig, FLAG_MANIFEST};
use crate::fountain::FountainEncoder;
use crate::{ecc, frame, palette};

/// Frames between manifests. At 60 fps this puts one on screen every ~0.4 s, so
/// a receiver that starts watching mid-stream locks on almost immediately.
/// Sending the manifest once at the start would be a mistake -- the camera is
/// very often not pointed at the screen yet when the transfer begins.
pub const MANIFEST_INTERVAL: u64 = 24;

pub struct Sender {
    cfg: LinkConfig,
    session: u16,
    encoder: FountainEncoder,
    manifest: Vec<u8>,
    frame_no: u64,
}

impl Sender {
    pub fn new(cfg: LinkConfig, data: &[u8], name: &str, session: u16) -> Self {
        let encoder = FountainEncoder::new(data, cfg.symbol_size(), 2.0);

        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len().min(255);
        let mut manifest = Vec::with_capacity(21 + name_len);
        manifest.extend_from_slice(&encoder.oti().0);
        manifest.extend_from_slice(&(data.len() as u32).to_le_bytes());
        manifest.extend_from_slice(&frame::crc32(data).to_le_bytes());
        manifest.push(name_len as u8);
        manifest.extend_from_slice(&name_bytes[..name_len]);

        Self {
            cfg,
            session,
            encoder,
            manifest,
            frame_no: 0,
        }
    }

    pub fn config(&self) -> &LinkConfig {
        &self.cfg
    }

    pub fn frames_emitted(&self) -> u64 {
        self.frame_no
    }

    /// Produce the next frame as cell symbols, in payload-cell raster order.
    pub fn next_frame(&mut self) -> Vec<u8> {
        let n = self.frame_no;
        self.frame_no += 1;

        let (flags, payload) = if n % MANIFEST_INTERVAL == 0 {
            (FLAG_MANIFEST, self.manifest.clone())
        } else {
            (0u8, self.encoder.next_packet().to_vec())
        };

        let framed = frame::build(&self.cfg, flags, self.session, &payload);
        let coded = ecc::encode(&framed, self.cfg.shard_count(), self.cfg.parity);

        let mut symbols = vec![0u8; self.cfg.payload_cells()];
        palette::pack_symbols(&coded, &mut symbols);
        symbols
    }
}
