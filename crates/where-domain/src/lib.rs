//! Domain layer: entities, value objects and errors.
//!
//! Field names are English while the persistence layer maps them onto the
//! reconstructed legacy column names (German, historical). See
//! `crates/where-persistence/src/rows.rs` for the mapping.

pub mod address;
pub mod agenda;
pub mod article;
pub mod auth;
pub mod dashboard;
pub mod error;
pub mod file;
pub mod inspection;
pub mod paging;
pub mod permissions;
pub mod settings;
pub mod tenants;
pub mod users;

pub use address::{Address, AddressInput};
pub use agenda::{
    AgendaBirthday, AgendaCategory, AgendaCategoryInput, AgendaEvent, AgendaEventInput,
    AgendaOccurrence, AgendaRange, AgendaRecurrence, RecurrenceFrequency,
};
pub use article::{
    Article, ArticlePricing, ArticlePyro, ArticleSupplier, ArticleTexts, ScaleCategory, ScaleStep,
};
pub use auth::{
    ADMIN_RIGHT, LoginRequest, LoginResponse, LoginTenant, LoginUser, PasswordChange, SessionUser,
};
pub use dashboard::{
    AddressStats, ArticleStats, Dashboard, GroupShare, PyroStats, QualityStats, RecentAddress,
    RecentArticle, StockStats, ValueCount, WarehouseStock,
};
pub use error::DomainError;
pub use file::ArticleFile;
pub use inspection::{ArticleImage, ArticleImageFile, InspectionReport};
pub use paging::{Page, PageRequest};
pub use permissions::{
    PermissionArea, PermissionGroup, PermissionRight, all_right_keys, catalogue, right_count,
};
pub use settings::{
    FieldKind, PrefInput, PrefValues, Prefs, SettingsArea, SettingsField, SettingsInput,
    SettingsList, SettingsRow, SettingsValues,
};
pub use tenants::{
    CountryEntry, PrintLanguageEntry, Tenant, TenantInput, TenantReference, country_name,
};
pub use users::{
    GroupMember, GroupRef, PreparedUser, User, UserGroup, UserGroupInput, UserInput, UserSummary,
    UserView, join_color, split_color,
};
