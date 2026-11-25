//! Module manifests and the installation registry.
//!
//! WHERE is one database for four products:
//!
//! * `WHERE - is my stuff?` — the core warehouse logistics application.
//! * `WHERE - does my stuff go?` — orders and package slips.
//! * `WHERE - are my people?` — employees and personnel planning.
//! * `WHERE - is my money?` — accounting and invoicing.
//!
//! Every product stays usable on its own. When a module is installed first it
//! creates the database and is recorded as its owner; a core installation that
//! follows reuses that database instead of creating another one. The registry
//! tables live in the same database as the data, so "same database" and "same
//! tenants" are the same statement.
//!
//! The module code itself ships as separate crates (`where-module-go`,
//! `where-module-money`, `where-module-people`); this crate only carries the
//! shared vocabulary and the registry.

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use thiserror::Error;
use utoipa::ToSchema;

/// Registry key of the core application.
pub const CORE_MODULE_ID: &str = "stuff";

/// Advisory lock that serialises concurrent installs of two products.
const INSTALL_LOCK: i64 = 0x5748_4552_45; // "WHERE"

const REGISTRY_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS "where_installation" (
    "key"             boolean PRIMARY KEY DEFAULT true CHECK ("key"),
    "installation_id" uuid NOT NULL DEFAULT gen_random_uuid(),
    "created_at"      timestamptz NOT NULL DEFAULT now(),
    "database_owner"  text
);

CREATE TABLE IF NOT EXISTS "where_modules" (
    "id"           text PRIMARY KEY,
    "product_name" text NOT NULL,
    "kind"         text NOT NULL CHECK ("kind" IN ('core', 'module')),
    "installed_at" timestamptz NOT NULL DEFAULT now(),
    "installed_by" text
);
"#;

/// The four products of the WHERE family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleId {
    /// `WHERE - is my stuff?` — core warehouse logistics.
    Stuff,
    /// `WHERE - does my stuff go?` — orders and package slips.
    Go,
    /// `WHERE - are my people?` — personnel.
    People,
    /// `WHERE - is my money?` — accounting.
    Money,
}

impl ModuleId {
    pub const ALL: [ModuleId; 4] = [
        ModuleId::Stuff,
        ModuleId::Go,
        ModuleId::People,
        ModuleId::Money,
    ];

    /// Stable registry key; also the value at the end of `/api/v1/modules/{id}`.
    pub fn key(self) -> &'static str {
        match self {
            ModuleId::Stuff => CORE_MODULE_ID,
            ModuleId::Go => "go",
            ModuleId::People => "people",
            ModuleId::Money => "money",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|id| id.key() == key)
    }

    pub fn manifest(self) -> &'static ModuleManifest {
        match self {
            ModuleId::Stuff => &STUFF,
            ModuleId::Go => &GO,
            ModuleId::People => &PEOPLE,
            ModuleId::Money => &MONEY,
        }
    }
}

/// What a module adds to the shared database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    /// Rental and sale prices on the article form.
    ArticlePricing,
    /// Orders and package slips.
    Orders,
    /// Employees and personnel planning.
    People,
    /// Accounting and invoicing.
    Accounting,
}

impl Capability {
    pub fn key(self) -> &'static str {
        match self {
            Capability::ArticlePricing => "article-pricing",
            Capability::Orders => "orders",
            Capability::People => "people",
            Capability::Accounting => "accounting",
        }
    }
}

/// Static description of one product.
#[derive(Debug, Clone)]
pub struct ModuleManifest {
    pub id: ModuleId,
    /// Product name as it appears in the installer and the title bar.
    pub product_name: &'static str,
    pub description: &'static str,
    pub is_core: bool,
    pub capabilities: &'static [Capability],
}

pub static STUFF: ModuleManifest = ModuleManifest {
    id: ModuleId::Stuff,
    product_name: "WHERE - is my stuff?",
    description: "Wareneingang, Artikel, Adressen, Lager und Bestand.",
    is_core: true,
    capabilities: &[],
};

pub static GO: ModuleManifest = ModuleManifest {
    id: ModuleId::Go,
    product_name: "WHERE - does my stuff go?",
    description: "Aufträge, Packscheine und Miet-/Verkaufspreise an Artikeln.",
    is_core: false,
    capabilities: &[Capability::Orders, Capability::ArticlePricing],
};

