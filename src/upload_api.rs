#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowserUploadFile {
    pub name: String,
    pub sha256: String,
    pub bytes: u64,
    pub mime_type: String,
}

impl BrowserUploadFile {
    pub fn extension(&self) -> String {
        self.name
            .rsplit_once('.')
            .map(|(_, extension)| format!(".{}", extension.to_ascii_lowercase()))
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowserUploadPresignRequest {
    pub collection_name: String,
    pub files: Vec<BrowserUploadFile>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowserUploadPresignResponse {
    pub ingest_id: String,
    pub files: Vec<BrowserUploadTarget>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowserUploadTarget {
    pub name: String,
    pub sha256: String,
    pub original_key: String,
    pub metadata_key: String,
    pub original_upload_url: String,
    pub metadata_upload_url: String,
    pub original_headers: Vec<UploadHeader>,
    pub metadata_headers: Vec<UploadHeader>,
    pub already_uploaded: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UploadHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowserUploadCompleteRequest {
    pub uploaded_sha256: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BrowserUploadCompleteResponse {
    pub completed_assets: u32,
    pub total_assets: u32,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UploadHistoryResponse {
    pub runs: Vec<UploadHistoryRun>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct UploadHistoryRun {
    pub ingest_id: String,
    pub collection_name: String,
    pub source: String,
    pub status: String,
    pub total_assets: i32,
    pub completed_assets: i32,
    pub total_bytes: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::BrowserUploadFile;

    #[test]
    fn upload_file_extension_is_lowercase() {
        let file = BrowserUploadFile {
            name: "IMG_0001.HEIC".to_string(),
            sha256: "abc123".to_string(),
            bytes: 42,
            mime_type: "image/heic".to_string(),
        };

        assert_eq!(file.extension(), ".heic");
    }

    #[test]
    fn upload_file_without_extension_has_empty_extension() {
        let file = BrowserUploadFile {
            name: "IMG_0001".to_string(),
            sha256: "abc123".to_string(),
            bytes: 42,
            mime_type: "application/octet-stream".to_string(),
        };

        assert_eq!(file.extension(), "");
    }
}
