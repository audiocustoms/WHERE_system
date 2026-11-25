//! `where-installer` — registers a WHERE product in an installation database.
//!
//! The installer implements the rule of the family: the database is the shared
//! unit. A module installed first creates the database and owns it; the core
//! installed later joins that database instead of creating a second one.
//!
//! ```text
//! where-installer --status
//! where-installer --module go
//! where-installer --remove people
//! ```
//!
//! The connection string comes from the same settings as the API
//! (`--config`, `WHERE_CONFIG`, the per-user file or `DATABASE_URL`).

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use where_modules::{ModuleId, install, remove, snapshot};
use where_persistence::{PgPool, connect, migrate};
use where_server::Settings;

#[tokio::main]
async fn main() -> Result<()> {
    let action = Action::parse()?;
    let settings = match &action.config {
        Some(path) => Settings::load_from(path)?,
        None => Settings::from_environment(),
    };
    let database_url = settings
        .resolved_database_url()
        .context("no database configured — set DATABASE_URL or --config")?;

    let pool = connect(&database_url, settings.max_connections)
        .await
        .context("connecting to PostgreSQL")?;
    if settings.run_migrations {
        migrate(&pool).await.context("applying migrations")?;
    }
    where_modules::ensure_registry(&pool)
        .await
        .context("creating the module registry")?;

    match action.kind {
        Kind::Status => status(&pool).await,
        Kind::Install(id) => {
            let outcome = install(&pool, id, Some("where-installer")).await?;
            let manifest = id.manifest();
            println!("{} ({})", manifest.product_name, id.key());
            if outcome.newly_installed {
                println!("  installiert.");
            } else {
                println!("  war bereits installiert.");
            }
            if outcome.shares_core_database {
                println!("  nutzt die Datenbank und Mandanten der Kernanwendung.");
            } else if outcome.owns_database {
                println!("  eigene Datenbank angelegt; die Kernanwendung übernimmt sie später.");
            } else if outcome.reused_existing_database {
                println!(
                    "  nutzt die vorhandene Datenbank (Eigentümer: {}).",
                    outcome.database_owner.as_deref().unwrap_or("?")
                );
            }
            Ok(())
        }
        Kind::Remove(id) => {
            remove(&pool, id).await?;
            println!("{} entfernt. Die Daten bleiben in der Datenbank.", id.key());
            Ok(())
        }
    }
}

async fn status(pool: &PgPool) -> Result<()> {
    let snapshot = snapshot(pool).await?;
    println!(
        "Installation {} — Datenbank angelegt von: {}",
        snapshot.installation_id.as_deref().unwrap_or("?"),
        snapshot.database_owner.as_deref().unwrap_or("?")
    );
    for module in &snapshot.modules {
        println!(
            "  [{}{}] {} ({})",
            if module.installed { "x" } else { " " },
            if module.is_core { "K" } else { "M" },
            module.product_name,
            module.id
        );
    }
    Ok(())
}

struct Action {
    config: Option<PathBuf>,
    kind: Kind,
}

enum Kind {
    Status,
    Install(ModuleId),
    Remove(ModuleId),
}

impl Action {
    fn parse() -> Result<Self> {
        let mut config = None;
        let mut kind = None;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--config" => {
                    config = Some(PathBuf::from(args.next().context("--config needs a path")?));
                }
                "--status" => kind = Some(Kind::Status),
                "--module" => {
                    let key = args.next().context("--module needs an id")?;
                    kind = Some(Kind::Install(module(&key)?));
                }
                "--remove" => {
                    let key = args.next().context("--remove needs an id")?;
                    kind = Some(Kind::Remove(module(&key)?));
                }
                "--help" | "-h" => {
                    println!(
                        "where-installer --status | --module <go|people|money|stuff> | \
                         --remove <go|people|money> [--config <path>]"
                    );
                    std::process::exit(0);
                }
                other if other.starts_with("--config=") => {
                    config = Some(PathBuf::from(&other["--config=".len()..]));
                }
                other => bail!("unknown argument '{other}' (try --help)"),
            }
        }
        let kind = kind
            .context("nothing to do — use --status, --module <id> or --remove <id> (try --help)")?;
        Ok(Self { config, kind })
    }
}

fn module(key: &str) -> Result<ModuleId> {
    ModuleId::from_key(key)
        .with_context(|| format!("unknown module '{key}' — known: stuff, go, people, money"))
}
