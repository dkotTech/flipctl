//! display-agent — мост между render-rs и USB-дисплеем (MCU).
//!
//! Подписывается на WebSocket рендера в raw-формате (rgb565/rgb888, серверный
//! даунскейл под матрицу) и пишет кадры в sink фреймированным бинарным
//! протоколом. Сейчас sink — файл (валидация); протокол готов под serial.
//!
//! Протокол устройства (host → device), little-endian:
//!   0xA5 | type u8 | len u32 | ts_ms u64 | body[len]
//!   type 0x01 = frame, body = кадр WS рендера как есть:
//!     "FR" | fmt u8 (0=rgb565, 1=rgb888) | flags u8 | w u16 | h u16 | seq u32 | pixels
//!   type 0x02 = input (device → host, для serial в будущем):
//!     key u8 | state u8 (1=down, 0=up)
//!
//! Примеры:
//!   display-agent stream --url ws://localhost:8092/api/renders/1/ws \
//!       --format rgb565 --width 256 --height 144 --fps 10 --out file:stream.bin
//!   display-agent inspect stream.bin
//!   display-agent inspect stream.bin --dump 5:frame5.png

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::{Parser, Subcommand};
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::tungstenite::Message;

const PACKET_MAGIC: u8 = 0xA5;
const PACKET_FRAME: u8 = 0x01;

#[derive(Parser)]
#[command(name = "display-agent", about = "Bridge render-rs frames to a USB display")]
struct Args {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Subscribe to a render and stream frames into a sink
    Stream {
        /// Render WS endpoint, e.g. ws://localhost:8092/api/renders/1/ws
        #[arg(long)]
        url: String,

        /// Pixel format the display eats: rgb565 | rgb888
        #[arg(long, default_value = "rgb565")]
        format: String,

        /// Display width (server-side downscale); omit to keep render size
        #[arg(long)]
        width: Option<u32>,

        /// Display height
        #[arg(long)]
        height: Option<u32>,

        /// Frame pacing for this subscriber
        #[arg(long, default_value_t = 10)]
        fps: u32,

        /// Sink: file:<path> (validation) | serial:<dev>[:baud] (TODO)
        #[arg(long, default_value = "file:display-stream.bin")]
        out: String,
    },

    /// Parse a recorded stream file: packet stats, pacing, integrity
    Inspect {
        /// Stream file written by `stream --out file:...`
        file: String,

        /// Extract one frame as PNG: "<packet_index>:<out.png>"
        #[arg(long)]
        dump: Option<String>,
    },
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── stream ────────────────────────────────────────────────────────────────────

enum Sink {
    File(tokio::fs::File),
}

impl Sink {
    async fn open(spec: &str) -> Result<Sink, String> {
        match spec.split_once(':') {
            Some(("file", path)) => {
                let file = tokio::fs::File::create(path)
                    .await
                    .map_err(|e| format!("create {path}: {e}"))?;
                eprintln!("sink: file {path}");
                Ok(Sink::File(file))
            }
            Some(("serial", dev)) => Err(format!(
                "serial sink ({dev}) not implemented yet — use file: for validation"
            )),
            _ => Err(format!("bad sink {spec:?}, expected file:<path> or serial:<dev>")),
        }
    }