pub static PEOPLE: ModuleManifest = ModuleManifest {
    id: ModuleId::People,
    product_name: "WHERE - are my people?",
    description: "Personalwesen und Personaleinsatz.",
    is_core: false,
    capabilities: &[Capability::People],
};

pub static MONEY: ModuleManifest = ModuleManifest {
    id: ModuleId::Money,
    product_name: "WHERE - is my money?",
    description: "Buchhaltung, Bankverbindungen und Rechnungswesen.",
    is_core: false,
    capabilities: &[Capability::Accounting],
};

/// Everything the header and the module admin page need to know.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ModuleSnapshot {
    /// Stable id of the installation; `None` before the registry exists.
    pub installation_id: Option<String>,
    /// Product that created the database (`stuff`, `go`, …).
    pub database_owner: Option<String>,
    /// The core application is installed in this database.
    pub core_installed: bool,
    /// One entry per known product.
    pub modules: Vec<ModuleStatus>,
    /// Registry rows of products this build does not know.
    #[serde(default)]
    pub unknown_modules: Vec<String>,
}

impl ModuleSnapshot {
    pub fn installed(&self, id: ModuleId) -> bool {
        self.modules
            .iter()
            .any(|module| module.id == id.key() && module.installed)
    }

    /// Name of the product this installation runs.
    ///
    /// The core application names the installation; without it the first installed
    /// module does. `None` while the registry holds no installed product.
    pub fn product_name(&self) -> Option<&'static str> {
        ModuleId::ALL
            .into_iter()
            .find(|id| self.installed(*id))
            .map(|id| id.manifest().product_name)
    }

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.modules.iter().any(|module| {
            module.installed
                && module
                    .capabilities
                    .iter()
                    .any(|candidate| *candidate == capability.key())
        })
    }
}

/// Availability of one product in this installation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModuleStatus {
    pub id: String,
    pub product_name: String,
    pub description: String,
    /// `core` or `module`.
    pub kind: String,
    pub is_core: bool,
    pub installed: bool,
    /// Capability keys the module contributes while installed.
    pub capabilities: Vec<String>,
}

impl ModuleStatus {
    fn from_manifest(manifest: &ModuleManifest, installed: bool) -> Self {
        Self {
            id: manifest.id.key().to_string(),
            product_name: manifest.product_name.to_string(),
            description: manifest.description.to_string(),
            kind: if manifest.is_core { "core" } else { "module" }.to_string(),
            is_core: manifest.is_core,
            installed,
            capabilities: manifest
                .capabilities
                .iter()
                .map(|capability| capability.key().to_string())
                .collect(),
        }
    }
}

/// What happened when a product was installed.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstallOutcome {
    pub module: String,
    /// `false` when the product was already registered.
    pub newly_installed: bool,
    /// The product uses the database of the core installation.
    pub shares_core_database: bool,
    /// This install created or adopted the database.
    pub owns_database: bool,
    /// The database already belonged to another product before this install.
    pub reused_existing_database: bool,
    pub installation_id: Option<String>,
    pub database_owner: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    pub already_installed: bool,
    pub shares_core_database: bool,
    pub owns_database: bool,
    pub reused_existing_database: bool,
}

