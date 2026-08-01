//! Receive side: camera frames (or raw cell symbols) in, reconstructed object out.

use crate::config::{LinkConfig, FLAG_MANIFEST};
use crate::fountain::{FountainDecoder, Oti};
use crate::geometry::{self, Image};
use crate::{ecc, frame, palette};

/// Data frames held while waiting for a manifest. Without this the receiver
/// throws away every frame that arrives before the first manifest -- up to a
/// full manifest interval of perfectly good symbols, for no reason.
const PENDING_CAP: usize = 128;

#[derive(Debug, Clone)]
pub struct Manifest {
    pub oti: Oti,
    pub total_len: u32,
    pub crc: u32,
    pub name: String,
}

fn parse_manifest(p: &[u8]) -> Option<Manifest> {
    if p.len() < 21 {
        return None;
    }
    let mut oti = [0u8; 12];
    oti.copy_from_slice(&p[0..12]);
    let total_len = u32::from_le_bytes([p[12], p[13], p[14], p[15]]);
    let crc = u32::from_le_bytes([p[16], p[17], p[18], p[19]]);
    let name_len = p[20] as usize;
    if p.len() < 21 + name_len {
        return None;
    }
    let name = String::from_utf8_lossy(&p[21..21 + name_len]).into_owned();
    Some(Manifest {
        oti: Oti(oti),
        total_len,
        crc,
        name,
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RxStats {
    /// Camera frames handed to the receiver.
    pub frames_seen: u64,
    /// Frames where all four markers were found and a homography was solved.
    pub frames_locked: u64,
    /// Frames that survived ECC and passed their CRC.
    pub frames_valid: u64,
    /// Frames dropped after geometry lock -- too many cell errors for the RS budget.
    pub frames_rejected: u64,
    pub ecc_corrected_shards: u64,
    pub ecc_uncorrectable_shards: u64,
    pub manifests_seen: u64,
    pub symbols_accepted: u64,
}

impl RxStats {
    pub fn lock_rate(&self) -> f64 {
        ratio(self.frames_locked, self.frames_seen)
    }
    /// The number that actually matters: fraction of camera frames that turned
    /// into usable fountain symbols.
    pub fn yield_rate(&self) -> f64 {
        ratio(self.frames_valid, self.frames_seen)
    }
}

fn ratio(a: u64, b: u64) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

pub struct Receiver {
    cfg: LinkConfig,
    session: Option<u16>,
    manifest: Option<Manifest>,
    decoder: Option<FountainDecoder>,
    pending: Vec<Vec<u8>>,
    done: bool,
    pub stats: RxStats,
}

impl Receiver {
    pub fn new(cfg: LinkConfig) -> Self {
        Self {
            cfg,
            session: None,
            manifest: None,
            decoder: None,
            pending: Vec::new(),
            done: false,
            stats: RxStats::default(),
        }
    }

    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }

    /// Full camera path: localise the grid, sample it, then decode.
    pub fn push_image(&mut self, img: &Image) -> Option<Vec<u8>> {
        self.push_image_with_factor(img, 1)
    }

    /// As [`Receiver::push_image`], but localise markers on a `factor`-downscaled
    /// copy. Cell sampling still runs at full resolution -- only detection is
    /// cheapened, which is what makes the loop affordable on a phone.
    /// `factor` of 0 or 1 means full resolution.
    pub fn push_image_with_factor(&mut self, img: &Image, factor: usize) -> Option<Vec<u8>> {
        self.stats.frames_seen += 1;
        let h = match geometry::locate_downscaled(img, self.cfg.grid, factor.max(1)) {
            Some(h) => h,
            None => return None, // no lock; fountain absorbs it
        };
        self.stats.frames_locked += 1;
        let sampled = geometry::sample(img, &h, self.cfg.grid);
        self.decode_cells(&sampled.symbols)
    }

    /// Decode from cell symbols directly, skipping optics. Used by tests and by
    /// the simulator's `--perfect-geometry` mode so codec bugs and geometry bugs
    /// can be isolated from each other.
    pub fn push_cells(&mut self, symbols: &[u8]) -> Option<Vec<u8>> {
        self.stats.frames_seen += 1;
        self.stats.frames_locked += 1;
        self.decode_cells(symbols)
    }

    fn decode_cells(&mut self, symbols: &[u8]) -> Option<Vec<u8>> {
        if self.done {
            return None;
        }

        let mut coded = vec![0u8; self.cfg.shard_count() * ecc::CODEWORD];
        palette::unpack_symbols(symbols, &mut coded);

        let (framed, es) = ecc::decode(&coded, self.cfg.shard_count(), self.cfg.parity);
        self.stats.ecc_corrected_shards += es.corrected as u64;
        self.stats.ecc_uncorrectable_shards += es.uncorrectable as u64;

        let f = match frame::parse(&self.cfg, &framed) {
            Some(f) => f,
            None => {
                self.stats.frames_rejected += 1;
                return None;
            }
        };

        // Ignore a stale window still showing a previous transfer.
        match self.session {
            None => self.session = Some(f.session),
            Some(s) if s != f.session => return None,
            _ => {}
        }

        self.stats.frames_valid += 1;

        if f.flags & FLAG_MANIFEST != 0 {
            self.stats.manifests_seen += 1;
            if self.manifest.is_none() {
                if let Some(m) = parse_manifest(&f.payload) {
                    self.decoder = Some(FountainDecoder::new(m.oti));
                    self.manifest = Some(m);
                    // Replay whatever arrived before we knew how to interpret it.
                    let queued = std::mem::take(&mut self.pending);
                    for p in queued {
                        if let Some(out) = self.feed(&p) {
                            return Some(out);
                        }
                    }
                }
            }
            return None;
        }

        if self.decoder.is_none() {
            if self.pending.len() < PENDING_CAP {
                self.pending.push(f.payload);
            }
            return None;
        }

        self.feed(&f.payload)
    }

    fn feed(&mut self, packet: &[u8]) -> Option<Vec<u8>> {
        let dec = self.decoder.as_mut()?;
        self.stats.symbols_accepted += 1;
        let out = dec.push(packet)?;

        // Trust nothing until the whole-object CRC agrees.
        if let Some(m) = &self.manifest {
            if out.len() < m.total_len as usize {
                return None;
            }
            let out = out[..m.total_len as usize].to_vec();
            if frame::crc32(&out) != m.crc {
                return None;
            }
            self.done = true;
            return Some(out);
        }
        None
    }
}
