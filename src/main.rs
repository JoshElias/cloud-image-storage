mod app;
mod auth;
mod cli;
mod config;
mod db;
mod domain;
mod ingest;
mod storage;
mod ui;
mod upload_api;
mod worker;

use anyhow::Context;
use clap::Parser;
use cli::{AuthCommand, Cli, Command, ImportCommand};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Serve { config } => {
            let config = config::AppConfig::load(config.as_deref()).await?;
            app::serve(config).await
        }
        Command::Auth { command } => match command {
            AuthCommand::HashPassword { password } => {
                println!("{}", auth::hash_password(&password)?);
                Ok(())
            }
        },
        Command::Worker { config } => {
            let config = config::AppConfig::load(config.as_deref()).await?;
            worker::run(config).await
        }
        Command::Import { command } => match command {
            ImportCommand::Takeout {
                path,
                collection,
                server,
                json,
            } => {
                let manifest = ingest::scan_takeout(&path, collection, server)
                    .with_context(|| format!("scan Google Takeout export at {path}"))?;
                cli::print_manifest(&manifest, json)
            }
        },
        Command::Upload {
            path,
            collection,
            server,
            json,
        } => {
            let manifest = ingest::scan_manual_upload(&path, collection, server)
                .with_context(|| format!("scan manual upload path at {path}"))?;
            cli::print_manifest(&manifest, json)
        }
    }
}
