//! Warehouses and stock.
//!
//! The stock of an article consists of **devices/batches** in `"Seriennummern"` — one
//! row per device (or batch with `"Menge"` > 1), with warehouse, bin location, condition and
//! usage areas. The totals per warehouse in `"Artikel_Env"` are derived from it; the
//! The mask shows both stacked (see `img/Lagerbestand_01.PNG`).
//!
//! A device can satisfy **several** usage areas at once (inventory, rental,
//! sale, used sale) — exactly as the legacy system models it on article level with separate
//! `*_Flag` columns. It is stored in `"where_inventory"`, `"where_rental"`,
//! `"where_sale"`, `"where_used_sale"`. The legacy field `"Einsatzgebiet"` has an
//! unknown semantic and is no longer written by WHERE.

use chrono::{NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sqlx::{PgConnection, PgPool};
use where_domain::DomainError;

use crate::error::db_error;

/// Upper bound for one goods receipt with serial numbers: a typo in the quantity must
/// not create thousands of rows.
const MAX_DEVICES_PER_RECEIPT: u64 = 500;

/// Stock of an article in one warehouse (upper grid of the form).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StockRow {
    pub stock_id: i32,
    pub stock_name: String,
    pub is_default: bool,
    pub total: Option<Decimal>,
    pub rental: Option<Decimal>,
    pub sale: Option<Decimal>,
    pub inventory: Option<Decimal>,
    pub used_sale: Option<Decimal>,
    pub reserved: Option<Decimal>,
    pub withdrawal: Option<Decimal>,
}

impl StockRow {
    /// Stock minus reservations and withdrawals.
    pub fn available(&self) -> Option<Decimal> {
        let total = self.total?;
        let weg = self.reserved.unwrap_or_default() + self.withdrawal.unwrap_or_default();
        Some(total - weg)
    }
}

/// One device or batch (lower grid, right side = device data).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeviceRow {
    pub id: i32,
    pub serial_no: Option<String>,
    pub inventory_no: Option<String>,
    pub inventory_no2: Option<String>,
    pub quantity: Option<Decimal>,
    /// Usage areas — several at the same time are allowed.
    pub inventory: bool,
    pub rental: bool,
    pub sale: bool,
    pub used_sale: bool,
    pub stock_id: Option<i32>,
    pub stock_name: Option<String>,
    pub bin_location: Option<String>,
    pub condition: Option<String>,
    pub discarded: bool,
    pub purchase_price: Option<Decimal>,
    pub purchase_date: Option<NaiveDate>,
    pub supplier_no: Option<i32>,
    pub owner: Option<String>,
    pub warranty_months: Option<i32>,
    pub ext_barcode: Option<String>,
    pub year_built: Option<i32>,
    pub note: Option<String>,
    pub without_serial: bool,
    pub recorded_at: Option<NaiveDate>,
}

