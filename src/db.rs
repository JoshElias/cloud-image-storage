use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::config::StorageConfig;
use crate::upload_api::{BrowserUploadFile, UploadHistoryRun};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

#[derive(Clone, Debug)]
pub struct UploadAssetRecord {
    pub name: String,
    pub sha256: String,
    pub mime_type: String,
    pub original_key: String,
    pub metadata_key: String,
    pub already_uploaded: bool,
}

#[derive(Clone, Debug)]
pub struct BrowserUploadRun {
    pub ingest_id: Uuid,
    pub files: Vec<UploadAssetRecord>,
}

impl Database {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn create_browser_upload(
        &self,
        collection_name: &str,
        files: &[BrowserUploadFile],
        storage: &StorageConfig,
    ) -> anyhow::Result<BrowserUploadRun> {
        let ingest_id = Uuid::new_v4();
        let collection_id = Uuid::new_v4();
        let total_assets = i32::try_from(files.len())?;
        let mut total_bytes = 0_i64;
        for file in files {
            total_bytes += i64::try_from(file.bytes)?;
        }
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO ingest_runs (id, source, collection_name, status, total_assets, total_bytes)
            VALUES ($1, 'manual_upload', $2, 'pending', $3, $4)
            "#,
        )
        .bind(ingest_id)
        .bind(collection_name)
        .bind(total_assets)
        .bind(total_bytes)
        .execute(&mut *transaction)
        .await?;

