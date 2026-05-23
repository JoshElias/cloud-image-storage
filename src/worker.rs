use std::time::Duration;

use crate::config::AppConfig;

pub async fn run(config: AppConfig) -> anyhow::Result<()> {
    tracing::info!(
        bucket = config.storage.bucket,
        database_configured = !config.database.url.is_empty(),
        "worker started; restore and backup scheduling hooks are ready"
    );

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("worker shutdown requested");
        }
        _ = tokio::time::sleep(Duration::from_secs(1)) => {
            tracing::info!("worker smoke cycle complete");
        }
    }

    Ok(())
}
