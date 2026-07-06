//! flipctl backend.
//!
//! - Serves frontend apps packed as zip archives from the `apps/` directory,
//!   one HTTP port per app (`main` on 5173, the rest on 5174+), SPA-style.
//! - Runs whatever command a frontend asks for via `POST /api/exec` and
//!   returns the output unchanged; the frontend decides what to run and how to
//!   parse the result. Commands are spawned directly (no shell), so args are
//!   passed as an array and never re-interpreted. The API is available on every
//!   app port, so frontends talk to it same-origin.

use std::{
    collections::BTreeMap,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::{
    extract::State,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;

const MAIN_PORT: u16 = 5173;
const EXEC_TIMEOUT: Duration = Duration::from_secs(30);

struct AppBundle {
    id: String,
    manifest: Value,
    port: u16,
    files: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone)]
struct AppState {
    apps: Arc<Vec<Arc<AppBundle>>>,
    bundle: Arc<AppBundle>,
}

fn load_bundle(path: &Path) -> Result<(String, Value, BTreeMap<String, Vec<u8>>), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open {path:?}: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("read {path:?}: {e}"))?;

    let mut files = BTreeMap::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        files.insert(entry.name().to_string(), data);
    }

    let manifest: Value = serde_json::from_slice(
        files
            .get("flipctl.json")
            .ok_or_else(|| format!("{path:?}: no flipctl.json in archive"))?,
    )
    .map_err(|e| format!("{path:?}: bad flipctl.json: {e}"))?;

    let id = manifest["id"]
        .as_str()
        .ok_or_else(|| format!("{path:?}: manifest has no id"))?
        .to_string();

    Ok((id, manifest, files))
}

fn load_apps(dir: &Path) -> Vec<Arc<AppBundle>> {
    let mut loaded = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("cannot read apps dir {dir:?}: {e}");
            std::process::exit(1);
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "zip") {
            match load_bundle(&path) {
                Ok(bundle) => loaded.push(bundle),
                Err(e) => eprintln!("skip {path:?}: {e}"),
            }
        }
    }

    // `main` always gets MAIN_PORT, the rest follow in alphabetical order.
    loaded.sort_by(|a, b| {
        let rank = |id: &str| if id == "main" { 0 } else { 1 };
        (rank(&a.0), a.0.clone()).cmp(&(rank(&b.0), b.0.clone()))
    });

    loaded
        .into_iter()
        .enumerate()
        .map(|(i, (id, manifest, files))| {
            Arc::new(AppBundle {
                id,
                manifest,
                port: MAIN_PORT + i as u16,
                files,
            })
        })
        .collect()
}

async fn list_apps(State(state): State<AppState>) -> Json<Value> {
    let apps: Vec<Value> = state
        .apps
        .iter()
        .map(|app| {
            let mut m = app.manifest.clone();
            m["port"] = json!(app.port);
            m["url"] = json!(format!("http://localhost:{}/", app.port));
            m
        })
        .collect();
    Json(json!(apps))
}

#[derive(Deserialize)]
struct ExecRequest {
    cmd: String,
    #[serde(default)]
    args: Vec<String>,
}

#[derive(Serialize)]
struct ExecResponse {
    cmd: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Runs the command the frontend asked for and returns its output verbatim.
/// The binary is spawned directly with `args` as a plain argv array — no shell
/// is involved, so nothing in the request is re-interpreted or expanded.
async fn exec(Json(req): Json<ExecRequest>) -> Response {
    if req.cmd.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "empty cmd" })),
        )
            .into_response();
    }

    let mut command = tokio::process::Command::new(&req.cmd);
    command.args(&req.args).kill_on_drop(true);

    let output = match tokio::time::timeout(EXEC_TIMEOUT, command.output()).await {
        Err(_) => {
            return (
                StatusCode::GATEWAY_TIMEOUT,
                Json(json!({ "error": format!("{} timed out", req.cmd) })),
            )
                .into_response()
        }
        Ok(Err(e)) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("spawn {}: {e}", req.cmd) })),
            )
                .into_response()
        }
        Ok(Ok(output)) => output,
    };

    Json(ExecResponse {
        cmd: req.cmd,
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
    .into_response()
}

async fn serve_file(State(state): State<AppState>, uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    // SPA fallback: unknown paths get index.html, same as go-server did.
    let (name, data) = match state.bundle.files.get(path) {
        Some(data) => (path, data),
        None => match state.bundle.files.get("index.html") {
            Some(data) => ("index.html", data),
            None => return (StatusCode::NOT_FOUND, "not found").into_response(),
        },
    };

    let mime = mime_guess::from_path(name).first_or_octet_stream();
    (
        [(header::CONTENT_TYPE, mime.to_string())],
        data.clone(),
    )
        .into_response()
}

#[tokio::main]
async fn main() {
    let apps_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../apps"));

    let apps = Arc::new(load_apps(&apps_dir));
    if apps.is_empty() {
        eprintln!("no app bundles found in {apps_dir:?}");
        std::process::exit(1);
    }

    let mut handles = Vec::new();
    for bundle in apps.iter() {
        let state = AppState {
            apps: apps.clone(),
            bundle: bundle.clone(),
        };
        let router = Router::new()
            .route("/api/apps", get(list_apps))
            .route("/api/exec", post(exec))
            .fallback(get(serve_file))
            .layer(CorsLayer::permissive())
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], bundle.port));
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("bind {addr}: {e}");
                std::process::exit(1);
            }
        };
        println!("{:<10} → http://localhost:{}", bundle.id, bundle.port);
        handles.push(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}
