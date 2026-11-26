//! Aggregates for the start page (`Zentrale`).
//!
//! One snapshot of the master data: how much is there, what is missing and what
//! changed recently. The queries are deliberately separate and small — the start
//! page is opened once per session, so readability beats a single mega-statement,
//! and a missing optional table can be spotted in the log by name.

use rust_decimal::Decimal;
use sqlx::PgPool;
use where_domain::{
    AddressStats, ArticleStats, Dashboard, DomainError, GroupShare, PyroStats, QualityStats,
    RecentAddress, RecentArticle, StockStats, ValueCount, WarehouseStock,
};

use crate::error::db_error;

/// How many rows the "recent" lists show.
const RECENT_LIMIT: i64 = 5;
/// How many groups the distribution shows.
const GROUP_LIMIT: i64 = 8;
/// How many hazard classes the distribution shows.
const CLASS_LIMIT: i64 = 6;

const ARTICLES_SQL: &str = r#"
SELECT count(*) AS total,
       count(*) FILTER (WHERE COALESCE(a."Aktiv", false)) AS active,
       count(*) FILTER (WHERE NOT COALESCE(a."Aktiv", false)) AS inactive,
       count(*) FILTER (WHERE COALESCE(a."virtuellerArtikel", false)) AS virtual_articles,
       count(*) FILTER (WHERE a."audit_created_at" >= now() - interval '30 days') AS created_last_30_days,
       count(*) FILTER (WHERE a."audit_changed_at" >= now() - interval '30 days') AS changed_last_30_days
  FROM "Artikel" a"#;

const ADDRESSES_SQL: &str = r#"
SELECT count(*) AS total,
       count(*) FILTER (WHERE COALESCE(d."active", false)) AS active,
       count(*) FILTER (WHERE COALESCE(d."Type_Kunde", false)) AS customers,
       count(*) FILTER (WHERE COALESCE(d."Type_Lieferant", false)) AS suppliers,
       count(*) FILTER (WHERE COALESCE(d."Type_Interessent", false)) AS interested,
       count(*) FILTER (WHERE COALESCE(d."Type_Location", false)) AS locations,
       count(*) FILTER (WHERE COALESCE(d."Type_VIP", false)) AS vip,
       count(*) FILTER (WHERE COALESCE(d."Type_Personal", false)) AS employees
  FROM "Adresse" d"#;

/// Stock of the rental: `"Bestand_GesamtMiete"`.
///
/// An article that was set inactive keeps its rows in `"Artikel_Env"` — the stock is not
/// deleted, it is only no longer reported.
const STOCK_SQL: &str = r#"
SELECT COALESCE(SUM(COALESCE(e."Bestand_GesamtMiete", 0)), 0) AS rental,
       COALESCE(SUM(COALESCE(e."Bestand_GesamtNVerkauf", 0)
                    + COALESCE(e."Bestand_GesamtGVerkauf", 0)), 0) AS sale,
       COALESCE(SUM(COALESCE(e."Bestand_Festblockade", 0)), 0) AS reserved,
       COALESCE(SUM(COALESCE(e."Bestand_EntnahmeGesamt", 0)), 0) AS withdrawn
  FROM "Artikel_Env" e
  JOIN "Artikel" a ON a."Primary_Key" = e."Artikel_Primkey"
 WHERE COALESCE(a."Aktiv", false)"#;

const DEVICES_SQL: &str = r#"
SELECT count(*) AS devices,
       count(*) FILTER (WHERE COALESCE(g."Ausgesondert", false)) AS discarded
  FROM "Seriennummern" g
  JOIN "Artikel" a ON a."Primary_Key" = g."Artikel_PrimKey"
 WHERE COALESCE(a."Aktiv", false)"#;

/// Articles per group. A group can hold an article twice, so the members are counted
/// distinctly — the number is "articles in this group", not "rows in the link table".
const GROUPS_SQL: &str = r#"
SELECT w."Primary_Key" AS id,
       w."Index" AS name,
       count(DISTINCT aw."Artikel_Primkey") AS articles
  FROM "Storage_Warengruppe" w
  JOIN "Artikel_Warengruppe" aw ON aw."Storage_Primkey" = w."Primary_Key"
  JOIN "Artikel" a ON a."Primary_Key" = aw."Artikel_Primkey"
 WHERE COALESCE(a."Aktiv", false)
 GROUP BY w."Primary_Key", w."Index"
 ORDER BY count(DISTINCT aw."Artikel_Primkey") DESC, w."Index"
 LIMIT $1"#;

const PYRO_SQL: &str = r#"
SELECT count(*) FILTER (
           WHERE NULLIF(btrim(COALESCE(a."pyro_UN_number", '')), '') IS NOT NULL
       ) AS articles,
       count(*) FILTER (
           WHERE NULLIF(btrim(COALESCE(a."pyro_hazard_class_o", '')), '') IS NOT NULL
       ) AS articles_with_hazard_class
  FROM "Artikel" a
 WHERE COALESCE(a."Aktiv", false)"#;

