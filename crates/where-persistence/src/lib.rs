//! Persistence layer: SQLx repositories over the imported legacy schema.
//!
//! Queries use runtime binding (`query_as`) instead of the `query!` macros so the
//! workspace compiles without a live database. Once a development database is
//! available, run `cargo sqlx prepare` and switch to the checked macros.

pub mod address_repo;
pub mod agenda_repo;
pub mod article_group_repo;
pub mod article_repo;
pub mod article_search_repo;
pub mod auth_repo;
pub mod dashboard_repo;
pub mod error;
pub mod file_repo;
pub mod inspection_repo;
pub mod number_series_repo;
pub mod rows;
pub mod settings_repo;
pub mod stock_repo;
pub mod tenant_repo;
pub mod user_repo;
pub mod warehouse_repo;

pub use address_repo::{
    AddressList, AddressRepository, AddressSearch, AddressSort, AddressState, AddressType,
    TextMatch,
};
pub use agenda_repo::AgendaRepository;
pub use article_group_repo::{ArticleGroupEntry, ArticleGroupRepository};
pub use article_repo::ArticleRepository;
pub use article_search_repo::{
    ArticleSearch, ArticleSearchRepository, ArticleType, PyroSearch, SetOperator,
};
pub use auth_repo::AuthRepository;
pub use dashboard_repo::DashboardRepository;
pub use error::db_error;
pub use file_repo::FileRepository;
pub use inspection_repo::{DeviceBinding, InspectionRepository, NewReport};
pub use number_series_repo::{NumberSeriesEntry, NumberSeriesRepository};
pub use settings_repo::SettingsRepository;
pub use sqlx::PgPool;
pub use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
pub use stock_repo::{
    DeviceRow, DeviceUpdate, GoodsReceipt, StockRepository, StockRow, WarehouseRow,
};
pub use tenant_repo::{INITIAL_TENANT_NAME, TenantRepository, ensure_initial_tenant};
pub use user_repo::{
    INITIAL_ADMIN_GROUP, INITIAL_ADMIN_NAME, INITIAL_ADMIN_PASSWORD, MailAccount, UserRepository,
    effective_rights, ensure_initial_admin, known_rights,
};
pub use warehouse_repo::{
    BinRow, SubBinRow, WarehouseAdminRow, WarehouseFlags, WarehouseRepository,
};

/// Opens a pooled connection. `DATABASE_URL` must point at a PostgreSQL
/// instance with the migrations applied.
///
/// `DB_STATEMENT_CACHE` sets the per-connection prepared-statement cache size
/// (default 100).
///
/// `DB_SHARED_BACKEND=1` selects "shared backend" mode for poolers that expose a
/// single engine to many clients (PgBouncer in transaction mode, or the PGlite
/// dev database). It disables the statement cache and pins the pool to one
/// long-lived connection: SQLx numbers prepared statements per connection
/// (`sqlx_s_1`, `sqlx_s_2`, ...), so a second connection against a shared engine
/// fails with "prepared statement already exists".
pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    let shared_backend = std::env::var("DB_SHARED_BACKEND")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let statement_cache: usize = std::env::var("DB_STATEMENT_CACHE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(if shared_backend { 0 } else { 100 });

    let options: PgConnectOptions = database_url
        .parse::<PgConnectOptions>()?
        .statement_cache_capacity(statement_cache);

    let mut builder = PgPoolOptions::new().max_connections(max_connections);
    if shared_backend {
        builder = builder
            .max_connections(1)
            .min_connections(1)
            .max_lifetime(None)
            .idle_timeout(None)
            .test_before_acquire(false)
            // A shared engine outlives the client process, so the statement names a
            // previous run prepared are still registered. Clear them before use,
            // otherwise a restarted server fails with "prepared statement sqlx_s_1
            // already exists".
            .after_connect(|conn, _meta| {
                Box::pin(async move {
                    // `raw_sql` uses the simple query protocol, so it works even while
                    // the name `sqlx_s_1` is still taken by the previous process.
                    sqlx::raw_sql("DEALLOCATE ALL").execute(&mut *conn).await?;
                    Ok(())
                })
            });
    }

    builder.connect_with(options).await
}