impl DeviceRow {
    /// Status column of the form. The legacy `"VK_Status"` codes are unknown; WHERE
    /// distinguishes "in stock" and "discarded".
    pub fn status(&self) -> &'static str {
        if self.discarded {
            "ausgesondert"
        } else {
            "im Lager"
        }
    }

    /// Usage areas as text, e.g. "inventory, rental".
    pub fn usage_text(&self) -> String {
        let parts: Vec<&str> = [
            (self.inventory, "Inventar"),
            (self.rental, "Miete"),
            (self.sale, "Verkauf"),
            (self.used_sale, "Gebrauchtverkauf"),
        ]
        .into_iter()
        .filter_map(|(gesetzt, name)| gesetzt.then_some(name))
        .collect();
        if parts.is_empty() {
            "—".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Input for the goods receipt (`img/goods-receipt.PNG`).
#[derive(Debug, Clone)]
pub struct GoodsReceipt {
    pub serial_no: Option<String>,
    /// "without serial number capture"
    pub without_serial: bool,
    pub stock_id: i32,
    pub bin_location: Option<String>,
    pub condition: Option<String>,
    pub purchase_price: Option<Decimal>,
    pub purchase_date: Option<NaiveDate>,
    pub supplier_no: Option<i32>,
    pub inventory_no: Option<String>,
    /// "Reason for manual entry" — ends up in the note.
    pub reason: Option<String>,
    /// Usage areas — at least one, several at the same time are allowed.
    pub inventory: bool,
    pub rental: bool,
    pub sale: bool,
    pub used_sale: bool,
    pub quantity: Option<Decimal>,
}

/// Editable device data of the right-hand side.
#[derive(Debug, Clone, Default)]
pub struct DeviceUpdate {
    pub serial_no: Option<String>,
    pub inventory_no: Option<String>,
    pub inventory_no2: Option<String>,
    pub stock_id: Option<i32>,
    pub bin_location: Option<String>,
    pub condition: Option<String>,
    pub inventory: bool,
    pub rental: bool,
    pub sale: bool,
    pub used_sale: bool,
    pub discarded: bool,
    pub purchase_price: Option<Decimal>,
    pub purchase_date: Option<NaiveDate>,
    pub supplier_no: Option<i32>,
    pub owner: Option<String>,
    pub warranty_months: Option<i32>,
    pub ext_barcode: Option<String>,
    pub year_built: Option<i32>,
    pub note: Option<String>,
}

const STOCK_SQL: &str = r#"
SELECT l."Primary_Key" AS stock_id,
       COALESCE(NULLIF(l."Name", ''), 'Lager ' || l."Primary_Key") AS stock_name,
       COALESCE(l."__DefaultLager", false) AS is_default,
       e.total, e.rental, e.sale, e.inventory, e.used_sale, e.reserved, e.withdrawal
  FROM "Storage_Lager" l
  LEFT JOIN LATERAL (
      SELECT SUM("Bestand_Gesamt") AS total,
             SUM(COALESCE("Bestand_GesamtMiete", 0)) AS rental,
             SUM(COALESCE("Bestand_GesamtNVerkauf", 0)) AS sale,
             SUM(COALESCE("Bestand_GesamtInventar", 0)) AS inventory,
             SUM(COALESCE("Bestand_GesamtGVerkauf", 0)) AS used_sale,
             SUM(COALESCE("Bestand_Festblockade", 0)) AS reserved,
             SUM(COALESCE("Bestand_EntnahmeGesamt", 0)) AS withdrawal
        FROM "Artikel_Env"
       WHERE "Artikel_Primkey" = $1 AND "Lager_Primkey" = l."Primary_Key"
        HAVING COUNT(*) > 0
  ) e ON true
 ORDER BY COALESCE(l."__DefaultLager", false) DESC, stock_name"#;

const DEVICES_SQL: &str = r#"
SELECT g."Primary_Key" AS id,
       g."SerienNr" AS serial_no,
       g."InventarNr" AS inventory_no,
       g."InventarNr2" AS inventory_no2,
       g."Menge" AS quantity,
       COALESCE(g."where_inventory", false) AS inventory,
       COALESCE(g."where_rental", false) AS rental,
       COALESCE(g."where_sale", false) AS sale,
       COALESCE(g."where_used_sale", false) AS used_sale,
       g."Lager_Primkey" AS stock_id,
       l."Name" AS stock_name,
       g."RegalFach" AS bin_location,
       g."Zustand" AS condition,
       COALESCE(g."Ausgesondert", false) AS discarded,
       g."Kaufpreis" AS purchase_price,
       g."Kaufdatum" AS purchase_date,
       g."LieferantenNr" AS supplier_no,
       g."Eigentuemer" AS owner,
       g."GarantieZeit" AS warranty_months,
       g."extBarcode" AS ext_barcode,
       g."Baujahr" AS year_built,
       g."Bemerkung" AS note,
       COALESCE(g."ohneSN", false) AS without_serial,
       g."sn_createDate" AS recorded_at
  FROM "Seriennummern" g
  LEFT JOIN "Storage_Lager" l ON l."Primary_Key" = g."Lager_Primkey"
 WHERE g."Artikel_PrimKey" = $1
   AND ($2::text IS NULL
        OR g."SerienNr" ILIKE '%' || $2 || '%'
        OR g."InventarNr" ILIKE '%' || $2 || '%'
        OR g."InventarNr2" ILIKE '%' || $2 || '%'
        OR g."RegalFach" ILIKE '%' || $2 || '%'
        OR g."Zustand" ILIKE '%' || $2 || '%')
 ORDER BY COALESCE(g."Ausgesondert", false), g."Primary_Key" DESC"#;

const DEVICE_SQL: &str = r#"
SELECT g."Primary_Key" AS id,
       g."SerienNr" AS serial_no,
       g."InventarNr" AS inventory_no,
       g."InventarNr2" AS inventory_no2,
       g."Menge" AS quantity,
       COALESCE(g."where_inventory", false) AS inventory,
       COALESCE(g."where_rental", false) AS rental,
       COALESCE(g."where_sale", false) AS sale,
       COALESCE(g."where_used_sale", false) AS used_sale,
       g."Lager_Primkey" AS stock_id,
       l."Name" AS stock_name,
       g."RegalFach" AS bin_location,
       g."Zustand" AS condition,
       COALESCE(g."Ausgesondert", false) AS discarded,
       g."Kaufpreis" AS purchase_price,
       g."Kaufdatum" AS purchase_date,
       g."LieferantenNr" AS supplier_no,
       g."Eigentuemer" AS owner,
       g."GarantieZeit" AS warranty_months,
       g."extBarcode" AS ext_barcode,
       g."Baujahr" AS year_built,
       g."Bemerkung" AS note,
       COALESCE(g."ohneSN", false) AS without_serial,
       g."sn_createDate" AS recorded_at
  FROM "Seriennummern" g
  LEFT JOIN "Storage_Lager" l ON l."Primary_Key" = g."Lager_Primkey"
 WHERE g."Artikel_PrimKey" = $1 AND g."Primary_Key" = $2"#;

/// Goods receipt: one row per device/batch. `$16` = date, `$17`/`$18` = time/user.
const GOODS_RECEIPT_SQL: &str = r#"
INSERT INTO "Seriennummern"
       ("Artikel_PrimKey", "ArtikelNr", "Artikel", "SerienNr", "ohneSN", "Menge",
        "where_inventory", "where_rental", "where_sale", "where_used_sale",
        "Lager_Primkey", "Basislager_Primkey", "RegalFach", "Zustand",
        "Kaufpreis", "Kaufdatum", "LieferantenNr", "InventarNr", "Bemerkung",
        "Eingang_Date", "sn_createDate", "sn_createTime", "sn_createUser",
        "sn_changeDate", "sn_changeTime", "sn_changeUser", "MandantenID", "Ausgesondert")
VALUES ($1,
        (SELECT "ArtikelNr" FROM "Artikel" WHERE "Primary_Key" = $1),
        (SELECT "Name" FROM "Artikel" WHERE "Primary_Key" = $1),
        $2, $3, $4, $5, $6, $7, $8, $9, $9, $10, $11, $12, $13, $14, $15, $16,
        $17, $17, $18, $19, $17, $18, $19, 0, false)
RETURNING "Primary_Key""#;

const SERIAL_DUPLICATE_SQL: &str = r#"
SELECT 1 FROM "Seriennummern"
 WHERE "Artikel_PrimKey" = $1 AND lower(btrim("SerienNr")) = lower(btrim($2))
 LIMIT 1"#;

const DEVICE_UPDATE_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "SerienNr" = $3,
       "InventarNr" = $4,
       "InventarNr2" = $5,
       "Lager_Primkey" = $6,
       "RegalFach" = $7,
       "Zustand" = $8,
       "where_inventory" = $9,
       "where_rental" = $10,
       "where_sale" = $11,
       "where_used_sale" = $12,
       "Ausgesondert" = $13,
       "Kaufpreis" = $14,
       "Kaufdatum" = $15,
       "LieferantenNr" = $16,
       "Eigentuemer" = $17,
       "GarantieZeit" = $18,
       "extBarcode" = $19,
       "Baujahr" = $20,
       "Bemerkung" = $21,
       "sn_changeDate" = $22,
       "sn_changeTime" = $23,
       "sn_changeUser" = $24
 WHERE "Artikel_PrimKey" = $1 AND "Primary_Key" = $2"#;

const DEVICE_DELETE_SQL: &str = r#"
DELETE FROM "Seriennummern"
 WHERE "Artikel_PrimKey" = $1 AND "Primary_Key" = $2
RETURNING "Lager_Primkey""#;

/// Totals of the active devices of a warehouse — the basis of `"Artikel_Env"`.
/// The usage columns can overlap: a device counts in every usage
/// area it satisfies.
const TOTALS_SQL: &str = r#"
SELECT COALESCE(SUM(COALESCE("Menge", 1)), 0) AS total,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_inventory", false)), 0) AS inventory,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_sale", false)), 0) AS sale,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_rental", false)), 0) AS rental,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_used_sale", false)), 0) AS used_sale
  FROM "Seriennummern"
 WHERE "Artikel_PrimKey" = $1
   AND "Lager_Primkey" = $2
   AND COALESCE("Ausgesondert", false) = false"#;

