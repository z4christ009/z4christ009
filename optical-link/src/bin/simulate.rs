//! End-to-end simulation harness.
//!
//! Runs a real transfer through a synthetic camera and reports the numbers that
//! decide whether the link is viable: frame yield, ECC load, and goodput. Tune
//! here where a run costs seconds, not in front of two phones and a tripod.
//!
//!     cargo run --release --bin simulate -- --channel harsh --size 500000
//!     cargo run --release --bin simulate -- --preset fast --file clip.mp4

use std::time::Instant;

use optical_link::channel::{Channel, ChannelParams};
use optical_link::config::LinkConfig;
use optical_link::frame::crc32;
use optical_link::geometry::render;
use optical_link::{Receiver, Sender};

struct Args {
    cfg: LinkConfig,
    channel: Option<ChannelParams>,
    data: Vec<u8>,
    name: String,
    scale: usize,
    quiet: usize,
    seed: u64,
    max_frames: u64,
    perfect_geometry: bool,
}

fn usage() -> ! {
    eprintln!(
        "\
optical-link simulator

  --preset <robust|balanced|fast>   grid size / parity tradeoff  [balanced]
  --channel <good|realistic|harsh|perfect>                       [realistic]
  --file <path>                     payload to send
  --size <bytes>                    synthesise a payload instead  [250000]
  --scale <px>                      rendered pixels per cell      [6]
  --quiet <cells>                   quiet zone around the grid    [4]
  --capture <px>                    camera resolution, square     [per channel]
  --fps <n>                         display rate, for reporting   [60]
  --seed <n>                        channel PRNG seed             [1]
  --max-frames <n>                  give up after this many       [4000]
  --perfect-geometry                skip optics, isolate the codec
"
    );
    std::process::exit(2)
}

fn parse_args() -> Args {
    let mut cfg = LinkConfig::balanced();
    let mut channel = Some(ChannelParams::realistic());
    let mut file: Option<String> = None;
    let mut size = 250_000usize;
    let mut scale = 6usize;
    let mut quiet = 4usize;
    let mut seed = 1u64;
    let mut max_frames = 4000u64;
    let mut perfect_geometry = false;
    let mut capture: Option<usize> = None;

    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < argv.len() {
        let next = |i: usize| -> String { argv.get(i + 1).cloned().unwrap_or_else(|| usage()) };
        match argv[i].as_str() {
            "--preset" => {
                cfg = match next(i).as_str() {
                    "robust" => LinkConfig::robust(),
                    "balanced" => LinkConfig::balanced(),
                    "fast" => LinkConfig::fast(),
                    _ => usage(),
                };
                i += 2;
            }
            "--channel" => {
                channel = match next(i).as_str() {
                    "good" => Some(ChannelParams::good()),
                    "realistic" => Some(ChannelParams::realistic()),
                    "harsh" => Some(ChannelParams::harsh()),
                    "perfect" => None,
                    _ => usage(),
                };
                i += 2;
            }
            "--file" => {
                file = Some(next(i));
                i += 2;
            }
            "--size" => {
                size = next(i).parse().unwrap_or_else(|_| usage());
                i += 2;
            }
            "--scale" => {
                scale = next(i).parse().unwrap_or_else(|_| usage());
                i += 2;
            }
            "--quiet" => {
                quiet = next(i).parse().unwrap_or_else(|_| usage());
                i += 2;
            }
            "--capture" => {
                capture = Some(next(i).parse().unwrap_or_else(|_| usage()));
                i += 2;
            }
            "--fps" => {
                cfg.fps = next(i).parse().unwrap_or_else(|_| usage());
                i += 2;
            }
            "--seed" => {
                seed = next(i).parse().unwrap_or_else(|_| usage());
                i += 2;
            }
            "--max-frames" => {
                max_frames = next(i).parse().unwrap_or_else(|_| usage());
                i += 2;
            }
            "--perfect-geometry" => {
                perfect_geometry = true;
                i += 1;
            }
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }

    let (data, name) = match &file {
        Some(p) => {
            let d = std::fs::read(p).unwrap_or_else(|e| {
                eprintln!("cannot read {p}: {e}");
                std::process::exit(1)
            });
            let n = std::path::Path::new(p)
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "payload.bin".into());
            (d, n)
        }
        None => (
            (0..size)
                .map(|i| (i.wrapping_mul(2654435761) >> 7) as u8)
                .collect(),
            format!("synthetic-{size}.bin"),
        ),
    };

    if let (Some(c), Some(p)) = (capture, channel.as_mut()) {
        p.capture = c;
    }

    Args {
        cfg,
        channel,
        data,
        name,
        scale,
        quiet,
        seed,
        max_frames,
        perfect_geometry,
    }
}