/// Decides database ownership for an install without touching the database.
///
/// `installed` is the registry before the install. The rule is deliberately
/// one-way: the first product creates the database, everything afterwards
/// shares it. Installing the core later never creates a second database — it
/// joins the one a module created.
pub fn resolve_install(installed: &[String], installing: ModuleId) -> InstallPlan {
    let core_present = installed.iter().any(|id| id.as_str() == CORE_MODULE_ID);
    let this_present = installed.iter().any(|id| id.as_str() == installing.key());
    let others_present = installed
        .iter()
        .any(|id| id.as_str() != installing.key() && id.as_str() != CORE_MODULE_ID);

    if installing == ModuleId::Stuff {
        InstallPlan {
            already_installed: this_present,
            shares_core_database: true,
            owns_database: !this_present && installed.is_empty(),
            reused_existing_database: !this_present && others_present,
        }
    } else {
        InstallPlan {
            already_installed: this_present,
            shares_core_database: core_present,
            owns_database: !this_present && !core_present && installed.is_empty(),
            reused_existing_database: !this_present && !core_present && others_present,
        }
    }
}

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("unknown module '{0}'")]
    UnknownModule(String),
    #[error("only modules can be installed; '{0}' is the core application")]
    NotAModule(String),
    #[error("the core application must not be removed")]
    CoreCannotBeRemoved,
    #[error("module '{0}' is not installed")]
    NotInstalled(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Creates the registry tables when the migration did not run yet.
pub async fn ensure_registry(pool: &PgPool) -> Result<(), ModuleError> {
    sqlx::raw_sql(REGISTRY_SQL).execute(pool).await?;
    Ok(())
}

/// Registers a product in the installation's database.
///
/// The database is the unit of sharing: whoever installs first creates it and
/// every later product reuses it. Tenants are part of that database and
/// therefore shared as well.
pub async fn install(
    pool: &PgPool,
    id: ModuleId,
    installed_by: Option<&str>,
) -> Result<InstallOutcome, ModuleError> {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(REGISTRY_SQL).execute(&mut *tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(INSTALL_LOCK)
        .execute(&mut *tx)
        .await?;

    let existing: Vec<String> = sqlx::query_scalar("SELECT \"id\" FROM \"where_modules\"")
        .fetch_all(&mut *tx)
        .await?;
    let plan = resolve_install(&existing, id);

    if !plan.already_installed {
        let manifest = id.manifest();
        sqlx::query(
            "INSERT INTO \"where_modules\" (\"id\", \"product_name\", \"kind\", \"installed_by\") \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id.key())
        .bind(manifest.product_name)
        .bind(if manifest.is_core { "core" } else { "module" })
        .bind(installed_by)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "INSERT INTO \"where_installation\" (\"key\") VALUES (true) \
         ON CONFLICT (\"key\") DO NOTHING",
    )
    .execute(&mut *tx)
    .await?;

    if plan.owns_database {
        sqlx::query(
            "UPDATE \"where_installation\" SET \"database_owner\" = $1 \
             WHERE \"database_owner\" IS NULL",
        )
        .bind(id.key())
        .execute(&mut *tx)
        .await?;
    }

    let row = sqlx::query(
        "SELECT \"installation_id\"::text AS installation_id, \"database_owner\" \
         FROM \"where_installation\" WHERE \"key\" = true",
    )
    .fetch_one(&mut *tx)
    .await?;
    let installation_id: String = row.get("installation_id");
    let database_owner: Option<String> = row.get("database_owner");

    tx.commit().await?;

    Ok(InstallOutcome {
        module: id.key().to_string(),
        newly_installed: !plan.already_installed,
        shares_core_database: plan.shares_core_database,
        owns_database: plan.owns_database,
        reused_existing_database: plan.reused_existing_database,
        installation_id: Some(installation_id),
        database_owner,
    })
}

/// Removes a module. The core application cannot be removed.
pub async fn remove(pool: &PgPool, id: ModuleId) -> Result<(), ModuleError> {
    if id == ModuleId::Stuff {
        return Err(ModuleError::CoreCannotBeRemoved);
    }
    let affected = sqlx::query("DELETE FROM \"where_modules\" WHERE \"id\" = $1")
        .bind(id.key())
        .execute(pool)
        .await?
        .rows_affected();
    if affected == 0 {
        return Err(ModuleError::NotInstalled(id.key().to_string()));
    }
    Ok(())
}

/// Reads the registry of the database.
pub async fn snapshot(pool: &PgPool) -> Result<ModuleSnapshot, ModuleError> {
    sqlx::raw_sql(REGISTRY_SQL).execute(pool).await?;

    let installation = sqlx::query(
        "SELECT \"installation_id\"::text AS installation_id, \"database_owner\" \
         FROM \"where_installation\" WHERE \"key\" = true",
    )
    .fetch_optional(pool)
    .await?;

    let installed: Vec<String> = sqlx::query_scalar("SELECT \"id\" FROM \"where_modules\"")
        .fetch_all(pool)
        .await?;

    let core_installed = installed.iter().any(|id| id.as_str() == CORE_MODULE_ID);
    let modules = ModuleId::ALL
        .into_iter()
        .map(|id| {
            ModuleStatus::from_manifest(
                id.manifest(),
                installed.iter().any(|x| x.as_str() == id.key()),
            )
        })
        .collect();
    let unknown_modules = installed
        .into_iter()
        .filter(|id| ModuleId::from_key(id).is_none())
        .collect();

    Ok(ModuleSnapshot {
        installation_id: installation.as_ref().map(|row| row.get("installation_id")),
        database_owner: installation.and_then(|row| row.get("database_owner")),
        core_installed,
        modules,
        unknown_modules,
    })
}

/// Cached registry used by the request handlers.
///
/// The cache is filled at start-up and after an install or removal, so the
/// module guard does not query the database on every request.
#[derive(Debug, Clone, Default)]
pub struct ModuleState(Arc<RwLock<ModuleSnapshot>>);

impl ModuleState {
    pub fn new(snapshot: ModuleSnapshot) -> Self {
        Self(Arc::new(RwLock::new(snapshot)))
    }

    pub fn snapshot(&self) -> ModuleSnapshot {
        self.0.read().map(|guard| guard.clone()).unwrap_or_default()
    }

    pub fn replace(&self, snapshot: ModuleSnapshot) {
        if let Ok(mut guard) = self.0.write() {
            *guard = snapshot;
        }
    }

    pub fn core_installed(&self) -> bool {
        self.0
            .read()
            .map(|guard| guard.core_installed)
            .unwrap_or(false)
    }

    pub fn is_installed(&self, id: ModuleId) -> bool {
        self.0
            .read()
            .map(|guard| guard.installed(id))
            .unwrap_or(false)
    }

    pub fn has_capability(&self, capability: Capability) -> bool {
        self.0
            .read()
            .map(|guard| guard.has_capability(capability))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn a_module_installed_first_owns_the_database() {
        let plan = resolve_install(&[], ModuleId::Go);
        assert!(plan.owns_database);
        assert!(!plan.shares_core_database);
        assert!(!plan.reused_existing_database);
    }

    #[test]
    fn a_module_joins_an_existing_core_database() {
        let plan = resolve_install(&ids(&["stuff"]), ModuleId::Money);
        assert!(plan.shares_core_database);
        assert!(!plan.owns_database);
        assert!(!plan.reused_existing_database);
    }

    #[test]
    fn the_core_reuses_a_database_created_by_a_module() {
        let plan = resolve_install(&ids(&["go"]), ModuleId::Stuff);
        assert!(plan.shares_core_database);
        assert!(!plan.owns_database);
        assert!(plan.reused_existing_database);
    }

    #[test]
    fn the_first_product_owns_the_database() {
        let plan = resolve_install(&[], ModuleId::Stuff);
        assert!(plan.owns_database);
        assert!(!plan.reused_existing_database);
    }

    #[test]
    fn installing_twice_is_idempotent() {
        let plan = resolve_install(&ids(&["stuff", "people"]), ModuleId::People);
        assert!(plan.already_installed);
        assert!(plan.shares_core_database);
        assert!(!plan.owns_database);
    }

    #[test]
    fn the_manifest_names_the_four_products() {
        let names: Vec<&str> = ModuleId::ALL
            .into_iter()
            .map(|id| id.manifest().product_name)
            .collect();
        assert_eq!(
            names,
            [
                "WHERE - is my stuff?",
                "WHERE - does my stuff go?",
                "WHERE - are my people?",
                "WHERE - is my money?"
            ]
        );
        assert!(
            ModuleId::Go
                .manifest()
                .capabilities
                .contains(&Capability::ArticlePricing)
        );
    }

    #[test]
    fn capabilities_follow_the_installed_modules() {
        let snapshot = ModuleSnapshot {
            modules: ModuleId::ALL
                .into_iter()
                .map(|id| ModuleStatus::from_manifest(id.manifest(), id == ModuleId::Go))
                .collect(),
            ..Default::default()
        };
        assert!(snapshot.has_capability(Capability::ArticlePricing));
        assert!(!snapshot.has_capability(Capability::Accounting));
    }

    #[test]
    fn the_installed_product_names_the_installation() {
        let mit_kern = ModuleSnapshot {
            modules: ModuleId::ALL
                .into_iter()
                .map(|id| {
                    ModuleStatus::from_manifest(
                        id.manifest(),
                        id == ModuleId::Stuff || id == ModuleId::People,
                    )
                })
                .collect(),
            ..Default::default()
        };
        assert_eq!(mit_kern.product_name(), Some("WHERE - is my stuff?"));

        let nur_personal = ModuleSnapshot {
            modules: ModuleId::ALL
                .into_iter()
                .map(|id| ModuleStatus::from_manifest(id.manifest(), id == ModuleId::People))
                .collect(),
            ..Default::default()
        };
        assert_eq!(nur_personal.product_name(), Some("WHERE - are my people?"));

        assert_eq!(ModuleSnapshot::default().product_name(), None);
    }
}
