//! WHERE API server as a library.
//!
//! [`run`] is the classic binary entry point (signal handling included);
//! [`serve_until`] lets the desktop shell own the lifecycle, which is what the
//! single-workstation installation does with its embedded database.

pub mod config;

use std::future::Future;
use std::net::SocketAddr;

use anyhow::{Context, Result};
use where_api::{AppState, TenantPools, router_with_modules};
use where_module_go::GoModule;
use where_module_money::MoneyModule;
use where_module_people::PeopleModule;
use where_modules::ModuleState;
use where_persistence::{connect, execute_script, migrate};

pub use config::{Embedded, Mode, Product, Settings};

/// Starts the API with the given settings and serves until the process is
/// asked to stop.
///
/// The database connection string comes from [`Settings::resolved_database_url`]:
/// explicit `DATABASE_URL` first, otherwise the embedded instance.
pub async fn run(settings: Settings) -> Result<()> {
    serve_until(settings, shutdown_signal()).await
}

/// Starts the API and serves until `shutdown` resolves.
///
/// The returned future resolves after the last connection was served, so a
/// caller can stop a database it started itself right afterwards.
pub async fn serve_until<F>(settings: Settings, shutdown: F) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let database_url = settings.resolved_database_url().with_context(|| {
        let path = Settings::config_path();
        format!(
            "no database configured — set DATABASE_URL or database_url in {}",
            path.display()
        )
    })?;

    let pool = connect(&database_url, settings.max_connections)
        .await
        .context("connecting to PostgreSQL")?;

    if settings.run_migrations {
        migrate(&pool).await.context("applying migrations")?;
        tracing::info!("migrations applied");
    }

    // The module registry lives in the same database as the data. The product this
    // installation is registers itself here: the core application in a warehouse
    // installation, the personnel module in `WHERE - are my people?`. When the
    // database already exists because another product was installed first, it adopts
    // that database and its tenants instead of creating a second one.
    where_modules::ensure_registry(&pool)
        .await
        .context("creating the module registry")?;
    let product = settings.product;
    let core_install = where_modules::install(&pool, product.module(), Some("system"))
        .await
        .with_context(|| format!("registering {}", product.label()))?;
    if core_install.reused_existing_database {
        tracing::info!(
            owner = core_install.database_owner.as_deref().unwrap_or("?"),
            product = product.key(),
            "the product joins the database created by another one"
        );
    }
    let module_state = ModuleState::new(
        where_modules::snapshot(&pool)
            .await
            .context("reading the module registry")?,
    );

    // Initial installation: standalone/demo and server get their administrator
    // account here, with the initial password that must be changed on first sign-in.
    if where_persistence::ensure_initial_admin(&pool)
        .await
        .context("creating the initial administrator")?
    {
        tracing::info!(
            user = where_persistence::INITIAL_ADMIN_NAME,
            group = where_persistence::INITIAL_ADMIN_GROUP,
            "initial administrator created — the password has to be changed on the first login"
        );
    }

    // The tenant such an installation starts in: the login mask picks one before it
    // lists accounts, and every working page needs one.
    if where_persistence::ensure_initial_tenant(&pool)
        .await
        .context("creating the initial tenant")?
    {
        tracing::info!(
            tenant = where_persistence::INITIAL_TENANT_NAME,
            "initial tenant created"
        );
    }

    let state = AppState::with_file_root(pool, settings.file_storage_dir.clone())
        .with_modules(module_state);
    tracing::info!(files = %settings.file_storage_dir.display(), "file storage");

    // Tenants with a database of their own. A tenant without an entry works on the
    // database of the installation (see README, section on tenants).
    let tenant_pools = open_tenant_pools(&settings).await;
    let state = if tenant_pools.is_empty() {
        state
    } else {
        tracing::info!(tenants = tenant_pools.len(), "tenant databases");
        state.with_tenant_pools(tenant_pools)
    };

    let plugins: [&dyn where_api::ModulePlugin; 3] = [&GoModule, &PeopleModule, &MoneyModule];
    let app = router_with_modules(state, settings.cors_allowed_origins.as_deref(), &plugins);
    let bind: SocketAddr = settings.socket_addr()?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .with_context(|| format!("binding {bind}"))?;
    tracing::info!(%bind, "WHERE API listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .context("serving HTTP")?;
    Ok(())
}

/// Opens the database of every tenant that has one of its own.
///
/// A tenant whose connection fails is reported and left out, so the installation
/// keeps serving the others instead of refusing to start.
async fn open_tenant_pools(settings: &Settings) -> TenantPools {
    let mut pools = std::collections::HashMap::new();
    for (mandant, url) in settings.resolved_tenant_databases() {
        match connect(&url, settings.max_connections).await {
            Ok(pool) => {
                pools.insert(mandant, pool);
            }
            Err(error) => {
                tracing::error!(mandant, %error, "tenant database unavailable");
            }
        }
    }
    TenantPools::new(pools)
}

/// Loads a SQL script into a freshly created database.
///
/// The single-workstation installation uses this to bring the demo dataset in:
/// the script is a `pg_dump` of a seeded database, so it carries the schema and
/// the `_sqlx_migrations` bookkeeping; the migration step that follows sees
/// every version applied and does nothing.
pub async fn apply_script(database_url: &str, script: String) -> Result<()> {
    let pool = connect(database_url, 1)
        .await
        .context("connecting to the empty database")?;
    let outcome = execute_script(&pool, script).await;
    pool.close().await;
    outcome.context("loading the SQL script")
}

/// Resolves on Ctrl+C (and on SIGTERM where the platform has it).
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("installing Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("installing SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("shutdown: interrupt"),
        _ = terminate => tracing::info!("shutdown: terminate"),
    }
}
