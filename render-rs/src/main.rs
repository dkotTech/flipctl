//! render-rs — headless HTML renderer for flipctl.
//!
//! Renders web pages without a display and streams them as video (PNG frames)
//! over WebSocket. The engine is chosen at build time: WPE WebKit (default,
//! C FFI) or Servo (pure Rust) — see Cargo features.
//!
//! - Every render loads its HTML from an HTTP endpoint (e.g. a flipctl
//!   backend app port) and can be remote-controlled through the API:
//!   keyboard, mouse and wheel events.
//! - Multiple renders run side by side; start them with repeated `--url`
//!   flags or create/destroy them at runtime via `POST/DELETE /api/renders`.
//! - `GET /` serves a management UI, `GET /view/{id}` an interactive viewer.

mod api;
mod encode;
mod engine;
mod types;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use clap::Parser;

use api::{ApiState, CreateRender};

#[derive(Parser)]
#[command(name = "render-rs", about = "Headless HTML renderer: WebSocket video + remote control")]
struct Args {
    /// Control API / viewer port
    #[arg(long, default_value_t = 8090)]
    port: u16,

    /// Endpoint(s) to render at startup, e.g. --url http://localhost:5173/
    /// (repeat for multiple renders)
    #[arg(long)]
    url: Vec<String>,

    /// Default viewport width
    #[arg(long, default_value_t = 640)]
    width: u32,

    /// Default viewport height
    #[arg(long, default_value_t = 480)]
    height: u32,

    /// Frame rate cap per render
    #[arg(long, default_value_t = 30)]
    fps: u32,
}

fn main() {
    let args = Args::parse();

    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
    let state = ApiState {
        registry: Arc::new(Mutex::new(HashMap::new())),
        cmd_tx: Arc::new(Mutex::new(cmd_tx)),
        engine: engine::NAME,
    };

    // API server on a background thread. The command-line `--url` renders are
    // created *inside* the runtime (spawn_render starts a tokio ticker task, so
    // it must run within a runtime context), before serving begins.
    {
        let state = state.clone();
        let port = args.port;
        let urls = args.url.clone();
        let (width, height, fps) = (args.width, args.height, args.fps);
        std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async move {
                for url in urls {
                    api::spawn_render(&state, CreateRender { url, width, height, fps });
                }
                api::serve(port, state).await;
            });
        });
    }

    eprintln!("engine: {}", engine::NAME);
    // …the engine owns the main thread (GLib loop / servo event loop).
    engine::run(cmd_rx);
}
