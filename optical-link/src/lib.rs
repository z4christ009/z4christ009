//! A screen-to-camera optical data link.
//!
//! Two phones, no radio: one displays a colour grid at 60 fps, the other films it.
//! The coding stack is three layers, each doing a job the others cannot:
//!
//! 1. **Colour cells** ([`palette`]) -- 3 bits per cell as three independent
//!    binary channels, so classification is three thresholds rather than a
//!    nearest-neighbour search, and each channel gets its own calibration.
//! 2. **Reed-Solomon** ([`ecc`]) -- corrects misread cells *within* a frame.
//!    Interleaved, so glare damages many codewords lightly instead of one fatally.
//! 3. **RaptorQ** ([`fountain`]) -- corrects whole frames going missing, with no
//!    back-channel. The sender never learns what the camera dropped.
//!
//! At the default 160x160 grid and 60 fps this budgets to ~3.7 Mbps of goodput.
//! [`channel`] provides a synthetic camera so the whole stack can be measured
//! without any hardware in the loop.

pub mod channel;
pub mod config;
pub mod ecc;
pub mod fountain;
pub mod frame;
pub mod geometry;
pub mod palette;
pub mod receiver;
pub mod sender;
pub mod wasm;

pub use config::LinkConfig;
pub use receiver::{Receiver, RxStats};
pub use sender::Sender;

#[cfg(test)]
mod end_to_end {
    use super::*;
    use channel::{Channel, ChannelParams};
    use geometry::render;

    /// Codec only, no optics. If this fails the problem is in the coding stack.
    #[test]
    fn transfers_cleanly_without_optics() {
        let cfg = LinkConfig::balanced();
        let data: Vec<u8> = (0..300_000usize)
            .map(|i| (i.wrapping_mul(97) % 251) as u8)
            .collect();

        let mut tx = Sender::new(cfg, &data, "clip.mp4", 0x1234);
        let mut rx = Receiver::new(cfg);

        let mut out = None;
        for _ in 0..400 {
            let syms = tx.next_frame();
            if let Some(d) = rx.push_cells(&syms) {
                out = Some(d);
                break;
            }
        }

        assert_eq!(out.as_deref(), Some(&data[..]));
        assert_eq!(rx.stats.frames_rejected, 0);
    }

    /// Full path including render, warp, blur, noise and marker localisation.
    #[test]
    fn transfers_through_a_realistic_camera() {
        let cfg = LinkConfig::balanced();
        let data: Vec<u8> = (0..120_000usize)
            .map(|i| (i.wrapping_mul(31) % 253) as u8)
            .collect();

        let mut tx = Sender::new(cfg, &data, "clip.mp4", 0x77);
        let mut rx = Receiver::new(cfg);
        let mut ch = Channel::new(ChannelParams::realistic(), 99);

        let mut out = None;
        for _ in 0..300 {
            let screen = render(&cfg, &tx.next_frame(), 6, 4);
            if let Some(cam) = ch.capture(&screen) {
                if let Some(d) = rx.push_image(&cam) {
                    out = Some(d);
                    break;
                }
            }
        }

        assert_eq!(
            out.as_deref(),
            Some(&data[..]),
            "must reconstruct byte-identical"
        );
        assert!(
            rx.stats.yield_rate() > 0.5,
            "yield {:.1}% is too low to be believable",
            rx.stats.yield_rate() * 100.0
        );
    }

    /// A receiver that starts watching mid-transfer must still catch up. This is
    /// the normal case in practice -- you point the camera after starting the send.
    #[test]
    fn receiver_can_join_late() {
        let cfg = LinkConfig::balanced();
        let data: Vec<u8> = (0..150_000).map(|i| (i % 256) as u8).collect();

        let mut tx = Sender::new(cfg, &data, "late.bin", 0x5A);
        let mut rx = Receiver::new(cfg);

        for _ in 0..37 {
            let _ = tx.next_frame(); // camera not pointed at the screen yet
        }

        let mut out = None;
        for _ in 0..400 {
            if let Some(d) = rx.push_cells(&tx.next_frame()) {
                out = Some(d);
                break;
            }
        }

        assert_eq!(out.as_deref(), Some(&data[..]));
    }
}
