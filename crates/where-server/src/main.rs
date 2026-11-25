//! Binary entry point of the WHERE API.
//!
//! Configuration comes from the runtime settings (`--config`, `WHERE_CONFIG`,
//! per-user file) and from the environment; see [`where_server::config`].

use std::path::PathBuf;

use anyhow::Result;
use tracing_subscriber::EnvFilter;
use where_server::{Settings, run};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,where_api=debug,where_persistence=debug")),
        )
        .init();

    let settings = match config_argument()? {
        Some(path) => Settings::load_from(&path)?,
        // No file: defaults plus environment, so `.env` and the development
        // workflow behave exactly as before.
        None => Settings::from_environment(),
    };
    run(settings).await
}

/// Reads `--config <path>` / `--config=<path>` without pulling in a CLI crate.
fn config_argument() -> Result<Option<PathBuf>> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if let Some(value) = arg.strip_prefix("--config=") {
            return Ok(Some(PathBuf::from(value)));
        }
        if arg == "--config" {
            let value = args.next().ok_or_else(|| {
                anyhow::anyhow!(
                    "--config needs a path, e.g. --config C:\\ProgramData\\WHERE\\config.toml"
                )
            })?;
            return Ok(Some(PathBuf::from(value)));
        }
    }
    Ok(None)
}
