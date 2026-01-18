//! Reference HTTP cache server for Polykit.

mod config;
mod server;
mod storage;
mod verification;

use anyhow::Result;
use clap::Parser;
use tokio::signal;
use tracing::{info, Level};

use config::ServerConfig;
use server::{create_router, AppState};
use storage::Storage;
use verification::Verifier;

#[derive(Parser)]
#[command(name = "polykit-cache")]
#[command(about = "Reference HTTP cache server for Polykit")]
struct Cli {
    /// Storage directory for artifacts
    #[arg(long, default_value = "./cache")]
    storage_dir: String,

    /// Maximum artifact size in bytes
    #[arg(long, default_value = "1073741824")] // 1GB
    max_size: u64,

    /// Bind address
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,

    /// Port number
    #[arg(long, default_value = "8080")]
    port: u16,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    let log_level = match cli.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .init();

    // Create configuration
    let config = ServerConfig::new()
        .with_storage_dir(&cli.storage_dir)
        .with_max_artifact_size(cli.max_size)
        .with_bind_address(&cli.bind)
        .with_port(cli.port);

    info!("Starting polykit-cache server");
    info!("Storage directory: {}", config.storage_dir.display());
    info!("Max artifact size: {} bytes", config.max_artifact_size);
    info!("Listening on {}", config.bind_addr());

    // Create storage and verifier
    let storage = Storage::new(&config.storage_dir, config.max_artifact_size)?;
    let verifier = Verifier::new(config.max_artifact_size);

    // Clean up any stale temp files
    storage.cleanup_temp_files()?;

    // Create app state
    let state = AppState::new(storage, verifier);

    // Create router
    let app = create_router(state);

    // Create server
    let listener = tokio::net::TcpListener::bind(&config.bind_addr()).await?;

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
