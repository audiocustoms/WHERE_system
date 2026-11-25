//! Runtime configuration for the API process.
//!
//! The same settings serve both deployment shapes: a central server that owns a
//! PostgreSQL instance, and a single workstation that runs an embedded database
//! next to the API. Values are resolved in three layers, each winning over the
//! previous one:
//!
//! 1. built-in defaults,
//! 2. a TOML file (`--config <path>`, else `WHERE_CONFIG`, else the per-user
//!    path from [`Settings::config_path`]),
//! 3. environment variables — so the development workflow with `.env` keeps
//!    working unchanged.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Directory name used below `%LOCALAPPDATA%` / `$XDG_DATA_HOME`.
///
/// Deliberately not the bundle identifier of any product: the NSIS uninstaller of an
/// application deletes `%LOCALAPPDATA%\<identifier>` when "delete app data" is ticked,
/// and the database of every product lives here.
pub const APP_DIR: &str = "where-data";

/// Bundle identifier of the core application.
pub const CORE_IDENTIFIER: &str = "de.where.warehouse";

/// Where installations of earlier versions kept their data: the former bundle
/// identifier of the core. Never change it to [`CORE_IDENTIFIER`] — the folder only
/// exists on machines that ran one of those versions.
const LEGACY_APP_DIR: &str = "de.where.lagerlogistik";

/// Which product this installation is.
///
/// The core application always carried the module registry with it; a module that
/// also runs on its own (`WHERE - are my people?`) is installed instead of the core
/// and creates the database itself. The switch names the product, decides which
/// module registers itself at start-up and is what the window titles follow. All
/// products share one data directory and therefore one database.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Product {
    /// `WHERE - is my stuff?` — warehouse logistics, the core application.
    #[default]
    Stuff,
    /// `WHERE - are my people?` — personnel and duty planning, standing alone.
    People,
}

impl Product {
    /// Key of the product in the module registry.
    pub fn key(self) -> &'static str {
        match self {
            Self::Stuff => where_modules::CORE_MODULE_ID,
            Self::People => "people",
        }
    }

    /// Name on the title bar, the login mask and the start menu.
    pub fn label(self) -> &'static str {
        match self {
            Self::Stuff => "WHERE - is my stuff?",
            Self::People => "WHERE - are my people?",
        }
    }

    /// Bundle identifier of the desktop installation; the product is read from it.
    pub fn identifier(self) -> &'static str {
        match self {
            Self::Stuff => CORE_IDENTIFIER,
            Self::People => "de.where.are-my-people",
        }
    }

    /// The product as the installer and the registry know it.
    pub fn module(self) -> where_modules::ModuleId {
        match self {
            Self::Stuff => where_modules::ModuleId::Stuff,
            Self::People => where_modules::ModuleId::People,
        }
    }

    /// Reads a product from its identifier — a Tauri bundle carries it there.
    pub fn from_identifier(identifier: &str) -> Option<Self> {
        [Self::Stuff, Self::People]
            .into_iter()
            .find(|product| product.identifier() == identifier)
    }
}

/// How the desktop application runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Everything on one machine: embedded PostgreSQL plus a loopback API.
    #[default]
    Standalone,
    /// A workstation that talks to an API somewhere in the network.
    Client,
}

/// Settings of the embedded database used in [`Mode::Standalone`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Embedded {
    /// Loopback port of the embedded PostgreSQL. Never exposed to the network.
    pub port: u16,
    /// Database the API connects to.
    pub database: String,
    /// Superuser the engine is initialised with.
    pub username: String,
    /// Superuser password; generated on first start and kept in the config file.
    pub password: String,
    /// Data directory (`initdb` target).
    pub data_dir: PathBuf,
    /// Extracted PostgreSQL binaries.
    pub installation_dir: PathBuf,
}

impl Default for Embedded {
    fn default() -> Self {
        let root = Settings::data_root();
        Self {
            port: 5433,
            database: "where".to_string(),
            // `initdb` creates exactly one superuser; the crate's bootstrap
            // name is `postgres`, so the embedded instance must use it too.
            username: "postgres".to_string(),
            password: String::new(),
            data_dir: root.join("pg"),
            installation_dir: root.join("pg-bin"),
        }
    }
}

