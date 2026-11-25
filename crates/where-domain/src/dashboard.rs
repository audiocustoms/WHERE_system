//! Snapshot for the start page (`Zentrale`).
//!
//! Everything the dashboard shows is aggregated in the database: the numbers stay
//! correct when several workstations work on the data at the same time, and the
//! browser never has to load whole lists to count them.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Everything the start page needs in one read.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Dashboard {
    pub articles: ArticleStats,
    pub addresses: AddressStats,
    pub stock: StockStats,
    pub pyro: PyroStats,
    pub quality: QualityStats,
    /// Article groups with the most articles, largest first.
    pub groups: Vec<GroupShare>,
    /// Stock per warehouse.
    pub warehouses: Vec<WarehouseStock>,
    /// Articles that changed last, newest first.
    pub recent_articles: Vec<RecentArticle>,
    /// Addresses with the highest key, newest first.
    pub recent_addresses: Vec<RecentAddress>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ArticleStats {
    pub total: i64,
    pub active: i64,
    pub inactive: i64,
    /// Service articles without own stock (`"virtuellerArtikel"`).
    pub virtual_articles: i64,
    pub created_last_30_days: i64,
    pub changed_last_30_days: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AddressStats {
    pub total: i64,
    pub active: i64,
    pub customers: i64,
    pub suppliers: i64,
    pub interested: i64,
    pub locations: i64,
    pub vip: i64,
    pub employees: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct StockStats {
    /// Device and batch records in `"Seriennummern"`.
    pub devices: i64,
    pub discarded_devices: i64,
    /// Units available for rental, summed over all warehouses.
    pub rental: Decimal,
    /// Units for sale (new and used), summed over all warehouses.
    pub sale: Decimal,
    /// Units blocked for orders (`"Bestand_Festblockade"`).
    pub reserved: Decimal,
    /// Units taken out (`"Bestand_EntnahmeGesamt"`).
    pub withdrawn: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PyroStats {
    /// Articles with a UN number — the pyrotechnical register.
    pub articles: i64,
    /// Articles with a hazard class.
    pub articles_with_hazard_class: i64,
    /// Distribution of the hazard classes, largest group first.
    pub hazard_classes: Vec<ValueCount>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct QualityStats {
    pub articles_without_group: i64,
    pub articles_without_rental_price: i64,
    pub articles_without_stock: i64,
    pub addresses_without_email: i64,
    /// Neither a customer nor a supplier number.
    pub addresses_without_number: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GroupShare {
    pub id: i32,
    pub name: Option<String>,
    pub articles: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct WarehouseStock {
    pub id: i32,
    pub name: String,
    pub rental: Decimal,
    pub sale: Decimal,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct RecentArticle {
    pub id: i32,
    pub article_no: i32,
    pub name: Option<String>,
    pub group: Option<String>,
    pub changed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct RecentAddress {
    pub id: i32,
    pub customer_no: Option<i32>,
    pub name: Option<String>,
    pub city: Option<String>,
    pub group: Option<String>,
    pub active: bool,
}

/// One value and how often it occurs — used for the hazard class distribution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ValueCount {
    pub label: String,
    pub count: i64,
}