        let collection_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO collections (id, name, source)
            VALUES ($1, $2, 'manual_upload')
            ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
            RETURNING id
            "#,
        )
        .bind(collection_id)
        .bind(collection_name)
        .fetch_one(&mut *transaction)
        .await?;

        let mut targets = Vec::with_capacity(files.len());
        for file in files {
            let existing = sqlx::query(
                r#"
                SELECT id, original_key, metadata_key
                FROM assets
                WHERE sha256 = $1
                "#,
            )
            .bind(&file.sha256)
            .fetch_optional(&mut *transaction)
            .await?;

            let (asset_id, original_key, metadata_key, already_uploaded) =
                if let Some(row) = existing {
                    (
                        row.get::<Uuid, _>("id"),
                        row.get::<String, _>("original_key"),
                        row.get::<String, _>("metadata_key"),
                        true,
                    )
                } else {
                    let asset_id = Uuid::new_v4();
                    let original_key = format!(
                        "{}{}{}",
                        storage.originals_prefix,
                        file.sha256,
                        file.extension()
                    );
                    let preview_key = format!(
                        "{}{}{}",
                        storage.previews_prefix,
                        file.sha256,
                        file.extension()
                    );
                    let metadata_key = format!("{}{}.json", storage.metadata_prefix, file.sha256);

                    sqlx::query(
                        r#"
                        INSERT INTO assets (
                            id,
                            sha256,
                            original_key,
                            preview_key,
                            metadata_key,
                            mime_type,
                            byte_size
                        )
                        VALUES ($1, $2, $3, $4, $5, $6, $7)
                        "#,
                    )
                    .bind(asset_id)
                    .bind(&file.sha256)
                    .bind(&original_key)
                    .bind(&preview_key)
                    .bind(&metadata_key)
                    .bind(&file.mime_type)
                    .bind(i64::try_from(file.bytes)?)
                    .execute(&mut *transaction)
                    .await?;

                    (asset_id, original_key, metadata_key, false)
                };

            sqlx::query(
                r#"
                INSERT INTO collection_assets (collection_id, asset_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(collection_id)
            .bind(asset_id)
            .execute(&mut *transaction)
            .await?;

            sqlx::query(
                r#"
                INSERT INTO ingest_run_assets (
                    ingest_run_id,
                    asset_id
                )
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(ingest_id)
            .bind(asset_id)
            .execute(&mut *transaction)
            .await?;

            targets.push(UploadAssetRecord {
                name: file.name.clone(),
                sha256: file.sha256.clone(),
                mime_type: file.mime_type.clone(),
                original_key,
                metadata_key,
                already_uploaded,
            });
        }

        transaction.commit().await?;

        Ok(BrowserUploadRun {
            ingest_id,
            files: targets,
        })
    }

    pub async fn upload_keys_for_run(
        &self,
        ingest_id: Uuid,
        sha256_values: &[String],
    ) -> anyhow::Result<Vec<(String, String, String)>> {
        let rows = sqlx::query(
            r#"
            SELECT assets.sha256, assets.original_key, assets.metadata_key
            FROM ingest_run_assets
            JOIN assets ON assets.id = ingest_run_assets.asset_id
            WHERE ingest_run_assets.ingest_run_id = $1
              AND assets.sha256 = ANY($2)
            "#,
        )
        .bind(ingest_id)
        .bind(sha256_values)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                (
                    row.get::<String, _>("sha256"),
                    row.get::<String, _>("original_key"),
                    row.get::<String, _>("metadata_key"),
                )
            })
            .collect())
    }

    pub async fn complete_browser_upload(
        &self,
        ingest_id: Uuid,
        uploaded_sha256: &[String],
    ) -> anyhow::Result<(u32, u32, String)> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE ingest_run_assets
            SET original_uploaded_at = COALESCE(original_uploaded_at, now()),
                metadata_uploaded_at = COALESCE(metadata_uploaded_at, now())
            FROM assets
            WHERE assets.id = ingest_run_assets.asset_id
              AND ingest_run_assets.ingest_run_id = $1
              AND assets.sha256 = ANY($2)
            "#,
        )
        .bind(ingest_id)
        .bind(uploaded_sha256)
        .execute(&mut *transaction)
        .await?;

        let completed_assets: i32 = sqlx::query_scalar(
            r#"
            SELECT count(*)::integer
            FROM ingest_run_assets
            WHERE ingest_run_id = $1
              AND original_uploaded_at IS NOT NULL
              AND metadata_uploaded_at IS NOT NULL
            "#,
        )
        .bind(ingest_id)
        .fetch_one(&mut *transaction)
        .await?;

        let total_assets: i32 =
            sqlx::query_scalar("SELECT total_assets FROM ingest_runs WHERE id = $1")
                .bind(ingest_id)
                .fetch_one(&mut *transaction)
                .await?;

        let status = if completed_assets >= total_assets {
            "complete"
        } else {
            "pending"
        };

        sqlx::query(
            r#"
            UPDATE ingest_runs
            SET completed_assets = $2,
                status = $3,
                finished_at = CASE WHEN $3 = 'complete' THEN now() ELSE finished_at END
            WHERE id = $1
            "#,
        )
        .bind(ingest_id)
        .bind(completed_assets)
        .bind(status)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;

        Ok((
            u32::try_from(completed_assets)?,
            u32::try_from(total_assets)?,
            status.to_string(),
        ))
    }

    pub async fn upload_history(&self) -> anyhow::Result<Vec<UploadHistoryRun>> {
        let rows = sqlx::query(
            r#"
            SELECT id,
                   source,
                   collection_name,
                   status,
                   total_assets,
                   completed_assets,
                   total_bytes,
                   started_at,
                   finished_at
            FROM ingest_runs
            ORDER BY started_at DESC
            LIMIT 20
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| UploadHistoryRun {
                ingest_id: row.get::<Uuid, _>("id").to_string(),
                collection_name: row.get("collection_name"),
                source: row.get("source"),
                status: row.get("status"),
                total_assets: row.get("total_assets"),
                completed_assets: row.get("completed_assets"),
                total_bytes: row.get("total_bytes"),
                started_at: row
                    .get::<DateTime<Utc>, _>("started_at")
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                finished_at: row
                    .get::<Option<DateTime<Utc>>, _>("finished_at")
                    .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            })
            .collect())
    }
}
