use std::net::SocketAddr;
use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::db::Database;
use crate::domain::{
    PresignUploadRequest, PresignUploadResponse, RestoreRequest, RestoreState, RestoreStatus,
    StartIngestRequest, StartIngestResponse,
};
use crate::storage::Storage;
use crate::ui::{self, Page};
use crate::upload_api::{
    BrowserUploadCompleteRequest, BrowserUploadCompleteResponse, BrowserUploadPresignRequest,
    BrowserUploadPresignResponse, BrowserUploadTarget, UploadHistoryResponse,
};

#[derive(Clone)]
struct AppState {
    config: AppConfig,
    database: Database,
    storage: Storage,
}

pub async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let address: SocketAddr = config.server.bind_address.parse()?;
    let database = Database::connect(&config.database.url).await?;
    let storage = Storage::connect(&config.storage).await;
    let site_pkg = site_pkg_path();
    let state = AppState {
        config,
        database,
        storage,
    };

    let app = Router::new()
        .route("/", get(home))
        .route("/uploads", get(uploads))
        .route("/restores", get(restores))
        .route("/healthz", get(healthz))
        .route("/api/ingests", post(start_ingest))
        .route("/api/uploads/browser/history", get(upload_history))
        .route("/api/uploads/browser/presign", post(presign_browser_upload))
        .route(
            "/api/uploads/browser/{ingest_id}/complete",
            post(complete_browser_upload),
        )
        .route("/api/uploads/presign", post(presign_upload))
        .route("/api/restores", post(create_restore))
        .nest_service("/pkg", ServeDir::new(site_pkg))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!("serving admin UI on http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn site_pkg_path() -> &'static str {
    if FsPath::new("/srv/cis/site/pkg").exists() {
        "/srv/cis/site/pkg"
    } else {
        "target/site/pkg"
    }
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

async fn upload_history(
    State(state): State<AppState>,
) -> Result<Json<UploadHistoryResponse>, AppError> {
    let runs = state.database.upload_history().await?;
    Ok(Json(UploadHistoryResponse { runs }))
}

async fn presign_browser_upload(
    State(state): State<AppState>,
    Json(request): Json<BrowserUploadPresignRequest>,
) -> Result<Json<BrowserUploadPresignResponse>, AppError> {
    let run = state
        .database
        .create_browser_upload(
            request.collection_name.trim(),
            &request.files,
            &state.config.storage,
        )
        .await?;
    let mut files = Vec::with_capacity(run.files.len());

    for file in run.files {
        let already_uploaded = if file.already_uploaded {
            state.storage.object_exists(&file.original_key).await?
                && state.storage.object_exists(&file.metadata_key).await?
        } else {
            false
        };
        let original_upload = state
            .storage
            .presign_original_upload(&file.original_key, &file.mime_type)
            .await?;
        let metadata_upload = state
            .storage
            .presign_metadata_upload(&file.metadata_key)
            .await?;

        files.push(BrowserUploadTarget {
            name: file.name,
            sha256: file.sha256,
            original_key: file.original_key,
            metadata_key: file.metadata_key,
            original_upload_url: original_upload.url,
            metadata_upload_url: metadata_upload.url,
            original_headers: original_upload.headers,
            metadata_headers: metadata_upload.headers,
            already_uploaded,
        });
    }

    Ok(Json(BrowserUploadPresignResponse {
        ingest_id: run.ingest_id.to_string(),
        files,
    }))
}

async fn complete_browser_upload(
    State(state): State<AppState>,
    Path(ingest_id): Path<Uuid>,
    Json(request): Json<BrowserUploadCompleteRequest>,
) -> Result<Json<BrowserUploadCompleteResponse>, AppError> {
    let upload_keys = state
        .database
        .upload_keys_for_run(ingest_id, &request.uploaded_sha256)
        .await?;
    let mut verified_sha256 = Vec::new();

    for (sha256, original_key, metadata_key) in upload_keys {
        let original_exists = state.storage.object_exists(&original_key).await?;
        let metadata_exists = state.storage.object_exists(&metadata_key).await?;

        if original_exists && metadata_exists {
            verified_sha256.push(sha256);
        }
    }

    let (completed_assets, total_assets, status) = state
        .database
        .complete_browser_upload(ingest_id, &verified_sha256)
        .await?;

    Ok(Json(BrowserUploadCompleteResponse {
        completed_assets,
        total_assets,
        status,
    }))
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

struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!(error = %self.0, "request failed");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "request failed"
            })),
        )
            .into_response()
    }
}