impl Embedded {
    /// Connection string the API uses.
    pub fn database_url(&self) -> String {
        format!(
            "postgres://{}:{}@127.0.0.1:{}/{}",
            self.username, self.password, self.port, self.database
        )
    }
}

/// Everything the API and the desktop shell need to start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Which product this installation is: the core application or a module that
    /// stands alone. It decides the module that registers itself at start-up.
    pub product: Product,
    /// Only relevant for the desktop shell; the API server ignores it.
    pub mode: Mode,
    /// API address the workstation talks to in [`Mode::Client`].
    pub api_base: Option<String>,
    /// PostgreSQL connection string. Required unless [`Mode::Standalone`].
    pub database_url: Option<String>,
    /// HTTP address of the API.
    pub bind: String,
    /// Pool size of the API process.
    pub max_connections: u32,
    /// Apply the migrations on start.
    pub run_migrations: bool,
    /// Comma separated CORS allow-list; the built-in list is used when absent.
    pub cors_allowed_origins: Option<String>,
    /// Where uploaded files are stored.
    pub file_storage_dir: PathBuf,
    /// Own database of a tenant, keyed by `"Mandanten"."Primary_Key"`.
    ///
    /// Every tenant has its own database; whoever is not listed here shares the
    /// installation's database.
    pub tenant_databases: BTreeMap<String, String>,
    /// Embedded database of the single workstation installation.
    pub embedded: Embedded,
}

impl Default for Settings {
    fn default() -> Self {
        let root = Settings::data_root();
        Self {
            product: Product::default(),
            mode: Mode::default(),
            api_base: None,
            database_url: None,
            // The single-workstation install must be able to run next to a
            // development API on 8090, so its loopback API uses 8091.
            bind: "127.0.0.1:8091".to_string(),
            max_connections: 10,
            run_migrations: true,
            cors_allowed_origins: None,
            file_storage_dir: root.join("files"),
            tenant_databases: BTreeMap::new(),
            embedded: Embedded::default(),
        }
    }
}

impl Settings {
    /// Directory for data, binaries and the configuration file of this user.
    ///
    /// Windows: `%LOCALAPPDATA%\where-data` — deliberately not the roaming profile,
    /// because a PostgreSQL data directory must never be synchronised while the server
    /// is writing to it. `WHERE_DATA_DIR` wins.
    ///
    /// Every product uses this one directory: the database inside it is shared, so a
    /// product installed later joins the installation instead of creating a second one.
    /// An installation of an earlier version keeps its data in
    /// `%LOCALAPPDATA%\de.where.lagerlogistik`; that directory is moved here once (see
    /// [`Settings::adopt_legacy_root`]), because its name is the bundle identifier of
    /// the core application and its uninstaller deletes exactly that folder.
    pub fn data_root() -> PathBuf {
        if let Some(dir) = std::env::var_os("WHERE_DATA_DIR") {
            return PathBuf::from(dir);
        }
        let root = Self::root_named(APP_DIR);
        Self::adopt_legacy_root(root, Self::root_named(LEGACY_APP_DIR))
    }

    /// Takes over the data directory of an earlier installation.
    ///
    /// Returns `root` when it is already there, the moved directory when the old one
    /// could be renamed, and the old directory when the rename fails (another product
    /// is running on it) — the installation keeps working either way.
    fn adopt_legacy_root(root: PathBuf, legacy: PathBuf) -> PathBuf {
        if root.exists() || !legacy.is_dir() {
            return root;
        }
        match std::fs::rename(&legacy, &root) {
            Ok(()) => {
                tracing::info!(
                    from = %legacy.display(),
                    to = %root.display(),
                    "moved the data directory out of the identifier folder"
                );
                root
            }
            Err(error) => {
                tracing::warn!(
                    from = %legacy.display(),
                    %error,
                    "keeping the data directory of the earlier installation"
                );
                legacy
            }
        }
    }