fn main() {
    let args = parse_args();
    let cfg = args.cfg;

    println!("=== link configuration ===");
    println!("  grid              {0}x{0} cells", cfg.grid);
    println!("  payload cells     {}", cfg.payload_cells());
    println!("  raw bytes/frame   {}", cfg.raw_bytes());
    println!(
        "  inner ECC         {} shards x 255B, {} parity ({:.0}% rate)",
        cfg.shard_count(),
        cfg.parity,
        100.0 * cfg.shard_data() as f64 / 255.0
    );
    println!("  fountain symbol   {} B", cfg.symbol_size());
    println!(
        "  budgeted goodput  {:.2} Mbps at {} fps",
        cfg.ideal_bitrate() / 1.0e6,
        cfg.fps
    );

    match &args.channel {
        Some(p) if !args.perfect_geometry => {
            println!("\n=== channel ===");
            println!("  capture           {0}x{0} px", p.capture);
            println!(
                "  px per cell       {:.1}",
                p.capture as f64 * 0.9 / cfg.grid as f64
            );
            println!(
                "  warp {:.3}  blur {:.1}px  noise {:.1}  crosstalk {:.2}",
                p.warp, p.blur, p.noise, p.crosstalk
            );
            println!(
                "  drop {:.0}%  tear {:.0}%",
                p.drop_rate * 100.0,
                p.tear_rate * 100.0
            );
        }
        _ => println!("\n=== channel ===\n  bypassed (codec isolation)"),
    }

    println!("\n=== transfer ===");
    println!(
        "  payload           {} bytes ({})",
        args.data.len(),
        args.name
    );

    let mut tx = Sender::new(cfg, &args.data, &args.name, 0x4C4B);
    let mut rx = Receiver::new(cfg);
    let mut ch = args.channel.map(|p| Channel::new(p, args.seed));

    let started = Instant::now();
    let mut out: Option<Vec<u8>> = None;
    let mut displayed = 0u64;

    while displayed < args.max_frames {
        let symbols = tx.next_frame();
        displayed += 1;

        let got = match (&mut ch, args.perfect_geometry) {
            (Some(ch), false) => {
                let screen = render(&cfg, &symbols, args.scale, args.quiet);
                match ch.capture(&screen) {
                    Some(cam) => rx.push_image(&cam),
                    None => None,
                }
            }
            _ => rx.push_cells(&symbols),
        };

        if let Some(d) = got {
            out = Some(d);
            break;
        }
    }

    let wall = started.elapsed();
    let s = rx.stats;

    println!("\n=== results ===");
    println!("  frames displayed  {displayed}");
    if let Some(ch) = &ch {
        println!("  dropped by camera {}  torn {}", ch.dropped, ch.torn);
    }
    println!("  frames seen       {}", s.frames_seen);
    println!(
        "  geometry lock     {} ({:.1}%)",
        s.frames_locked,
        s.lock_rate() * 100.0
    );
    println!(
        "  frames valid      {} ({:.1}% of seen)",
        s.frames_valid,
        s.yield_rate() * 100.0
    );
    println!("  frames rejected   {}", s.frames_rejected);
    println!(
        "  RS shards         {} corrected, {} uncorrectable",
        s.ecc_corrected_shards, s.ecc_uncorrectable_shards
    );
    println!("  manifests seen    {}", s.manifests_seen);
    println!("  symbols accepted  {}", s.symbols_accepted);

    match out {
        Some(d) => {
            let ok = d == args.data && crc32(&d) == crc32(&args.data);
            let air_time = displayed as f64 / cfg.fps as f64;
            let goodput = args.data.len() as f64 * 8.0 / air_time;

            println!("\n  air time          {air_time:.2} s at {} fps", cfg.fps);
            println!(
                "  goodput           {:.2} Mbps ({:.1} KB/s)",
                goodput / 1.0e6,
                args.data.len() as f64 / air_time / 1024.0
            );
            println!(
                "  efficiency        {:.0}% of budget",
                100.0 * goodput / cfg.ideal_bitrate()
            );
            println!("  simulated in      {:.1} s wall clock", wall.as_secs_f64());
            println!(
                "\n  VERIFY            {}",
                if ok { "byte-identical" } else { "MISMATCH" }
            );
            if !ok {
                std::process::exit(1);
            }
        }
        None => {
            println!("\n  INCOMPLETE        gave up after {displayed} frames");
            println!("  The link is below the rate this channel can sustain.");
            println!("  Try --preset robust, a lower --channel, or more --scale.");
            std::process::exit(1);
        }
    }
}
