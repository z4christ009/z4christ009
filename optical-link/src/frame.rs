//! Per-frame header and integrity check.
//!
//! The CRC is the arbiter of whether a frame is usable. Inner ECC does its best
//! and reports what it could not fix, but a frame that passes RS with residual
//! corruption would poison the fountain decoder, so every frame is checked
//! end-to-end before it is handed upward.

use crate::config::{LinkConfig, HEADER_LEN, MAGIC, PROTOCOL_VERSION};

// ---------------------------------------------------------------------------
// CRC32 (IEEE 802.3)
// ---------------------------------------------------------------------------

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c = CRC_TABLE[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Frame {
    pub flags: u8,
    pub session: u16,
    pub payload: Vec<u8>,
}

/// Assemble a frame and pad it to exactly one frame's worth of coded bytes.
pub fn build(cfg: &LinkConfig, flags: u8, session: u16, payload: &[u8]) -> Vec<u8> {
    assert!(
        payload.len() <= cfg.frame_payload(),
        "payload {} exceeds frame capacity {}",
        payload.len(),
        cfg.frame_payload()
    );

    let mut buf = vec![0u8; cfg.coded_bytes()];
    buf[0..2].copy_from_slice(&MAGIC);
    buf[2] = PROTOCOL_VERSION;
    buf[3] = flags;
    buf[4..6].copy_from_slice(&session.to_le_bytes());
    buf[6..8].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    buf[8..12].copy_from_slice(&crc32(payload).to_le_bytes());
    // 12..16 reserved
    buf[HEADER_LEN..HEADER_LEN + payload.len()].copy_from_slice(payload);
    buf
}

/// Validate and unpack a frame. `None` means "drop this frame" -- never an error
/// worth surfacing, since losing frames is the normal operating condition here.
pub fn parse(cfg: &LinkConfig, buf: &[u8]) -> Option<Frame> {
    if buf.len() < HEADER_LEN || buf[0..2] != MAGIC || buf[2] != PROTOCOL_VERSION {
        return None;
    }

    let session = u16::from_le_bytes([buf[4], buf[5]]);
    let len = u16::from_le_bytes([buf[6], buf[7]]) as usize;
    let crc = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);

    if len > cfg.frame_payload() || HEADER_LEN + len > buf.len() {
        return None;
    }

    let payload = &buf[HEADER_LEN..HEADER_LEN + len];
    if crc32(payload) != crc {
        return None;
    }

    Some(Frame {
        flags: buf[3],
        session,
        payload: payload.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn frame_roundtrips() {
        let cfg = LinkConfig::balanced();
        let payload: Vec<u8> = (0..2000).map(|i| (i % 253) as u8).collect();
        let buf = build(&cfg, 0, 0xBEEF, &payload);
        let got = parse(&cfg, &buf).expect("valid frame");
        assert_eq!(got.session, 0xBEEF);
        assert_eq!(got.payload, payload);
    }

    #[test]
    fn corrupted_payload_is_rejected() {
        let cfg = LinkConfig::balanced();
        let payload: Vec<u8> = vec![7; 1000];
        let mut buf = build(&cfg, 0, 1, &payload);
        buf[HEADER_LEN + 500] ^= 0x01;
        assert!(
            parse(&cfg, &buf).is_none(),
            "single flipped bit must fail the CRC"
        );
    }

    #[test]
    fn garbage_is_rejected() {
        let cfg = LinkConfig::balanced();
        assert!(parse(&cfg, &vec![0xFF; cfg.coded_bytes()]).is_none());
    }
}