/// Applies the embedded migrations. Validated at compile time, so a malformed
/// migration filename fails the build rather than the deployment.
///
/// `sqlx` hashes the raw bytes of a migration file. A checkout that carried the file
/// with CRLF line endings and today's LF therefore looks like an edited migration,
/// and `sqlx` refuses to start even though the SQL is unchanged. That one case is
/// repaired before the run — the recorded checksum is rewritten — because the
/// alternative is a database that can no longer be opened after a line-ending
/// normalisation. Repairing up front also keeps `sqlx`'s advisory lock untouched: a
/// second `run` would wait for the lock its first call left behind.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let migrator = sqlx::migrate!("../../migrations");
    repair_line_endings(pool, &migrator).await?;
    migrator.run(pool).await
}

/// Rewrites recorded checksums that differ only in line endings.
async fn repair_line_endings(
    pool: &PgPool,
    migrator: &sqlx::migrate::Migrator,
) -> Result<(), sqlx::migrate::MigrateError> {
    let exists: bool = sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
        .fetch_one(pool)
        .await
        .map_err(sqlx::migrate::MigrateError::Execute)?;
    if !exists {
        return Ok(());
    }
    let recorded: Vec<(i64, Vec<u8>)> =
        sqlx::query_as("SELECT \"version\", \"checksum\" FROM _sqlx_migrations")
            .fetch_all(pool)
            .await
            .map_err(sqlx::migrate::MigrateError::Execute)?;
    for (version, checksum) in recorded {
        let Some(migration) = migrator.iter().find(|entry| entry.version == version) else {
            continue;
        };
        let Some(current) = line_ending_checksum(&checksum, migration.sql.as_ref()) else {
            continue;
        };
        sqlx::query("UPDATE _sqlx_migrations SET \"checksum\" = $1 WHERE \"version\" = $2")
            .bind(&current)
            .bind(version)
            .execute(pool)
            .await
            .map_err(sqlx::migrate::MigrateError::Execute)?;
        tracing::warn!(
            version,
            description = migration.description.as_ref(),
            "migration differs only in line endings — the recorded checksum was updated"
        );
    }
    Ok(())
}

/// The checksum of `sql` as today's file bytes, if `recorded` is only a line-ending
/// variant of it.
///
/// `sql` is the text as compiled into the binary, i.e. with LF.
pub fn line_ending_checksum(recorded: &[u8], sql: &str) -> Option<Vec<u8>> {
    use sha2::{Digest, Sha384};

    let crlf = sql.replace('\n', "\r\n");
    let candidates = [sql.as_bytes(), crlf.as_bytes()];
    if !candidates
        .iter()
        .any(|candidate| Sha384::digest(candidate).as_slice() == recorded)
    {
        return None;
    }
    Some(Sha384::digest(sql.as_bytes()).to_vec())
}

/// Executes a SQL script with several statements against `pool`.
///
/// Used to load the demo dataset of the single-workstation installation: the
/// script is a plain `pg_dump` of a seeded database, so it carries both the
/// schema and the `_sqlx_migrations` bookkeeping and the regular migration step
/// afterwards is a no-op.
pub async fn execute_script(pool: &PgPool, script: String) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(sqlx::AssertSqlSafe(script))
        .execute(pool)
        .await
        .map(|_| ())
}

/// Round-trips a trivial query to prove the pool can reach the database.
pub async fn ping(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}

/// Builds a pool without opening a connection. Used by tests and by tooling
/// that only needs the pool type (for example OpenAPI generation).
pub fn pool_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(database_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha384};

    #[test]
    fn a_migration_that_differs_only_in_line_endings_is_recognised() {
        let sql = "CREATE TABLE t (a int);\nINSERT INTO t VALUES (1);\n";
        let crlf = Sha384::digest(sql.replace('\n', "\r\n").as_bytes()).to_vec();
        let lf = Sha384::digest(sql.as_bytes()).to_vec();

        assert_eq!(line_ending_checksum(&crlf, sql), Some(lf.clone()));
        assert_eq!(line_ending_checksum(&lf, sql), Some(lf));
        // An edited migration stays an error, not a silent repair.
        let edited = Sha384::digest(b"CREATE TABLE t (a text);\n").to_vec();
        assert_eq!(line_ending_checksum(&edited, sql), None);
    }
}
