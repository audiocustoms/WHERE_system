//! Shared state handed to every handler.
//!
//! The installation's own database carries accounts, groups and the register of the
//! tenants. A tenant that has its own database (`tenant_databases` in the settings)
//! gets a **tenant state**: the same repositories over that pool. The middleware
//! [`crate::authz::authorize`] puts it next to the request, and [`State`] prefers it —
//! so a handler never has to know which database it is talking to.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use where_modules::ModuleState;
use where_persistence::{
    AddressRepository, AgendaRepository, ArticleGroupRepository, ArticleRepository,
    ArticleSearchRepository, AuthRepository, DashboardRepository, FileRepository,
    InspectionRepository, NumberSeriesRepository, PgPool, PlaceRepository, ProcurementRepository,
    PurchaseOrderRepository, SettingsRepository, StockRepository, StockValueRepository,
    StocktakeRepository, TenantRepository, UserRepository,
};

/// Pools of the tenants with a database of their own, keyed by
/// `"Mandanten"."Primary_Key"`.
#[derive(Clone, Default)]
pub struct TenantPools(Arc<HashMap<i64, PgPool>>);

impl TenantPools {
    pub fn new(pools: HashMap<i64, PgPool>) -> Self {
        Self(Arc::new(pools))
    }

    pub fn get(&self, mandant: i64) -> Option<&PgPool> {
        self.0.get(&mandant)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl std::fmt::Debug for TenantPools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut keys: Vec<i64> = self.0.keys().copied().collect();
        keys.sort_unstable();
        f.debug_struct("TenantPools")
            .field("tenants", &keys)
            .finish()
    }
}

/// Shared state handed to every handler.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub addresses: AddressRepository,
    pub article: ArticleRepository,
    pub article_search: ArticleSearchRepository,
    pub files: FileRepository,
    pub inspections: InspectionRepository,
    pub article_groups: ArticleGroupRepository,
    pub number_series: NumberSeriesRepository,
    pub stock: StockRepository,
    pub places: PlaceRepository,
    pub stocktakes: StocktakeRepository,
    pub stock_value: StockValueRepository,
    pub procurement: ProcurementRepository,
    pub orders: PurchaseOrderRepository,
    pub dashboard: DashboardRepository,
    pub agenda: AgendaRepository,
    pub settings: SettingsRepository,
    pub tenants: TenantRepository,
    pub users: UserRepository,
    pub auth: AuthRepository,
    /// Registered products of this installation (`WHERE - is my stuff?` and modules).
    pub modules: ModuleState,
    /// Databases of the tenants that have their own; empty in a tenant state.
    pub tenant_pools: TenantPools,
    /// The database of the installation (accounts, groups, tenants). A tenant state
    /// keeps it so that settings of the installation — the e-mail account of a user, for
    /// example — stay reachable from a module.
    install_pool: Option<PgPool>,
    file_root: std::path::PathBuf,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let root = std::env::var("FILE_STORAGE_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("data/files"));
        Self::with_file_root(pool, root)
    }

    /// Like [`AppState::new`], but with an explicit directory for stored files.
    ///
    /// The packaged application cannot rely on a working directory below
    /// `Program Files`, so the caller passes an absolute path from the runtime
    /// settings instead.
    pub fn with_file_root(pool: PgPool, file_root: impl Into<std::path::PathBuf>) -> Self {
        let file_root = file_root.into();
        Self {
            addresses: AddressRepository::new(pool.clone()),
            article: ArticleRepository::new(pool.clone()),
            article_search: ArticleSearchRepository::new(pool.clone()),
            files: FileRepository::with_root(pool.clone(), file_root.clone()),
            inspections: InspectionRepository::with_root(pool.clone(), file_root.clone()),
            article_groups: ArticleGroupRepository::new(pool.clone()),
            number_series: NumberSeriesRepository::new(pool.clone()),
            stock: StockRepository::new(pool.clone()),
            places: PlaceRepository::new(pool.clone()),
            stocktakes: StocktakeRepository::new(pool.clone()),
            stock_value: StockValueRepository::new(pool.clone()),
            procurement: ProcurementRepository::new(pool.clone()),
            orders: PurchaseOrderRepository::new(pool.clone()),
            dashboard: DashboardRepository::new(pool.clone()),
            agenda: AgendaRepository::new(pool.clone()),
            settings: SettingsRepository::new(pool.clone()),
            tenants: TenantRepository::new(pool.clone()),
            users: UserRepository::new(pool.clone()),
            auth: AuthRepository::new(pool.clone()),
            modules: ModuleState::default(),
            tenant_pools: TenantPools::default(),
            install_pool: None,
            file_root,
            pool,
        }
    }

    /// The database of the installation this state belongs to.
    ///
    /// It is the state's own pool outside of a tenant and the remembered one inside a
    /// tenant state, so a module can always read the accounts and their settings.
    pub fn installation_pool(&self) -> &PgPool {
        self.install_pool.as_ref().unwrap_or(&self.pool)
    }

    /// The tenant databases this installation knows.
    pub fn with_tenant_pools(mut self, pools: TenantPools) -> Self {
        self.tenant_pools = pools;
        self
    }

    /// The cached module registry, shared by every tenant state.
    pub fn with_modules(mut self, modules: ModuleState) -> Self {
        self.modules = modules;
        self
    }

    /// The state of one tenant's own database, `None` when it has none.
    ///
    /// The result carries an empty [`TenantPools`], so it cannot be nested again.
    pub fn for_tenant(&self, mandant: i64) -> Option<AppState> {
        let pool = self.tenant_pools.get(mandant)?.clone();
        let installation = self.installation_pool().clone();
        Some(
            Self::with_file_root(pool, self.file_root.clone())
                .with_modules(self.modules.clone())
                .with_install_pool(installation),
        )
    }

    /// Remembers the database of the installation in a tenant state.
    pub fn with_install_pool(mut self, pool: PgPool) -> Self {
        self.install_pool = Some(pool);
        self
    }
}

/// [`axum::extract::State`] with the tenant of the request in front.
///
/// The middleware stores the tenant state in the request when the session belongs to
/// a tenant with its own database; handlers keep asking for `State<AppState>` and get
/// that one. Without it the installation's own state applies.
pub struct State<T>(pub T);

impl FromRequestParts<AppState> for State<AppState> {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let chosen = parts
            .extensions
            .get::<AppState>()
            .cloned()
            .unwrap_or_else(|| state.clone());
        Ok(State(chosen))
    }
}
