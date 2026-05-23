use camino::Utf8Path;
use serde::Deserialize;
use tokio::fs;

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub database: DatabaseConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub bind_address: String,
    pub public_base_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StorageConfig {
    pub bucket: String,
    pub region: String,
    pub originals_prefix: String,
    pub previews_prefix: String,
    pub metadata_prefix: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

impl AppConfig {
    pub async fn load(path: Option<&Utf8Path>) -> anyhow::Result<Self> {
        let Some(path) = path else {
            return Ok(Self::development());
        };

        let content = fs::read_to_string(path).await?;
        Ok(toml::from_str(&content)?)
    }

    pub fn development() -> Self {
        Self {
            server: ServerConfig {
                bind_address: "127.0.0.1:8080".to_string(),
                public_base_url: "http://127.0.0.1:8080".to_string(),
            },
            storage: StorageConfig {
                bucket: "local-placeholder".to_string(),
                region: "us-east-1".to_string(),
                originals_prefix: "originals/".to_string(),
                previews_prefix: "previews/".to_string(),
                metadata_prefix: "metadata/raw/".to_string(),
            },
            database: DatabaseConfig {
                url: "postgres://cis:change-me@127.0.0.1:5432/cis".to_string(),
            },
        }
    }
}
