use axum::{extract::State, http::Method, http::StatusCode, routing::post, Json, Router};
use clap::Parser;
use lol_html::{element, html_content::ContentType, HtmlRewriter, Settings};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use tower_http::cors::{Any, CorsLayer};

#[derive(Parser)]
#[command(name = "florence", about = "Bidirectional HTML source patcher")]
struct Cli {
    /// Port to listen on
    #[arg(long, short, default_value = "3001")]
    port: u16,

    /// Root directory containing source files
    #[arg(long, short, default_value = "./website/src")]
    root: PathBuf,
}

#[derive(Clone)]
struct AppState {
    root: PathBuf,
}

#[derive(Deserialize)]
struct Patch {
    /// File path relative to root (e.g. "index.html")
    file: String,
    /// Value of data-flo-id on the target element
    id: String,
    /// New text content
    content: String,
}

#[derive(Serialize)]
struct PatchResponse {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn patch_handler(
    State(state): State<Arc<AppState>>,
    Json(patch): Json<Patch>,
) -> (StatusCode, Json<PatchResponse>) {
    match apply_patch(&state.root, &patch) {
        Ok(()) => (
            StatusCode::OK,
            Json(PatchResponse { ok: true, error: None }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(PatchResponse {
                ok: false,
                error: Some(e),
            }),
        ),
    }
}

fn apply_patch(root: &PathBuf, patch: &Patch) -> Result<(), String> {
    let path = root.join(&patch.file);

    // Prevent path traversal
    let root_canon = root
        .canonicalize()
        .map_err(|e| format!("invalid root: {e}"))?;
    let path_canon = path
        .canonicalize()
        .map_err(|e| format!("file not found: {e}"))?;
    if !path_canon.starts_with(&root_canon) {
        return Err("invalid file path".into());
    }

    let html = std::fs::read_to_string(&path_canon).map_err(|e| e.to_string())?;
    let selector = format!("[data-flo-id=\"{}\"]", patch.id);
    let content = patch.content.clone();

    let mut output = Vec::new();
    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![element!(&selector, |el| {
                el.set_inner_content(&content, ContentType::Html);
                Ok(())
            })],
            ..Settings::default()
        },
        |chunk: &[u8]| output.extend_from_slice(chunk),
    );

    rewriter
        .write(html.as_bytes())
        .map_err(|e| e.to_string())?;
    rewriter.end().map_err(|e| e.to_string())?;

    std::fs::write(&path_canon, &output).map_err(|e| e.to_string())?;
    Ok(())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let state = Arc::new(AppState {
        root: cli.root.clone(),
    });

    let cors = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_headers(Any)
        .allow_origin(Any);

    let app = Router::new()
        .route("/patch", post(patch_handler))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cli.port);
    println!("florence patcher → http://{addr}");
    println!("root → {}", cli.root.display());

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