const ENV_UPDATE_SQL: &str = r#"
UPDATE "Artikel_Env"
   SET "Bestand_Gesamt" = $3,
       "Bestand_GesamtMiete" = $4,
       "Bestand_GesamtNVerkauf" = $5,
       "Bestand_GesamtInventar" = $6,
       "Bestand_GesamtGVerkauf" = $7,
       "lastUpdate_TS" = $8,
       "Artikel_Nr" = (SELECT "ArtikelNr" FROM "Artikel" WHERE "Primary_Key" = $1)
 WHERE "Artikel_Primkey" = $1 AND "Lager_Primkey" = $2"#;

const ENV_INSERT_SQL: &str = r#"
INSERT INTO "Artikel_Env"
       ("Artikel_Primkey", "Lager_Primkey", "Artikel_Nr", "Bestand_Gesamt",
        "Bestand_GesamtMiete", "Bestand_GesamtNVerkauf", "Bestand_GesamtInventar",
        "Bestand_GesamtGVerkauf", "Bestand_Festblockade", "Bestand_EntnahmeGesamt",
        "Mandanten_ID", "lastUpdate_TS")
VALUES ($1, $2,
        (SELECT "ArtikelNr" FROM "Artikel" WHERE "Primary_Key" = $1),
        $3, $4, $5, $6, $7, 0, 0, 0, $8)"#;

