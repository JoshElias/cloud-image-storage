use std::net::SocketAddr;

use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::domain::{
    PresignUploadRequest, PresignUploadResponse, RestoreRequest, RestoreState, RestoreStatus,
    StartIngestRequest, StartIngestResponse,
};
use crate::ui::{self, Page};

#[derive(Clone)]
struct AppState {
    config: AppConfig,
}

pub async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let address: SocketAddr = config.server.bind_address.parse()?;
    let state = AppState { config };

    let app = Router::new()
        .route("/", get(home))
        .route("/uploads", get(uploads))
        .route("/restores", get(restores))
        .route("/healthz", get(healthz))
        .route("/api/ingests", post(start_ingest))
        .route("/api/uploads/presign", post(presign_upload))
        .route("/api/restores", post(create_restore))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!("serving admin UI on http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn home(State(state): State<AppState>) -> Html<String> {
    Html(ui::render_page(Page::Home, &state.config))
}

async fn uploads(State(state): State<AppState>) -> Html<String> {
    Html(ui::render_page(Page::Uploads, &state.config))
}

async fn restores(State(state): State<AppState>) -> Html<String> {
    Html(ui::render_page(Page::Restores, &state.config))
}

async fn healthz() -> impl IntoResponse {
    "ok"
}

async fn start_ingest(Json(_request): Json<StartIngestRequest>) -> Json<StartIngestResponse> {
    Json(StartIngestResponse {
        ingest_id: Uuid::new_v4(),
    })
}

async fn presign_upload(Json(request): Json<PresignUploadRequest>) -> Json<PresignUploadResponse> {
    let extension = mime_guess::get_mime_extensions_str(&request.mime_type)
        .and_then(|extensions| extensions.first())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();

    Json(PresignUploadResponse {
        original_key: format!("originals/{}{}", request.sha256, extension),
        preview_key: format!("previews/{}{}", request.sha256, extension),
        metadata_key: format!("metadata/raw/{}.json", request.sha256),
    })
}

async fn create_restore(Json(request): Json<RestoreRequest>) -> Json<RestoreStatus> {
    let restore_days = request.restore_days.max(1);
    tracing::info!(restore_days, "queued restore request");

    Json(RestoreStatus {
        restore_id: Uuid::new_v4(),
        state: RestoreState::Queued,
        requested_at: Utc::now(),
    })
}
