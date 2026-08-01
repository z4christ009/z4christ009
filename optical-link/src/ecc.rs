//! Inner error correction: Reed-Solomon over GF(256), interleaved.
//!
//! Two coding layers do different jobs here. This one fixes *symbol errors*
//! inside a frame -- a cell whose colour got misread. The fountain code above it
//! only handles whole frames going missing. Colour coding produces errors, not
//! erasures, so without this layer a handful of bad cells destroys an otherwise
//! perfectly good frame.
//!
//! Shards are interleaved across the cell grid so that a glare spot or a smudge
//! -- which hits physically contiguous cells -- spreads its damage thinly over
//! many codewords instead of blowing through one codeword's correction budget.

use reed_solomon::{Decoder, Encoder};

pub const CODEWORD: usize = 255;

#[derive(Debug, Clone, Copy, Default)]
pub struct EccStats {
    pub shards: usize,
    pub corrected: usize,
    pub uncorrectable: usize,
}

/// Encode `data` into `shard_count` interleaved RS codewords.
///
/// `data` must be exactly `shard_count * (CODEWORD - parity)` bytes.
pub fn encode(data: &[u8], shard_count: usize, parity: usize) -> Vec<u8> {
    let data_len = CODEWORD - parity;
    assert_eq!(
        data.len(),
        shard_count * data_len,
        "ecc::encode got a short payload"
    );

    let enc = Encoder::new(parity);
    let mut shards: Vec<Vec<u8>> = Vec::with_capacity(shard_count);
    for i in 0..shard_count {
        let chunk = &data[i * data_len..(i + 1) * data_len];
        shards.push(enc.encode(chunk)[..].to_vec());
    }

    // Column-major interleave: consecutive output bytes come from different shards.
    let mut out = vec![0u8; shard_count * CODEWORD];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = shards[i % shard_count][i / shard_count];
    }
    out
}

/// Deinterleave and correct. Returns the recovered data plus per-frame stats.
///
/// Uncorrectable shards are zero-filled rather than aborting the frame: the CRC
/// in the frame header is the real arbiter, and letting a partially-bad frame
/// through costs nothing since the fountain layer discards it anyway.
pub fn decode(coded: &[u8], shard_count: usize, parity: usize) -> (Vec<u8>, EccStats) {
    let data_len = CODEWORD - parity;
    let dec = Decoder::new(parity);
    let mut stats = EccStats {
        shards: shard_count,
        ..Default::default()
    };

    let mut shards: Vec<Vec<u8>> = vec![vec![0u8; CODEWORD]; shard_count];
    for i in 0..shard_count * CODEWORD {
        if i < coded.len() {
            shards[i % shard_count][i / shard_count] = coded[i];
        }
    }

    let mut out = vec![0u8; shard_count * data_len];
    for (i, shard) in shards.iter().enumerate() {
        match dec.correct(shard, None) {
            Ok(fixed) => {
                if fixed[..] != shard[..] {
                    stats.corrected += 1;
                }
                out[i * data_len..(i + 1) * data_len].copy_from_slice(&fixed[..data_len]);
            }
            Err(_) => {
                stats.uncorrectable += 1;
                // Leave zeros; the frame CRC will reject this frame.
            }
        }
    }

    (out, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_data(shards: usize, parity: usize) -> Vec<u8> {
        (0..shards * (CODEWORD - parity))
            .map(|i| (i.wrapping_mul(31) % 251) as u8)
            .collect()
    }

    #[test]
    fn clean_roundtrip() {
        let (shards, parity) = (37, 32);
        let data = sample_data(shards, parity);
        let coded = encode(&data, shards, parity);
        let (out, stats) = decode(&coded, shards, parity);
        assert_eq!(data, out);
        assert_eq!(stats.uncorrectable, 0);
    }

    #[test]
    fn corrects_a_burst_thanks_to_interleaving() {
        let (shards, parity) = (37, 32);
        let data = sample_data(shards, parity);
        let mut coded = encode(&data, shards, parity);

        // A contiguous smear across the cell stream, the shape of real glare.
        // Spread over 37 shards this is ~8 errors each, inside the 16-error budget.
        for b in coded.iter_mut().take(300) {
            *b ^= 0xFF;
        }

        let (out, stats) = decode(&coded, shards, parity);
        assert_eq!(data, out, "interleaved burst should be fully correctable");
        assert_eq!(stats.uncorrectable, 0);
    }

    #[test]
    fn reports_uncorrectable_when_overwhelmed() {
        let (shards, parity) = (37, 32);
        let data = sample_data(shards, parity);
        let mut coded = encode(&data, shards, parity);
        for b in coded.iter_mut() {
            *b ^= 0xAA;
        }
        let (_, stats) = decode(&coded, shards, parity);
        assert!(
            stats.uncorrectable > 0,
            "saturated frame should report failures"
        );
    }
}
