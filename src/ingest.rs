use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::domain::{IngestAsset, IngestManifest, IngestSource};

pub fn scan_takeout(
    root: &Utf8Path,
    collection_name: String,
    server: Option<String>,
) -> anyhow::Result<IngestManifest> {
    scan(root, collection_name, server, IngestSource::GoogleTakeout)
}

pub fn scan_manual_upload(
    root: &Utf8Path,
    collection_name: String,
    server: Option<String>,
) -> anyhow::Result<IngestManifest> {
    scan(root, collection_name, server, IngestSource::ManualUpload)
}

fn scan(
    root: &Utf8Path,
    collection_name: String,
    server: Option<String>,
    source: IngestSource,
) -> anyhow::Result<IngestManifest> {
    let mut seen = HashSet::new();
    let mut assets = Vec::new();
    let mut duplicates_skipped = 0;
    let mut total_bytes = 0;

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
            .map_err(|path| anyhow::anyhow!("path is not valid UTF-8: {}", path.display()))?;

        if !is_supported_image(&path) {
            continue;
        }

        let sha256 = hash_file(&path)?;
        if !seen.insert(sha256.clone()) {
            duplicates_skipped += 1;
            continue;
        }

        let metadata = entry.metadata()?;
        total_bytes += metadata.len();
        let mime_type = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .essence_str()
            .to_string();
        let original_key = object_key("originals", &sha256, &path);
        let preview_key = object_key("previews", &sha256, &path);
        let metadata_key = format!("metadata/raw/{sha256}.json");

        assets.push(IngestAsset {
            source_path: path.clone(),
            sha256,
            bytes: metadata.len(),
            mime_type,
            original_key,
            preview_key,
            metadata_key,
            sidecar_path: find_takeout_sidecar(&path),
        });
    }

    Ok(IngestManifest {
        ingest_id: Uuid::new_v4(),
        source,
        collection_name,
        server,
        assets,
        duplicates_skipped,
        total_bytes,
        created_at: Utc::now(),
    })
}

fn hash_file(path: &Utf8Path) -> anyhow::Result<String> {
    let file = File::open(path).with_context(|| format!("open {path}"))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn is_supported_image(path: &Utf8Path) -> bool {
    let Some(extension) = path.extension() else {
        return false;
    };

    matches!(
        extension.to_ascii_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "heic" | "heif" | "gif" | "avif" | "dng"
    )
}

fn object_key(prefix: &str, sha256: &str, path: &Utf8Path) -> String {
    let extension = path
        .extension()
        .map(|extension| format!(".{}", extension.to_ascii_lowercase()))
        .unwrap_or_default();

    format!("{prefix}/{sha256}{extension}")
}

fn find_takeout_sidecar(path: &Utf8Path) -> Option<Utf8PathBuf> {
    let file_name = path.file_name()?;
    let same_name = path.with_file_name(format!("{file_name}.json"));
    if same_name.exists() {
        return Some(same_name);
    }

    let stem = path.file_stem()?;
    let stem_json = path.with_file_name(format!("{stem}.json"));
    if stem_json.exists() {
        return Some(stem_json);
    }

    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use camino::Utf8Path;

    use super::{find_takeout_sidecar, is_supported_image, scan_manual_upload};

    #[test]
    fn supported_image_extensions_are_case_insensitive() {
        assert!(is_supported_image(Utf8Path::new("IMG_0001.JPG")));
        assert!(is_supported_image(Utf8Path::new("IMG_0002.heic")));
        assert!(!is_supported_image(Utf8Path::new("notes.txt")));
    }

    #[test]
    fn scan_skips_duplicate_file_content() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let first = temp.path().join("a.jpg");
        let second = temp.path().join("nested/b.jpg");
        fs::create_dir_all(second.parent().expect("nested parent exists"))?;
        fs::write(&first, b"same image bytes")?;
        fs::write(&second, b"same image bytes")?;

        let root = Utf8Path::from_path(temp.path()).expect("temp path is utf8");
        let manifest = scan_manual_upload(root, "manual".to_string(), None)?;

        assert_eq!(manifest.assets.len(), 1);
        assert_eq!(manifest.duplicates_skipped, 1);
        Ok(())
    }

    #[test]
    fn sidecar_detection_accepts_takeout_name_patterns() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let image = temp.path().join("IMG_0001.JPG");
        let sidecar = temp.path().join("IMG_0001.JPG.json");
        fs::write(&image, b"image")?;
        fs::write(&sidecar, b"{}")?;

        let image = Utf8Path::from_path(&image).expect("temp path is utf8");
        assert_eq!(
            find_takeout_sidecar(image).as_deref(),
            Some(image.with_file_name("IMG_0001.JPG.json").as_path())
        );
        Ok(())
    }
}
