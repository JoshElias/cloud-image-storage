use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IngestSource {
    GoogleTakeout,
    ManualUpload,
}

impl std::fmt::Display for IngestSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GoogleTakeout => formatter.write_str("google_takeout"),
            Self::ManualUpload => formatter.write_str("manual_upload"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestoreState {
    Queued,
    Restoring,
    Restored,
    Expired,
    Failed,
    PinnedHot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IngestManifest {
    pub ingest_id: Uuid,
    pub source: IngestSource,
    pub collection_name: String,
    pub server: Option<String>,
    pub assets: Vec<IngestAsset>,
    pub duplicates_skipped: usize,
    pub total_bytes: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IngestAsset {
    pub source_path: Utf8PathBuf,
    pub sha256: String,
    pub bytes: u64,
    pub mime_type: String,
    pub original_key: String,
    pub preview_key: String,
    pub metadata_key: String,
    pub sidecar_path: Option<Utf8PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StartIngestRequest {
    pub source: IngestSource,
    pub collection_name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StartIngestResponse {
    pub ingest_id: Uuid,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PresignUploadRequest {
    pub ingest_id: Uuid,
    pub sha256: String,
    pub bytes: u64,
    pub mime_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PresignUploadResponse {
    pub original_key: String,
    pub preview_key: String,
    pub metadata_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestoreRequest {
    pub collection_name: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub restore_days: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestoreStatus {
    pub restore_id: Uuid,
    pub state: RestoreState,
    pub requested_at: DateTime<Utc>,
}