    /// `%LOCALAPPDATA%\<app_dir>` (or the platform's equivalent).
    fn root_named(app_dir: &str) -> PathBuf {
        #[cfg(windows)]
        {
            if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
                return PathBuf::from(dir).join(app_dir);
            }
        }
        if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
            return PathBuf::from(dir).join(app_dir);
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".local")
                .join("share")
                .join(app_dir);
        }
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(app_dir)
    }

    /// Default location of the configuration file.
    pub fn config_path() -> PathBuf {
        Self::data_root().join("config.toml")
    }

    /// Defaults of one product, without a configuration file yet.
    ///
    /// The products of one machine differ in the module they install, not in their
    /// paths or ports: they share data directory and database, and only one of them
    /// runs at a time because the embedded database is a single instance.
    pub fn for_product(product: Product) -> Self {
        Self {
            product,
            ..Self::default()
        }
    }

    /// Loads the settings from the default file, honouring `WHERE_CONFIG`.
    ///
    /// The desktop shell uses this: its configuration file *is* the
    /// installation. The API server binary prefers [`Settings::from_environment`]
    /// so a served machine never picks up a workstation file by accident.
    pub fn load() -> Result<Self> {
        let path = std::env::var_os("WHERE_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(Self::config_path);
        Self::load_from(&path)
    }

    /// Defaults overridden by the environment, without any configuration file.
    pub fn from_environment() -> Self {
        let mut settings = Self::default();
        settings.apply_environment();
        settings
    }

    /// Loads the settings from `path`.
    ///
    /// A missing file is not an error: the defaults apply, which keeps the
    /// `.env` driven development workflow untouched.
    pub fn load_from(path: &Path) -> Result<Self> {
        let mut settings = if path.is_file() {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading configuration {}", path.display()))?;
            toml::from_str::<Settings>(&text)
                .with_context(|| format!("parsing configuration {}", path.display()))?
        } else {
            Self::default()
        };
        settings.apply_environment();
        Ok(settings)
    }

    /// Writes the settings as TOML, creating the parent directory.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serialising the configuration")?;
        std::fs::write(path, text)
            .with_context(|| format!("writing configuration {}", path.display()))?;
        Ok(())
    }

    /// Environment variables win over the file, as they always did.
    fn apply_environment(&mut self) {
        if let Ok(value) = std::env::var("WHERE_PRODUCT")
            && let Some(product) = parse_product(&value)
        {
            self.product = product;
        }
        if let Ok(value) = std::env::var("WHERE_MODE")
            && let Ok(mode) = toml::from_str::<Mode>(&format!("\"{value}\""))
        {
            self.mode = mode;
        }
        if let Ok(value) = std::env::var("WHERE_API_BASE") {
            self.api_base = Some(value);
        }
        if let Ok(value) = std::env::var("DATABASE_URL") {
            self.database_url = Some(value);
        }
        if let Ok(value) = std::env::var("BIND_ADDR") {
            self.bind = value;
        }
        if let Ok(value) = std::env::var("DB_MAX_CONNECTIONS")
            && let Ok(n) = value.parse()
        {
            self.max_connections = n;
        }
        if let Ok(value) = std::env::var("RUN_MIGRATIONS") {
            self.run_migrations = value != "0";
        }
        if let Ok(value) = std::env::var("CORS_ALLOWED_ORIGINS") {
            self.cors_allowed_origins = Some(value);
        }
        if let Ok(value) = std::env::var("FILE_STORAGE_DIR") {
            self.file_storage_dir = PathBuf::from(value);
        }
        if let Ok(value) = std::env::var("WHERE_PG_PORT")
            && let Ok(port) = value.parse()
        {
            self.embedded.port = port;
        }
        if let Ok(value) = std::env::var("TENANT_DATABASES") {
            self.tenant_databases = parse_tenant_databases(&value);
        }
    }

    /// The tenant databases, parsed to tenant id and connection string.
    ///
    /// Entries whose key is not a number are skipped: the register uses
    /// `"Mandanten"."Primary_Key"`, and a typo must not silently become a pool.
    pub fn resolved_tenant_databases(&self) -> Vec<(i64, String)> {
        self.tenant_databases
            .iter()
            .filter_map(|(key, url)| key.trim().parse::<i64>().ok().map(|id| (id, url.clone())))
            .collect()
    }

    /// Parsed `bind` address.
    pub fn socket_addr(&self) -> Result<SocketAddr> {
        self.bind
            .parse()
            .with_context(|| format!("BIND_ADDR must be a socket address, found {}", self.bind))
    }

    /// Connection string for the API process.
    ///
    /// In [`Mode::Standalone`] the embedded instance is the source when no
    /// explicit `database_url` is configured.
    pub fn resolved_database_url(&self) -> Option<String> {
        match (&self.database_url, self.mode) {
            (Some(url), _) => Some(url.clone()),
            (None, Mode::Standalone) => Some(self.embedded.database_url()),
            (None, Mode::Client) => None,
        }
    }
}

