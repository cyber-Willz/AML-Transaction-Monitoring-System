mod app;
mod auth;
mod error;
mod import;
mod ingest;
mod routes;

use anyhow::{Context, Result};
use aml_migration::{Migrator, MigratorTrait};
use clap::{Parser, Subcommand};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Parser)]
#[command(name = "aml", about = "Anti-money-laundering transaction monitoring service")]
struct Cli {
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Apply schema migrations (use --fresh to drop everything first).
    Migrate {
        #[arg(long)]
        fresh: bool,
    },
    /// Run the HTTP API.
    Serve {
        #[arg(long, env = "AML_BIND", default_value = "0.0.0.0:8080")]
        bind: String,
        /// Apply pending migrations on start.
        #[arg(long)]
        migrate: bool,
        /// How much history (relative to the newest transaction) to load into the rolling windows.
        #[arg(long, env = "AML_HYDRATE_DAYS", default_value_t = 180)]
        hydrate_days: i64,
    },
    /// Bulk-load a supported dataset through the scoring pipeline.
    #[command(alias = "import-saml")]
    ImportDataset {
        #[arg(long)]
        file: PathBuf,
        /// saml-d | aml-suspicious | risk-prediction | fraudshield | kyc-risk | lending
        #[arg(long, default_value = "saml-d")]
        format: String,
        #[arg(long, default_value_t = 0)]
        skip: usize,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, default_value_t = 5000)]
        batch: usize,
    },
}

async fn connect(url: &str) -> Result<DatabaseConnection> {
    let mut o = ConnectOptions::new(url.to_string());
    o.max_connections(10).min_connections(1).connect_timeout(Duration::from_secs(10)).sqlx_logging(false);
    Database::connect(o).await.context("connecting to PostgreSQL")
}

async fn state(db: DatabaseConnection, hydrate_days: i64) -> Result<app::AppState> {
    let engine = ingest::load_engine(&db, hydrate_days).await.map_err(|e| anyhow::anyhow!("{e}"))?;
    let keys = auth::Keys::parse(&std::env::var("AML_API_KEYS").unwrap_or_default())?;
    if keys.is_empty() {
        tracing::warn!("AML_API_KEYS is not set: running in DEVELOPMENT mode, every caller is an admin");
    }
    Ok(app::AppState { db, engine: Arc::new(Mutex::new(engine)), keys: Arc::new(keys), hydrate_days })
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=info".into()))
        .init();
    let cli = Cli::parse();
    let db = connect(&cli.database_url).await?;
    match cli.cmd {
        Cmd::Migrate { fresh } => {
            if fresh {
                Migrator::fresh(&db).await?;
            } else {
                Migrator::up(&db, None).await?;
            }
            tracing::info!("migrations applied");
        }
        Cmd::Serve { bind, migrate, hydrate_days } => {
            if migrate {
                Migrator::up(&db, None).await?;
            }
            let st = state(db, hydrate_days).await?;
            let listener = tokio::net::TcpListener::bind(&bind).await.with_context(|| format!("binding {bind}"))?;
            tracing::info!(%bind, "aml api listening");
            axum::serve(listener, app::router(st))
                .with_graceful_shutdown(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
        }
        Cmd::ImportDataset { file, format, skip, limit, batch } => {
            let fmt = aml_datasets::Format::parse(&format)?;
            let st = state(db, 180).await?;
            import::import_dataset(&st, &file, fmt, skip, limit, batch.clamp(100, ingest::MAX_BATCH)).await?;
        }
    }
    Ok(())
}
