//! Outer coding: RaptorQ (RFC 6330).
//!
//! There is no back-channel in an optical link -- the sender has no idea which
//! frames the camera missed. A rateless code makes that a non-problem: the sender
//! emits an endless stream of distinct encoded symbols and the receiver
//! reconstructs the object once it has collected any ~1.05x the source count.
//! Dropped frames cost time, never correctness.
//!
//! RaptorQ rather than plain LT because decoding stays linear as the object grows.
//! For a video-sized transfer, LT's peeling cost is a real problem.

use raptorq::{Decoder, Encoder, EncodingPacket, ObjectTransmissionInformation};

/// Everything the receiver needs before it can decode anything. Rides in the
/// manifest frames, which is why those are interleaved into the stream rather
/// than sent once at the start -- the camera may well not be looking yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Oti(pub [u8; 12]);

impl Oti {
    pub fn decode(&self) -> ObjectTransmissionInformation {
        ObjectTransmissionInformation::deserialize(&self.0)
    }
}

pub struct FountainEncoder {
    oti: Oti,
    packets: Vec<Vec<u8>>,
    cursor: usize,
}

impl FountainEncoder {
    /// Build an encoder over `data`, pre-generating a pool of `redundancy`x the
    /// source symbol count.
    ///
    /// Pre-generation is deliberate: on a phone you cannot afford to synthesise
    /// symbols inside a 16 ms frame budget, so real senders build a pool up front
    /// and cycle it. Sizing the pool at 2-3x source comfortably covers the loss
    /// rates a handheld camera actually produces.
    pub fn new(data: &[u8], symbol_size: u16, redundancy: f32) -> Self {
        let config = ObjectTransmissionInformation::with_defaults(data.len() as u64, symbol_size);
        let encoder = Encoder::new(data, config);

        let source_symbols = data.len().div_ceil(symbol_size as usize);
        let repair = ((source_symbols as f32) * redundancy).ceil() as u32;

        let packets = encoder
            .get_encoded_packets(repair)
            .iter()
            .map(|p| p.serialize())
            .collect();

        Self {
            oti: Oti(config.serialize()),
            packets,
            cursor: 0,
        }
    }

    pub fn oti(&self) -> Oti {
        self.oti
    }

    pub fn pool_len(&self) -> usize {
        self.packets.len()
    }

    /// Next symbol in the stream, cycling the pool when exhausted.
    pub fn next_packet(&mut self) -> &[u8] {
        let p = &self.packets[self.cursor % self.packets.len()];
        self.cursor += 1;
        p
    }
}

pub struct FountainDecoder {
    decoder: Decoder,
    accepted: usize,
}

impl FountainDecoder {
    pub fn new(oti: Oti) -> Self {
        Self {
            decoder: Decoder::new(oti.decode()),
            accepted: 0,
        }
    }

    /// Feed one symbol. Returns the object as soon as it is recoverable.
    pub fn push(&mut self, packet_bytes: &[u8]) -> Option<Vec<u8>> {
        self.accepted += 1;
        self.decoder
            .decode(EncodingPacket::deserialize(packet_bytes))
    }

    pub fn accepted(&self) -> usize {
        self.accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_from_a_lossy_subset() {
        let data: Vec<u8> = (0..200_000usize)
            .map(|i| (i.wrapping_mul(37) % 251) as u8)
            .collect();
        let symbol = 8008u16;

        let mut enc = FountainEncoder::new(&data, symbol, 2.0);
        let oti = enc.oti();
        let mut dec = FountainDecoder::new(oti);

        // Drop one frame in four, the shape of a mediocre handheld capture.
        let mut out = None;
        for i in 0..enc.pool_len() {
            let packet = enc.next_packet().to_vec();
            if i % 4 == 3 {
                continue;
            }
            if let Some(d) = dec.push(&packet) {
                out = Some(d);
                break;
            }
        }

        assert_eq!(
            out.as_deref(),
            Some(&data[..]),
            "object must reconstruct exactly"
        );
    }

    #[test]
    fn overhead_stays_near_optimal() {
        let data: Vec<u8> = (0..500_000).map(|i| (i % 256) as u8).collect();
        let symbol = 8008u16;
        let source = data.len().div_ceil(symbol as usize);

        let mut enc = FountainEncoder::new(&data, symbol, 1.0);
        let mut dec = FountainDecoder::new(enc.oti());

        let mut used = 0;
        for _ in 0..enc.pool_len() {
            let packet = enc.next_packet().to_vec();
            used += 1;
            if dec.push(&packet).is_some() {
                break;
            }
        }

        let overhead = used as f64 / source as f64;
        assert!(
            overhead < 1.15,
            "expected near-optimal overhead, got {overhead:.3}x"
        );
    }
}