/// Reads a product from a bare value (`people`, `stuff`), as the environment carries it.
///
/// The names are the ones the TOML file uses (`#[serde(rename_all = "kebab-case")]`);
/// a test keeps both in step.
fn parse_product(value: &str) -> Option<Product> {
    match value.trim() {
        "stuff" => Some(Product::Stuff),
        "people" => Some(Product::People),
        _ => None,
    }
}

/// Splits `2=postgres://…;3=postgres://…` into the map of tenant databases.
fn parse_tenant_databases(value: &str) -> BTreeMap<String, String> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    for entry in value.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        if let Some((key, url)) = entry.split_once('=')
            && !key.trim().is_empty()
            && !url.trim().is_empty()
        {
            map.insert(key.trim().to_string(), url.trim().to_string());
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("where-config-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn defaults_are_a_single_workstation() {
        let s = Settings::default();
        assert_eq!(s.mode, Mode::Standalone);
        assert_eq!(s.bind, "127.0.0.1:8091");
        assert_eq!(s.max_connections, 10);
        assert!(s.run_migrations);
        assert!(s.embedded.port > 0);
        assert!(s.embedded.data_dir.ends_with("pg"));
        assert!(s.file_storage_dir.ends_with("files"));
    }

    #[test]
    fn missing_file_yields_defaults() {
        let dir = temp_dir("missing");
        let s = Settings::load_from(&dir.join("nope.toml")).unwrap();
        assert_eq!(s.mode, Mode::Standalone);
    }

    #[test]
    fn the_product_switch_names_the_module_and_its_identifier() {
        assert_eq!(Product::default(), Product::Stuff);
        assert_eq!(parse_product("people"), Some(Product::People));
        assert_eq!(parse_product(" stuff "), Some(Product::Stuff));
        assert_eq!(parse_product("money"), None);
        // The environment carries the same name as the TOML file.
        assert!(
            toml::to_string_pretty(&Settings::for_product(Product::People))
                .unwrap()
                .contains("product = \"people\"")
        );

        // The personnel product registers as a module, not as the core application.
        assert_eq!(Product::People.module(), where_modules::ModuleId::People);
        assert_eq!(Product::Stuff.module(), where_modules::ModuleId::Stuff);
        assert_eq!(Product::People.key(), "people");
        assert_eq!(Product::People.label(), "WHERE - are my people?");
        assert_eq!(
            Product::from_identifier("de.where.are-my-people"),
            Some(Product::People)
        );
        assert_eq!(
            Product::from_identifier("de.where.warehouse"),
            Some(Product::Stuff)
        );
        assert_eq!(Product::from_identifier("com.example.other"), None);
    }

    #[test]
    fn every_product_shares_directory_and_database() {
        let core = Settings::default();
        let people = Settings::for_product(Product::People);
        assert_eq!(people.product, Product::People);
        // One database for all products: the second one joins the first installation.
        assert_eq!(people.bind, core.bind);
        assert_eq!(people.embedded.port, core.embedded.port);
        assert_eq!(people.embedded.database, core.embedded.database);
        assert_eq!(people.embedded.data_dir, core.embedded.data_dir);
        // A file can carry the product — the key is part of the TOML.
        let text = toml::to_string_pretty(&people).unwrap();
        assert!(text.contains("product = \"people\""), "{text}");
    }

    #[test]
    fn the_data_directory_of_an_earlier_installation_is_moved() {
        let base = temp_dir("legacy-move");
        let legacy = base.join("de.where.lagerlogistik");
        let root = base.join("where-data");
        std::fs::create_dir_all(legacy.join("pg")).unwrap();
        std::fs::write(legacy.join("config.toml"), "product = \"people\"\n").unwrap();

        assert_eq!(
            Settings::adopt_legacy_root(root.clone(), legacy.clone()),
            root
        );
        assert!(root.join("config.toml").is_file());
        assert!(root.join("pg").is_dir());
        assert!(!legacy.exists());
        // The second start finds the new directory and leaves it alone.
        assert_eq!(
            Settings::adopt_legacy_root(root.clone(), legacy.clone()),
            root
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn an_existing_data_directory_wins_over_the_earlier_one() {
        let base = temp_dir("legacy-keep");
        let legacy = base.join("de.where.lagerlogistik");
        let root = base.join("where-data");
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&root).unwrap();

        assert_eq!(
            Settings::adopt_legacy_root(root.clone(), legacy.clone()),
            root
        );
        assert!(legacy.is_dir(), "the earlier directory stays untouched");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn file_round_trip_keeps_the_client_mode() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("config.toml");
        let s = Settings {
            mode: Mode::Client,
            api_base: Some("http://buero-server:8090".to_string()),
            file_storage_dir: dir.join("files"),
            ..Settings::default()
        };
        s.save_to(&path).unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded.mode, Mode::Client);
        assert_eq!(loaded.api_base.as_deref(), Some("http://buero-server:8090"));
        assert_eq!(loaded.file_storage_dir, dir.join("files"));
    }

    #[test]
    fn standalone_falls_back_to_the_embedded_database() {
        let s = Settings {
            embedded: Embedded {
                password: "geheim".to_string(),
                ..Embedded::default()
            },
            ..Settings::default()
        };
        assert_eq!(
            s.resolved_database_url().unwrap(),
            "postgres://postgres:geheim@127.0.0.1:5433/where"
        );
    }

    #[test]
    fn explicit_database_url_wins() {
        let s = Settings {
            database_url: Some("postgres://u:p@server:5432/where".to_string()),
            mode: Mode::Client,
            ..Settings::default()
        };
        assert_eq!(
            s.resolved_database_url().unwrap(),
            "postgres://u:p@server:5432/where"
        );
    }

    #[test]
    fn environment_only_settings_keep_the_defaults() {
        let s = Settings::from_environment();
        assert_eq!(s.mode, Mode::Standalone);
        assert_eq!(s.bind, "127.0.0.1:8091");
        assert!(s.embedded.data_dir.ends_with("pg"));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let dir = temp_dir("unknown");
        let path = dir.join("config.toml");
        std::fs::write(&path, "mode = \"client\"\ntypo_field = 1\n").unwrap();
        assert!(Settings::load_from(&path).is_err());
    }

    #[test]
    fn socket_addr_reports_the_offending_value() {
        let s = Settings {
            bind: "not-an-address".to_string(),
            ..Settings::default()
        };
        let error = s.socket_addr().unwrap_err().to_string();
        assert!(error.contains("not-an-address"));
    }

    #[test]
    fn tenant_databases_are_listed_by_tenant_id() {
        let value = "2=postgres://a/where_m2; 3=postgres://a/where_m3;;kaputt";
        let map = parse_tenant_databases(value);
        assert_eq!(
            map.get("2").map(String::as_str),
            Some("postgres://a/where_m2")
        );
        assert_eq!(
            map.get("3").map(String::as_str),
            Some("postgres://a/where_m3")
        );
        assert_eq!(map.len(), 2);

        let s = Settings {
            tenant_databases: map,
            ..Settings::default()
        };
        assert_eq!(
            s.resolved_tenant_databases(),
            vec![
                (2, "postgres://a/where_m2".to_string()),
                (3, "postgres://a/where_m3".to_string())
            ]
        );
    }

    #[test]
    fn a_tenant_database_without_a_numeric_key_is_ignored() {
        let s = Settings {
            tenant_databases: parse_tenant_databases("Theater Ulm=postgres://a/x"),
            ..Settings::default()
        };
        assert!(s.resolved_tenant_databases().is_empty());
        assert!(Settings::default().resolved_tenant_databases().is_empty());
    }
}