const PYRO_CLASSES_SQL: &str = r#"
SELECT btrim(a."pyro_hazard_class_o") AS label,
       count(*) AS count
  FROM "Artikel" a
 WHERE COALESCE(a."Aktiv", false)
   AND NULLIF(btrim(COALESCE(a."pyro_hazard_class_o", '')), '') IS NOT NULL
 GROUP BY btrim(a."pyro_hazard_class_o")
 ORDER BY count(*) DESC, label
 LIMIT $1"#;

/// The quality checks only look at articles in use — an article that was set aside is
/// not a gap that somebody has to close.
const QUALITY_SQL: &str = r#"
SELECT (SELECT count(*) FROM "Artikel" a
         WHERE COALESCE(a."Aktiv", false)
           AND NOT EXISTS (SELECT 1 FROM "Artikel_Warengruppe" aw
                            WHERE aw."Artikel_Primkey" = a."Primary_Key")) AS articles_without_group,
       (SELECT count(*) FROM "Artikel" a
         WHERE COALESCE(a."Aktiv", false)
           AND NOT EXISTS (SELECT 1 FROM "Art_MandantDaten" m
                            WHERE m."Artikel_Primkey" = a."Primary_Key"
                              AND COALESCE(m."Mietpreis", 0) > 0)) AS articles_without_rental_price,
       (SELECT count(*) FROM "Artikel" a
         WHERE COALESCE(a."Aktiv", false)
           AND NOT EXISTS (SELECT 1 FROM "Artikel_Env" e
                            WHERE e."Artikel_Primkey" = a."Primary_Key"
                              AND COALESCE(e."Bestand_Gesamt", 0) > 0)) AS articles_without_stock,
       (SELECT count(*) FROM "Adresse" d
         WHERE NULLIF(btrim(COALESCE(d."import_Email", '')), '') IS NULL) AS addresses_without_email,
       (SELECT count(*) FROM "Adresse" d
         WHERE COALESCE(d."KundenNr", 0) = 0
           AND COALESCE(d."lief_LieferantenNr", 0) = 0) AS addresses_without_number"#;

const WAREHOUSES_SQL: &str = r#"
SELECT l."Primary_Key" AS id,
       COALESCE(NULLIF(l."Name", ''), 'Lager ' || l."Primary_Key") AS name,
       COALESCE(SUM(COALESCE(e."Bestand_GesamtMiete", 0)), 0) AS rental,
       COALESCE(SUM(COALESCE(e."Bestand_GesamtNVerkauf", 0)
                    + COALESCE(e."Bestand_GesamtGVerkauf", 0)), 0) AS sale
  FROM "Storage_Lager" l
  LEFT JOIN "Artikel_Env" e ON e."Lager_Primkey" = l."Primary_Key"
  LEFT JOIN "Artikel" a ON a."Primary_Key" = e."Artikel_Primkey"
 WHERE e."Primary_Key" IS NULL OR COALESCE(a."Aktiv", false)
 GROUP BY l."Primary_Key", l."Name"
 ORDER BY name"#;

const RECENT_ARTICLES_SQL: &str = r#"
SELECT a."Primary_Key" AS id,
       a."ArtikelNr" AS article_no,
       a."Name" AS name,
       (SELECT w."Index"
          FROM "Artikel_Warengruppe" aw
          JOIN "Storage_Warengruppe" w ON w."Primary_Key" = aw."Storage_Primkey"
         WHERE aw."Artikel_Primkey" = a."Primary_Key"
         ORDER BY w."Index"
         LIMIT 1) AS "group",
       COALESCE(a."audit_changed_at", a."audit_created_at") AS changed_at
  FROM "Artikel" a
 WHERE COALESCE(a."Aktiv", false)
 ORDER BY COALESCE(a."audit_changed_at", a."audit_created_at") DESC NULLS LAST,
          a."Primary_Key" DESC
 LIMIT $1"#;

/// The legacy address table has no creation timestamp; the identity column is the
/// order in which the rows came in, which is what "newest" means here.
const RECENT_ADDRESSES_SQL: &str = r#"
SELECT d."Primary_key" AS id,
       d."KundenNr" AS customer_no,
       COALESCE(NULLIF(d."Name1", ''), NULLIF(d."Name2", ''), d."Name3") AS name,
       d."import_Ort" AS city,
       d."GruppenNameAdresse" AS "group",
       COALESCE(d."active", false) AS active
  FROM "Adresse" d
 ORDER BY d."Primary_key" DESC
 LIMIT $1"#;