    async fn write_packet(&mut self, ptype: u8, body: &[u8]) -> std::io::Result<()> {
        let mut header = Vec::with_capacity(14);
        header.push(PACKET_MAGIC);
        header.push(ptype);
        header.extend_from_slice(&(body.len() as u32).to_le_bytes());
        header.extend_from_slice(&now_ms().to_le_bytes());
        match self {
            Sink::File(f) => {
                f.write_all(&header).await?;
                f.write_all(body).await?;
                f.flush().await
            }
        }
    }
}

fn build_ws_url(url: &str, format: &str, width: Option<u32>, height: Option<u32>, fps: u32) -> String {
    let mut full = format!("{url}?format={format}&fps={fps}");
    if let (Some(w), Some(h)) = (width, height) {
        full.push_str(&format!("&w={w}&h={h}"));
    }
    full
}

async fn run_stream(
    url: String,
    format: String,
    width: Option<u32>,
    height: Option<u32>,
    fps: u32,
    out: String,
) {
    let mut sink = match Sink::open(&out).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let full_url = build_ws_url(&url, &format, width, height, fps);
    eprintln!("connecting: {full_url}");

    let mut frames: u64 = 0;
    let mut bytes: u64 = 0;
    let mut stat_t = Instant::now();

    loop {
        let (mut ws, _) = match tokio_tungstenite::connect_async(&full_url).await {
            Ok(ok) => ok,
            Err(e) => {
                eprintln!("connect failed: {e}; retry in 2s");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        eprintln!("connected");

        loop {
            tokio::select! {
                msg = ws.next() => {
                    match msg {
                        Some(Ok(Message::Text(meta))) => eprintln!("meta: {meta}"),
                        Some(Ok(Message::Binary(frame))) => {
                            if let Err(e) = sink.write_packet(PACKET_FRAME, &frame).await {
                                eprintln!("sink write failed: {e}");
                                return;
                            }
                            frames += 1;
                            bytes += frame.len() as u64 + 14;
                            let dt = stat_t.elapsed().as_secs_f64();
                            if dt >= 5.0 {
                                eprintln!(
                                    "[stats] {:.1} fps, {:.1} KB/s, {frames} frames total",
                                    frames as f64 / dt.max(1e-9),
                                    bytes as f64 / dt / 1024.0
                                );
                                // per-window stats
                                frames = 0;
                                bytes = 0;
                                stat_t = Instant::now();
                            }
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => { eprintln!("ws error: {e}"); break; }
                        None => { eprintln!("ws closed"); break; }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("interrupted, sink flushed");
                    return;
                }
            }
        }
        eprintln!("reconnecting in 2s");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

// ── inspect ───────────────────────────────────────────────────────────────────

struct Packet {
    ts_ms: u64,
    body: Vec<u8>,
}

fn parse_packets(data: &[u8]) -> Result<Vec<Packet>, String> {
    let mut packets = Vec::new();
    let mut pos = 0usize;
    while pos < data.len() {
        if data.len() - pos < 14 {
            return Err(format!("truncated header at offset {pos}"));
        }
        if data[pos] != PACKET_MAGIC {
            return Err(format!("bad magic {:#04x} at offset {pos}", data[pos]));
        }
        let ptype = data[pos + 1];
        let len = u32::from_le_bytes(data[pos + 2..pos + 6].try_into().unwrap()) as usize;
        let ts_ms = u64::from_le_bytes(data[pos + 6..pos + 14].try_into().unwrap());
        if data.len() - pos - 14 < len {
            return Err(format!("truncated body at offset {pos} (need {len})"));
        }
        if ptype != PACKET_FRAME {
            return Err(format!("unexpected packet type {ptype:#04x} at offset {pos}"));
        }
        packets.push(Packet {
            ts_ms,
            body: data[pos + 14..pos + 14 + len].to_vec(),
        });
        pos += 14 + len;
    }
    Ok(packets)
}

struct FrameHeader {
    format: u8,
    w: u16,
    h: u16,
    seq: u32,
}

fn parse_frame(body: &[u8]) -> Result<(FrameHeader, &[u8]), String> {
    if body.len() < 12 || &body[0..2] != b"FR" {
        return Err("bad frame header".into());
    }
    let header = FrameHeader {
        format: body[2],
        w: u16::from_le_bytes(body[4..6].try_into().unwrap()),
        h: u16::from_le_bytes(body[6..8].try_into().unwrap()),
        seq: u32::from_le_bytes(body[8..12].try_into().unwrap()),
    };
    Ok((header, &body[12..]))
}

fn dump_frame(packet: &Packet, path: &str) -> Result<(), String> {
    let (header, pixels) = parse_frame(&packet.body)?;
    let (w, h) = (header.w as u32, header.h as u32);
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    match header.format {
        0 => {
            // rgb565 → rgb888 (expand with bit replication)
            for c in pixels.chunks_exact(2) {
                let v = u16::from_le_bytes([c[0], c[1]]);
                let r = ((v >> 11) & 0x1f) as u8;
                let g = ((v >> 5) & 0x3f) as u8;
                let b = (v & 0x1f) as u8;
                rgb.push((r << 3) | (r >> 2));
                rgb.push((g << 2) | (g >> 4));
                rgb.push((b << 3) | (b >> 2));
            }
        }
        1 => rgb.extend_from_slice(pixels),
        f => return Err(format!("unknown pixel format {f}")),
    }
    let img: image::RgbImage =
        image::ImageBuffer::from_raw(w, h, rgb).ok_or("pixel count mismatch")?;
    img.save(path).map_err(|e| e.to_string())?;
    println!("frame seq={} {}x{} → {path}", header.seq, w, h);
    Ok(())
}

fn run_inspect(file: String, dump: Option<String>) {
    let data = match std::fs::read(&file) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("read {file}: {e}");
            std::process::exit(1);
        }
    };
    let packets = match parse_packets(&data) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("parse error: {e}");
            std::process::exit(1);
        }
    };
    if packets.is_empty() {
        println!("no packets");
        return;
    }

    let mut seq_gaps = 0u32;
    let mut prev_seq: Option<u32> = None;
    let mut bad = 0u32;
    let mut dims = String::new();
    for p in &packets {
        match parse_frame(&p.body) {
            Ok((h, pixels)) => {
                let expect = h.w as usize * h.h as usize * if h.format == 0 { 2 } else { 3 };
                if pixels.len() != expect {
                    bad += 1;
                }
                if let Some(prev) = prev_seq {
                    if h.seq != prev.wrapping_add(1) {
                        seq_gaps += 1;
                    }
                }
                prev_seq = Some(h.seq);
                if dims.is_empty() {
                    let fmt = if h.format == 0 { "rgb565" } else { "rgb888" };
                    dims = format!("{}x{} {fmt}", h.w, h.h);
                }
            }
            Err(_) => bad += 1,
        }
    }

    let span_ms = packets.last().unwrap().ts_ms - packets.first().unwrap().ts_ms;
    let fps = if span_ms > 0 {
        (packets.len() as f64 - 1.0) / (span_ms as f64 / 1000.0)
    } else {
        0.0
    };
    println!("file:      {file} ({} bytes)", data.len());
    println!("packets:   {} ({dims})", packets.len());
    println!("duration:  {:.1}s → {:.1} fps", span_ms as f64 / 1000.0, fps);
    println!("integrity: {bad} bad frames, {seq_gaps} seq gaps");

    if let Some(spec) = dump {
        let Some((idx, path)) = spec.split_once(':') else {
            eprintln!("--dump expects <index>:<out.png>");
            std::process::exit(1);
        };
        let idx: usize = idx.parse().unwrap_or(0);
        let Some(packet) = packets.get(idx) else {
            eprintln!("no packet {idx} (have {})", packets.len());
            std::process::exit(1);
        };
        if let Err(e) = dump_frame(packet, path) {
            eprintln!("dump failed: {e}");
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    match Args::parse().cmd {
        Cmd::Stream { url, format, width, height, fps, out } => {
            run_stream(url, format, width, height, fps, out).await
        }
        Cmd::Inspect { file, dump } => run_inspect(file, dump),
    }
}