const ARTICLE_EXISTS_SQL: &str = r#"SELECT 1 FROM "Artikel" WHERE "Primary_Key" = $1"#;
const STOCK_EXISTS_SQL: &str = r#"SELECT 1 FROM "Storage_Lager" WHERE "Primary_Key" = $1"#;
const WAREHOUSES_SQL: &str = r#"
SELECT "Primary_Key" AS id, "Name" AS name, COALESCE("__DefaultLager", false) AS is_default
  FROM "Storage_Lager"
 ORDER BY "__DefaultLager" DESC NULLS LAST, "Name""#;

/// Warehouse of the "Lager" select of the package slip search.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WarehouseRow {
    pub id: i32,
    pub name: Option<String>,
    pub is_default: bool,
}

#[derive(Clone)]
pub struct StockRepository {
    pool: PgPool,
}

impl StockRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Warehouses for the "Lager" select of the search and receipt forms.
    pub async fn warehouses(&self) -> Result<Vec<WarehouseRow>, DomainError> {
        sqlx::query_as::<_, WarehouseRow>(WAREHOUSES_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Stock per warehouse (upper grid).
    pub async fn stock(&self, article_id: i32) -> Result<Vec<StockRow>, DomainError> {
        self.validate_article(article_id).await?;
        sqlx::query_as::<_, StockRow>(STOCK_SQL)
            .bind(article_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Devices/batches of the article (lower grid).
    pub async fn devices(
        &self,
        article_id: i32,
        search: Option<&str>,
    ) -> Result<Vec<DeviceRow>, DomainError> {
        self.validate_article(article_id).await?;
        sqlx::query_as::<_, DeviceRow>(DEVICES_SQL)
            .bind(article_id)
            .bind(search)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// One device.
    pub async fn device(&self, article_id: i32, geraet_id: i32) -> Result<DeviceRow, DomainError> {
        sqlx::query_as::<_, DeviceRow>(DEVICE_SQL)
            .bind(article_id)
            .bind(geraet_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Device",
                id: geraet_id,
            })
    }

    /// Goods receipt: adds the devices of one receipt to stock.
    ///
    /// With serial numbers, `quantity` is the **number of devices**: every device gets
    /// its own row and its own serial number, continuing the one that was entered
    /// (`…-001` → `…-002`, …). A batch without serial numbers stays one row whose
    /// `"Menge"` is the quantity — that is what the checkbox is for.
    pub async fn goods_receipt(
        &self,
        article_id: i32,
        eingang: &GoodsReceipt,
    ) -> Result<Vec<DeviceRow>, DomainError> {
        self.validate_article(article_id).await?;
        Self::validate_stock_id(eingang.stock_id)?;
        Self::validate_usage(
            eingang.inventory,
            eingang.rental,
            eingang.sale,
            eingang.used_sale,
        )?;

        let quantity = eingang.quantity.unwrap_or(Decimal::ONE);
        if quantity <= Decimal::ZERO {
            return Err(DomainError::Validation(
                "Quantity must be greater than 0".into(),
            ));
        }
        if eingang.purchase_price.unwrap_or_default() < Decimal::ZERO {
            return Err(DomainError::Validation(
                "Purchase price must not be negative".into(),
            ));
        }
        if eingang.supplier_no.unwrap_or_default() < 0 {
            return Err(DomainError::Validation(
                "Supplier number must not be negative".into(),
            ));
        }
        let serial_no = Self::validate_serial_number(&eingang.serial_no, eingang.without_serial)?;

        // Serial numbers: one device per unit, each with its own number. A batch
        // (`ohne Seriennummernerfassung`) keeps the quantity in one row.
        let anzahl = if eingang.without_serial {
            1
        } else {
            Self::count_devices(quantity)?
        };
        let menge_je_geraet = if eingang.without_serial {
            quantity
        } else {
            Decimal::ONE
        };

        let stock_exists: Option<(i32,)> = sqlx::query_as(STOCK_EXISTS_SQL)
            .bind(eingang.stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if stock_exists.is_none() {
            return Err(DomainError::NotFound {
                entity: "Stock",
                id: eingang.stock_id,
            });
        }

        let today = Self::today();
        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let mut ids = Vec::with_capacity(anzahl as usize);
        for index in 0..anzahl {
            let nummer = match &serial_no {
                Some(basis) => Some(Self::serial_number_at(basis, index)?),
                None => None,
            };
            // The check runs inside the transaction: that way it also finds the numbers
            // this receipt has just created itself.
            if let Some(nr) = &nummer {
                let duplicate: Option<(i32,)> = sqlx::query_as(SERIAL_DUPLICATE_SQL)
                    .bind(article_id)
                    .bind(nr)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_error)?;
                if duplicate.is_some() {
                    return Err(DomainError::Conflict(format!(
                        "Serial number '{nr}' is already recorded for this article"
                    )));
                }
            }
            let (id,): (i32,) = sqlx::query_as(GOODS_RECEIPT_SQL)
                .bind(article_id)
                .bind(nummer.as_deref())
                .bind(eingang.without_serial)
                .bind(menge_je_geraet)
                .bind(eingang.inventory)
                .bind(eingang.rental)
                .bind(eingang.sale)
                .bind(eingang.used_sale)
                .bind(eingang.stock_id)
                .bind(eingang.bin_location.as_deref())
                .bind(eingang.condition.as_deref())
                .bind(eingang.purchase_price)
                .bind(eingang.purchase_date)
                .bind(eingang.supplier_no)
                .bind(eingang.inventory_no.as_deref())
                .bind(eingang.reason.as_deref())
                .bind(today)
                .bind(zeit)
                .bind(&user)
                .fetch_one(&mut *tx)
                .await
                .map_err(db_error)?;
            ids.push(id);
        }

        Self::recompute_totals(&mut tx, article_id, eingang.stock_id).await?;
        tx.commit().await.map_err(db_error)?;

        let mut geraete = Vec::with_capacity(ids.len());
        for id in ids {
            geraete.push(self.device(article_id, id).await?);
        }
        Ok(geraete)
    }

    /// Changes device data; stock is recomputed for the old and the new warehouse.
    pub async fn update_device(
        &self,
        article_id: i32,
        geraet_id: i32,
        update: &DeviceUpdate,
    ) -> Result<DeviceRow, DomainError> {
        let before = self.device(article_id, geraet_id).await?;

        let stock_id = match update.stock_id.or(before.stock_id) {
            Some(id) => {
                Self::validate_stock_id(id)?;
                let exists: Option<(i32,)> = sqlx::query_as(STOCK_EXISTS_SQL)
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(db_error)?;
                if exists.is_none() {
                    return Err(DomainError::NotFound {
                        entity: "Stock",
                        id,
                    });
                }
                id
            }
            None => {
                return Err(DomainError::Validation(
                    "Device has no warehouse — a warehouse is required".into(),
                ));
            }
        };
        Self::validate_usage(
            update.inventory,
            update.rental,
            update.sale,
            update.used_sale,
        )?;

        if update.purchase_price.unwrap_or_default() < Decimal::ZERO {
            return Err(DomainError::Validation(
                "Purchase price must not be negative".into(),
            ));
        }
        if update.warranty_months.unwrap_or_default() < 0 {
            return Err(DomainError::Validation(
                "Warranty period must not be negative".into(),
            ));
        }

        let chosen_number = update
            .serial_no
            .clone()
            .or_else(|| before.serial_no.clone());
        let serial_no = Self::validate_serial_number(&chosen_number, before.without_serial)?;
        if let Some(nr) = &serial_no {
            let duplicate: Option<(i32,)> = sqlx::query_as(SERIAL_DUPLICATE_SQL)
                .bind(article_id)
                .bind(nr)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_error)?;
            if duplicate.is_some() && before.serial_no.as_deref() != Some(nr.as_str()) {
                return Err(DomainError::Conflict(format!(
                    "Serial number '{nr}' is already recorded for this article"
                )));
            }
        }

        let today = Self::today();
        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(DEVICE_UPDATE_SQL)
            .bind(article_id)
            .bind(geraet_id)
            .bind(serial_no.as_deref())
            .bind(update.inventory_no.as_deref())
            .bind(update.inventory_no2.as_deref())
            .bind(stock_id)
            .bind(update.bin_location.as_deref())
            .bind(update.condition.as_deref())
            .bind(update.inventory)
            .bind(update.rental)
            .bind(update.sale)
            .bind(update.used_sale)
            .bind(update.discarded)
            .bind(update.purchase_price)
            .bind(update.purchase_date)
            .bind(update.supplier_no)
            .bind(update.owner.as_deref())
            .bind(update.warranty_months)
            .bind(update.ext_barcode.as_deref())
            .bind(update.year_built)
            .bind(update.note.as_deref())
            .bind(today)
            .bind(zeit)
            .bind(user)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

        Self::recompute_totals(&mut tx, article_id, stock_id).await?;
        if let Some(old) = before.stock_id.filter(|old| *old != stock_id) {
            Self::recompute_totals(&mut tx, article_id, old).await?;
        }
        tx.commit().await.map_err(db_error)?;

        self.device(article_id, geraet_id).await
    }

    /// Removes a device for good (form button "−"). To retire it, set
    /// `ausgesondert` instead.
    pub async fn delete_device(&self, article_id: i32, geraet_id: i32) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let stock: Option<(Option<i32>,)> = sqlx::query_as(DEVICE_DELETE_SQL)
            .bind(article_id)
            .bind(geraet_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        let (stock_id,) = stock.ok_or(DomainError::NotFound {
            entity: "Device",
            id: geraet_id,
        })?;
        if let Some(id) = stock_id {
            Self::recompute_totals(&mut tx, article_id, id).await?;
        }
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Recomputes `"Artikel_Env"` from the devices of the warehouse.
    async fn recompute_totals(
        conn: &mut PgConnection,
        article_id: i32,
        stock_id: i32,
    ) -> Result<(), DomainError> {
        let (total, inventory, sale, rental, used_sale): (
            Decimal,
            Decimal,
            Decimal,
            Decimal,
            Decimal,
        ) = sqlx::query_as(TOTALS_SQL)
            .bind(article_id)
            .bind(stock_id)
            .fetch_one(&mut *conn)
            .await
            .map_err(db_error)?;

        let ts = Utc::now().timestamp() as i32;
        let geaendert = sqlx::query(ENV_UPDATE_SQL)
            .bind(article_id)
            .bind(stock_id)
            .bind(total)
            .bind(rental)
            .bind(sale)
            .bind(inventory)
            .bind(used_sale)
            .bind(ts)
            .execute(&mut *conn)
            .await
            .map_err(db_error)?
            .rows_affected();

        if geaendert == 0 {
            sqlx::query(ENV_INSERT_SQL)
                .bind(article_id)
                .bind(stock_id)
                .bind(total)
                .bind(rental)
                .bind(sale)
                .bind(inventory)
                .bind(used_sale)
                .bind(ts)
                .execute(&mut *conn)
                .await
                .map_err(db_error)?;
        }
        Ok(())
    }

    fn validate_serial_number(
        serial_no: &Option<String>,
        without_serial: bool,
    ) -> Result<Option<String>, DomainError> {
        let nr = serial_no
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        match (nr, without_serial) {
            (Some(nr), _) => {
                if nr.chars().count() > 255 {
                    return Err(DomainError::Validation(
                        "Serial number must not exceed 255 characters".into(),
                    ));
                }
                Ok(Some(nr))
            }
            (None, true) => Ok(None),
            (None, false) => Err(DomainError::Validation(
                "serial number missing (or choose \"without serial number capture\")".into(),
            )),
        }
    }

    /// How many devices one receipt creates: the quantity must be a whole number.
    fn count_devices(quantity: Decimal) -> Result<u64, DomainError> {
        if !quantity.fract().is_zero() {
            return Err(DomainError::Validation(
                "Quantity must be a whole number for devices with a serial number".into(),
            ));
        }
        let anzahl = quantity.to_u64().unwrap_or(0);
        if anzahl == 0 {
            return Err(DomainError::Validation(
                "Quantity must be at least 1".into(),
            ));
        }
        if anzahl > MAX_DEVICES_PER_RECEIPT {
            return Err(DomainError::Validation(format!(
                "At most {MAX_DEVICES_PER_RECEIPT} devices can be booked in at once"
            )));
        }
        Ok(anzahl)
    }

    /// Serial number of the `schritt`-th device of one receipt: the digit block at the
    /// end counts up and keeps its width (`LX-001` + 2 → `LX-003`).
    fn serial_number_at(basis: &str, schritt: u64) -> Result<String, DomainError> {
        let prefix = basis.trim_end_matches(|c: char| c.is_ascii_digit());
        let ziffern = &basis[prefix.len()..];
        if ziffern.is_empty() {
            return Err(DomainError::Validation(format!(
                "For several devices the serial number needs a running number at the end \
                 (for example „{basis}-001“) — „{basis}“ has none"
            )));
        }
        let wert: u64 = ziffern.parse().map_err(|_| {
            DomainError::Validation(format!("Serial number '{basis}' cannot be continued"))
        })?;
        let neu = wert.checked_add(schritt).ok_or_else(|| {
            DomainError::Validation(format!("Serial number '{basis}' cannot be continued"))
        })?;
        Ok(format!("{prefix}{neu:0breite$}", breite = ziffern.len()))
    }

    fn validate_stock_id(stock_id: i32) -> Result<(), DomainError> {
        if stock_id <= 0 {
            return Err(DomainError::Validation("A warehouse is required".into()));
        }
        Ok(())
    }

    /// At least one usage area must be set — several at the same time are allowed.
    fn validate_usage(
        inventory: bool,
        rental: bool,
        sale: bool,
        used_sale: bool,
    ) -> Result<(), DomainError> {
        if !(inventory || rental || sale || used_sale) {
            return Err(DomainError::Validation(
                "At least one usage area must be selected (inventory, rental, sale or used sale)"
                    .into(),
            ));
        }
        Ok(())
    }

    fn today() -> NaiveDate {
        chrono::Local::now().date_naive()
    }

    fn now() -> (NaiveTime, String) {
        let now = chrono::Local::now();
        (now.time(), "where".to_string())
    }

    async fn validate_article(&self, article_id: i32) -> Result<(), DomainError> {
        let found: Option<(i32,)> = sqlx::query_as(ARTICLE_EXISTS_SQL)
            .bind(article_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if found.is_none() {
            return Err(DomainError::NotFound {
                entity: "Article",
                id: article_id,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        total: Option<Decimal>,
        reserved: Option<Decimal>,
        withdrawal: Option<Decimal>,
    ) -> StockRow {
        StockRow {
            stock_id: 1,
            stock_name: "Hauptlager".into(),
            is_default: true,
            total,
            rental: None,
            sale: None,
            inventory: None,
            used_sale: None,
            reserved,
            withdrawal,
        }
    }

    fn d(s: &str) -> Option<Decimal> {
        Some(s.parse().unwrap())
    }

    #[test]
    fn available_without_stock_is_unknown() {
        assert_eq!(row(None, None, None).available(), None);
    }

    #[test]
    fn available_without_reservation_is_stock() {
        assert_eq!(row(d("12"), None, None).available(), d("12"));
    }

    #[test]
    fn available_subtracts_reservation_and_withdrawal() {
        assert_eq!(row(d("10"), d("3"), d("2")).available(), d("5"));
    }

    #[test]
    fn serial_number_required_unless_without_serial() {
        assert!(StockRepository::validate_serial_number(&None, false).is_err());
        assert_eq!(
            StockRepository::validate_serial_number(&None, true).unwrap(),
            None
        );
        assert_eq!(
            StockRepository::validate_serial_number(&Some("  0815 ".into()), false).unwrap(),
            Some("0815".to_string())
        );
    }

    #[test]
    fn usage_areas_are_combinable() {
        assert!(StockRepository::validate_usage(true, false, false, false).is_ok());
        assert!(StockRepository::validate_usage(true, true, true, false).is_ok());
        assert!(StockRepository::validate_usage(true, true, true, true).is_ok());
        assert!(StockRepository::validate_usage(false, false, false, false).is_err());
    }

    #[test]
    fn usage_text_lists_all_areas() {
        let device = DeviceRow {
            id: 1,
            serial_no: Some("0815".into()),
            inventory_no: None,
            inventory_no2: None,
            quantity: d("1"),
            inventory: true,
            rental: true,
            sale: false,
            used_sale: true,
            stock_id: Some(1),
            stock_name: None,
            bin_location: None,
            condition: None,
            discarded: false,
            purchase_price: None,
            purchase_date: None,
            supplier_no: None,
            owner: None,
            warranty_months: None,
            ext_barcode: None,
            year_built: None,
            note: None,
            without_serial: false,
            recorded_at: None,
        };
        assert_eq!(
            device.usage_text(),
            "Inventar, Miete, Gebrauchtverkauf".to_string()
        );
    }
}
