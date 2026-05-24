use std::net::SocketAddr;
use std::path::Path as FsPath;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect};
use axum::routing::{get, post};
use axum::{Form, Json, Router};
use chrono::Utc;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::auth::{self, LoginForm};
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
        .route("/login", get(login).post(create_login))
        .route("/logout", post(logout))
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

async fn home(headers: HeaderMap, State(state): State<AppState>) -> Result<Html<String>, AppError> {
    require_page_auth(&state, &headers).await?;
    Ok(Html(ui::render_page(Page::Home, &state.config)))
}

async fn uploads(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    require_page_auth(&state, &headers).await?;
    Ok(Html(ui::render_page(Page::Uploads, &state.config)))
}

async fn restores(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    require_page_auth(&state, &headers).await?;
    Ok(Html(ui::render_page(Page::Restores, &state.config)))
}

async fn healthz() -> impl IntoResponse {
    "ok"
}

async fn login(State(state): State<AppState>) -> impl IntoResponse {
    if !state.config.auth.enabled() {
        return Redirect::to("/").into_response();
    }

    Html(ui::render_login_page(None)).into_response()
}

async fn create_login(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, AppError> {
    if !state.config.auth.enabled() {
        return Ok(Redirect::to("/").into_response());
    }

    let username_matches = form.username == state.config.auth.admin_username;
    let password_matches = auth::verify_password(&form.password, &state.config.auth.password_hash)?;

    if !username_matches || !password_matches {
        return Ok((
            StatusCode::UNAUTHORIZED,
            Html(ui::render_login_page(Some("Invalid username or password."))),
        )
            .into_response());
    }

    let session = auth::new_session(&state.config.auth);
    state
        .database
        .create_auth_session(&session.token_hash, session.expires_at)
        .await?;

    let mut headers = HeaderMap::new();
    auth::set_session_cookie(&mut headers, &state.config.auth, &session.token)?;

    Ok((headers, Redirect::to("/")).into_response())
}

async fn logout(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(token) = auth::session_cookie(&headers, &state.config.auth) {
        let token_hash = auth::hash_session_token(&token);
        state.database.delete_auth_session(&token_hash).await?;
    }

    let mut response_headers = HeaderMap::new();
    auth::clear_session_cookie(&mut response_headers, &state.config.auth)?;

    Ok((response_headers, Redirect::to("/login")).into_response())
}

async fn start_ingest(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(_request): Json<StartIngestRequest>,
) -> Result<Json<StartIngestResponse>, AppError> {
    require_api_auth(&state, &headers).await?;

    Ok(Json(StartIngestResponse {
        ingest_id: Uuid::new_v4(),
    }))
}

async fn presign_upload(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<PresignUploadRequest>,
) -> Result<Json<PresignUploadResponse>, AppError> {
    require_api_auth(&state, &headers).await?;

    let extension = mime_guess::get_mime_extensions_str(&request.mime_type)
        .and_then(|extensions| extensions.first())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_default();

    Ok(Json(PresignUploadResponse {
        original_key: format!("originals/{}{}", request.sha256, extension),
        preview_key: format!("previews/{}{}", request.sha256, extension),
        metadata_key: format!("metadata/raw/{}.json", request.sha256),
    }))
}

async fn upload_history(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<UploadHistoryResponse>, AppError> {
    require_api_auth(&state, &headers).await?;

    let runs = state.database.upload_history().await?;
    Ok(Json(UploadHistoryResponse { runs }))
}

async fn presign_browser_upload(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<BrowserUploadPresignRequest>,
) -> Result<Json<BrowserUploadPresignResponse>, AppError> {
    require_api_auth(&state, &headers).await?;

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
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(ingest_id): Path<Uuid>,
    Json(request): Json<BrowserUploadCompleteRequest>,
) -> Result<Json<BrowserUploadCompleteResponse>, AppError> {
    require_api_auth(&state, &headers).await?;

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

async fn create_restore(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(request): Json<RestoreRequest>,
) -> Result<Json<RestoreStatus>, AppError> {
    require_api_auth(&state, &headers).await?;

    let restore_days = request.restore_days.max(1);
    tracing::info!(restore_days, "queued restore request");

    Ok(Json(RestoreStatus {
        restore_id: Uuid::new_v4(),
        state: RestoreState::Queued,
        requested_at: Utc::now(),
    }))
}

async fn require_page_auth(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    if is_authenticated(state, headers).await? {
        return Ok(());
    }

    Err(AppError::Redirect("/login"))
}

async fn require_api_auth(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    if is_authenticated(state, headers).await? {
        return Ok(());
    }

    Err(AppError::Unauthorized)
}

async fn is_authenticated(state: &AppState, headers: &HeaderMap) -> Result<bool, AppError> {
    if !state.config.auth.enabled() {
        return Ok(true);
    }

    let Some(token) = auth::session_cookie(headers, &state.config.auth) else {
        return Ok(false);
    };
    let token_hash = auth::hash_session_token(&token);

    Ok(state.database.auth_session_exists(&token_hash).await?)
}

enum AppError {
    Internal(anyhow::Error),
    Redirect(&'static str),
    Unauthorized,
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Internal(error) => {
                tracing::error!(error = %error, "request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": "request failed"
                    })),
                )
                    .into_response()
            }
            Self::Redirect(path) => Redirect::to(path).into_response(),
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "unauthorized"
                })),
            )
                .into_response(),
        }
    }
}
