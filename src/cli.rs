use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};

use crate::domain::IngestManifest;

#[derive(Debug, Parser)]
#[command(name = "cis", about = "Private image archive")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Serve {
        #[arg(long, env = "CIS_CONFIG")]
        config: Option<Utf8PathBuf>,
    },
    Auth {
        #[command(subcommand)]
        command: AuthCommand,
    },
    Worker {
        #[arg(long, env = "CIS_CONFIG")]
        config: Option<Utf8PathBuf>,
    },
    Import {
        #[command(subcommand)]
        command: ImportCommand,
    },
    Upload {
        #[arg(long)]
        path: Utf8PathBuf,

        #[arg(long)]
        collection: String,

        #[arg(long, env = "CIS_SERVER")]
        server: Option<String>,

        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum AuthCommand {
    HashPassword {
        #[arg(long)]
        password: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ImportCommand {
    Takeout {
        #[arg(long)]
        path: Utf8PathBuf,

        #[arg(long)]
        collection: String,

        #[arg(long, env = "CIS_SERVER")]
        server: Option<String>,

        #[arg(long)]
        json: bool,
    },
}

pub fn print_manifest(manifest: &IngestManifest, json: bool) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(manifest)?);
        return Ok(());
    }

    println!("source: {}", manifest.source);
    println!("collection: {}", manifest.collection_name);
    println!(
        "server: {}",
        manifest.server.as_deref().unwrap_or("(not set)")
    );
    println!("assets: {}", manifest.assets.len());
    println!("duplicates skipped: {}", manifest.duplicates_skipped);
    println!("bytes: {}", manifest.total_bytes);

    Ok(())
}