#[derive(Debug, sqlx::FromRow)]
struct ArticleCounts {
    total: i64,
    active: i64,
    inactive: i64,
    virtual_articles: i64,
    created_last_30_days: i64,
    changed_last_30_days: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct AddressCounts {
    total: i64,
    active: i64,
    customers: i64,
    suppliers: i64,
    interested: i64,
    locations: i64,
    vip: i64,
    employees: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct StockSums {
    rental: Decimal,
    sale: Decimal,
    reserved: Decimal,
    withdrawn: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct DeviceCounts {
    devices: i64,
    discarded: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct PyroCounts {
    articles: i64,
    articles_with_hazard_class: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct QualityCounts {
    articles_without_group: i64,
    articles_without_rental_price: i64,
    articles_without_stock: i64,
    addresses_without_email: i64,
    addresses_without_number: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct GroupRow {
    id: i32,
    name: Option<String>,
    articles: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct LabelCount {
    label: String,
    count: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct WarehouseRow {
    id: i32,
    name: String,
    rental: Decimal,
    sale: Decimal,
}

#[derive(Debug, sqlx::FromRow)]
struct ArticleRow {
    id: i32,
    article_no: i32,
    name: Option<String>,
    group: Option<String>,
    changed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, sqlx::FromRow)]
struct AddressRow {
    id: i32,
    customer_no: Option<i32>,
    name: Option<String>,
    city: Option<String>,
    group: Option<String>,
    active: bool,
}

/// Read-only aggregates of the start page.
#[derive(Clone)]
pub struct DashboardRepository {
    pool: PgPool,
}

impl DashboardRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// One snapshot. Every block is fetched separately so a slow aggregate does not
    /// hold back the others and the log names the failing one.
    pub async fn load(&self) -> Result<Dashboard, DomainError> {
        let articles = sqlx::query_as::<_, ArticleCounts>(ARTICLES_SQL)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        let addresses = sqlx::query_as::<_, AddressCounts>(ADDRESSES_SQL)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        let stock = sqlx::query_as::<_, StockSums>(STOCK_SQL)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        let devices = sqlx::query_as::<_, DeviceCounts>(DEVICES_SQL)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        let pyro = sqlx::query_as::<_, PyroCounts>(PYRO_SQL)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        let hazard_classes = sqlx::query_as::<_, LabelCount>(PYRO_CLASSES_SQL)
            .bind(CLASS_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let quality = sqlx::query_as::<_, QualityCounts>(QUALITY_SQL)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;

        let groups = sqlx::query_as::<_, GroupRow>(GROUPS_SQL)
            .bind(GROUP_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let warehouses = sqlx::query_as::<_, WarehouseRow>(WAREHOUSES_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let recent_articles = sqlx::query_as::<_, ArticleRow>(RECENT_ARTICLES_SQL)
            .bind(RECENT_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let recent_addresses = sqlx::query_as::<_, AddressRow>(RECENT_ADDRESSES_SQL)
            .bind(RECENT_LIMIT)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;

        Ok(Dashboard {
            articles: ArticleStats {
                total: articles.total,
                active: articles.active,
                inactive: articles.inactive,
                virtual_articles: articles.virtual_articles,
                created_last_30_days: articles.created_last_30_days,
                changed_last_30_days: articles.changed_last_30_days,
            },
            addresses: AddressStats {
                total: addresses.total,
                active: addresses.active,
                customers: addresses.customers,
                suppliers: addresses.suppliers,
                interested: addresses.interested,
                locations: addresses.locations,
                vip: addresses.vip,
                employees: addresses.employees,
            },
            stock: StockStats {
                devices: devices.devices,
                discarded_devices: devices.discarded,
                rental: stock.rental,
                sale: stock.sale,
                reserved: stock.reserved,
                withdrawn: stock.withdrawn,
            },
            pyro: PyroStats {
                articles: pyro.articles,
                articles_with_hazard_class: pyro.articles_with_hazard_class,
                hazard_classes: hazard_classes
                    .into_iter()
                    .map(|c| ValueCount {
                        label: c.label,
                        count: c.count,
                    })
                    .collect(),
            },
            quality: QualityStats {
                articles_without_group: quality.articles_without_group,
                articles_without_rental_price: quality.articles_without_rental_price,
                articles_without_stock: quality.articles_without_stock,
                addresses_without_email: quality.addresses_without_email,
                addresses_without_number: quality.addresses_without_number,
            },
            groups: groups
                .into_iter()
                .map(|g| GroupShare {
                    id: g.id,
                    name: g.name,
                    articles: g.articles,
                })
                .collect(),
            warehouses: warehouses
                .into_iter()
                .map(|w| WarehouseStock {
                    id: w.id,
                    name: w.name,
                    rental: w.rental,
                    sale: w.sale,
                })
                .collect(),
            recent_articles: recent_articles
                .into_iter()
                .map(|a| RecentArticle {
                    id: a.id,
                    article_no: a.article_no,
                    name: a.name,
                    group: a.group,
                    changed_at: a.changed_at,
                })
                .collect(),
            recent_addresses: recent_addresses
                .into_iter()
                .map(|d| RecentAddress {
                    id: d.id,
                    customer_no: d.customer_no,
                    name: d.name,
                    city: d.city,
                    group: d.group,
                    active: d.active,
                })
                .collect(),
        })
    }
}
