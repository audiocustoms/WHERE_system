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

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
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
    /// The warehouse takes part in the working masks (`"where_zugriff"`).
    pub zugriff: bool,
    /// Material may be taken out of it (`"where_entnahme"`).
    pub entnahme: bool,
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
    /// Place the device stands at while it is issued (`NULL` = in the warehouse).
    pub place_id: Option<i32>,
    pub place_name: Option<String>,
    pub issued_from: Option<NaiveDate>,
    pub issued_until: Option<NaiveDate>,
}

impl DeviceRow {
    /// Status column of the form. The legacy `"VK_Status"` codes are unknown; WHERE
    /// distinguishes "in stock", "issued to a place" and "discarded".
    pub fn status(&self) -> String {
        if self.discarded {
            "ausgesondert".to_string()
        } else if let Some(place) = self.place_name.as_deref() {
            format!("ausgegeben: {place}")
        } else {
            "im Lager".to_string()
        }
    }

    /// `true` while the device stands at a place of the house.
    pub fn is_issued(&self) -> bool {
        self.place_id.is_some()
    }

    /// Quantity of this row — a single device counts one.
    pub fn amount(&self) -> Decimal {
        self.quantity.unwrap_or(Decimal::ONE)
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

/// One line of the stock history (`"where_stock_movement"`).
///
/// Warehouse names are resolved at read time; the ids stay in the row, so the entry
/// keeps its meaning when a warehouse is renamed.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MovementRow {
    pub id: i64,
    pub device_id: i32,
    pub serial_no: Option<String>,
    pub kind: String,
    pub quantity: Option<Decimal>,
    pub warehouse_from: Option<i32>,
    pub warehouse_from_name: Option<String>,
    pub warehouse_to: Option<i32>,
    pub warehouse_to_name: Option<String>,
    pub bin_from: Option<String>,
    pub bin_to: Option<String>,
    pub place_from: Option<i32>,
    pub place_from_name: Option<String>,
    pub place_to: Option<i32>,
    pub place_to_name: Option<String>,
    pub condition_from: Option<String>,
    pub condition_to: Option<String>,
    pub discarded_from: Option<bool>,
    pub discarded_to: Option<bool>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
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
    /// Line of a purchase order this delivery belongs to (`where_purchase_order_line`).
    pub order_line: Option<i32>,
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

/// Input for handing material out to a place of the house (`Ausgeben`).
#[derive(Debug, Clone)]
pub struct Issue {
    pub place_id: i32,
    /// Amount handed out; the whole row when omitted.
    pub quantity: Option<Decimal>,
    /// Day the material is expected back.
    pub until: Option<NaiveDate>,
    pub note: Option<String>,
}

const STOCK_SQL: &str = r#"
SELECT l."Primary_Key" AS stock_id,
       COALESCE(NULLIF(l."Name", ''), 'Lager ' || l."Primary_Key") AS stock_name,
       COALESCE(l."__DefaultLager", false) AS is_default,
       COALESCE(l."where_zugriff", false) AS zugriff,
       COALESCE(l."where_entnahme", false) AS entnahme,
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
       g."sn_createDate" AS recorded_at,
       g."where_place_id" AS place_id,
       p."name" AS place_name,
       g."where_issued_from" AS issued_from,
       g."where_issued_until" AS issued_until
  FROM "Seriennummern" g
  LEFT JOIN "Storage_Lager" l ON l."Primary_Key" = g."Lager_Primkey"
  LEFT JOIN "where_place" p ON p."id" = g."where_place_id"
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
       g."sn_createDate" AS recorded_at,
       g."where_place_id" AS place_id,
       p."name" AS place_name,
       g."where_issued_from" AS issued_from,
       g."where_issued_until" AS issued_until
  FROM "Seriennummern" g
  LEFT JOIN "Storage_Lager" l ON l."Primary_Key" = g."Lager_Primkey"
  LEFT JOIN "where_place" p ON p."id" = g."where_place_id"
 WHERE g."Artikel_PrimKey" = $1 AND g."Primary_Key" = $2"#;

/// Goods receipt: one row per device/batch. `$16` = date, `$17`/`$18` = time/user.
const GOODS_RECEIPT_SQL: &str = r#"
INSERT INTO "Seriennummern"
       ("Artikel_PrimKey", "ArtikelNr", "Artikel", "SerienNr", "ohneSN", "Menge",
        "where_inventory", "where_rental", "where_sale", "where_used_sale",
        "Lager_Primkey", "Basislager_Primkey", "RegalFach", "Zustand",
        "Kaufpreis", "Kaufdatum", "LieferantenNr", "InventarNr", "Bemerkung",
        "Eingang_Date", "sn_createDate", "sn_createTime", "sn_createUser",
        "sn_changeDate", "sn_changeTime", "sn_changeUser", "MandantenID", "Ausgesondert",
        "where_order_line")
VALUES ($1,
        (SELECT "ArtikelNr" FROM "Artikel" WHERE "Primary_Key" = $1),
        (SELECT "Name" FROM "Artikel" WHERE "Primary_Key" = $1),
        $2, $3, $4, $5, $6, $7, $8, $9, $9, $10, $11, $12, $13, $14, $15, $16,
        $17, $17, $18, $19, $17, $18, $19, 0, false, $20)
RETURNING "Primary_Key""#;

const SERIAL_DUPLICATE_SQL: &str = r#"
SELECT 1 FROM "Seriennummern"
 WHERE "Artikel_PrimKey" = $1 AND lower(btrim("SerienNr")) = lower(btrim($2))
 LIMIT 1"#;

/// One line of a purchase order as the goods receipt has to judge it.
const ORDER_LINE_SQL: &str = r#"
SELECT l."article_id",
       l."quantity",
       o."state",
       o."number",
       COALESCE((SELECT SUM(COALESCE(g."Menge", 0))
                   FROM "Seriennummern" g WHERE g."where_order_line" = l."id"), 0) AS received
  FROM "where_purchase_order_line" l
  JOIN "where_purchase_order" o ON o."id" = l."order_id"
 WHERE l."id" = $1"#;

/// An order whose lines are all delivered is done — no further receipt is expected.
const ORDER_SETTLE_SQL: &str = r#"
UPDATE "where_purchase_order" o
   SET "state" = 'done', "changed_at" = now()
 WHERE o."state" = 'open'
   AND o."id" = (SELECT "order_id" FROM "where_purchase_order_line" WHERE "id" = $1)
   AND NOT EXISTS (
       SELECT 1 FROM "where_purchase_order_line" l
        WHERE l."order_id" = o."id"
          AND COALESCE((SELECT SUM(COALESCE(g."Menge", 0))
                          FROM "Seriennummern" g WHERE g."where_order_line" = l."id"), 0)
              < l."quantity")"#;

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
RETURNING "Lager_Primkey", "SerienNr", "Menge", "RegalFach", "Zustand",
          COALESCE("Ausgesondert", false)"#;

/// One booking in the stock history. Written inside the transaction of the booking
/// itself, so a failed booking leaves no line behind.
const MOVEMENT_INSERT_SQL: &str = r#"
INSERT INTO "where_stock_movement"
       ("article_id", "device_id", "serial_no", "kind", "quantity",
        "warehouse_from", "warehouse_to", "bin_from", "bin_to",
        "condition_from", "condition_to", "discarded_from", "discarded_to",
        "place_from", "place_to", "place_from_label", "place_to_label",
        "note", "created_by")
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)"#;

/// History of an article, optionally of one device; newest first.
const MOVEMENTS_SQL: &str = r#"
SELECT m."id", m."device_id", m."serial_no", m."kind", m."quantity",
       m."warehouse_from", wf."Name" AS warehouse_from_name,
       m."warehouse_to", wt."Name" AS warehouse_to_name,
       m."bin_from", m."bin_to",
       m."place_from", COALESCE(pf."name", m."place_from_label") AS place_from_name,
       m."place_to", COALESCE(pt."name", m."place_to_label") AS place_to_name,
       m."condition_from", m."condition_to",
       m."discarded_from", m."discarded_to", m."note",
       m."created_at", m."created_by"
  FROM "where_stock_movement" m
  LEFT JOIN "Storage_Lager" wf ON wf."Primary_Key" = m."warehouse_from"
  LEFT JOIN "Storage_Lager" wt ON wt."Primary_Key" = m."warehouse_to"
  LEFT JOIN "where_place" pf ON pf."id" = m."place_from"
  LEFT JOIN "where_place" pt ON pt."id" = m."place_to"
 WHERE m."article_id" = $1
   AND ($2::int IS NULL OR m."device_id" = $2)
 ORDER BY m."created_at" DESC, m."id" DESC
 LIMIT $3"#;

/// Sets the place on a row: that is what makes it "issued".
const ISSUE_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "where_place_id" = $2,
       "where_issued_from" = $3,
       "where_issued_until" = $4,
       "sn_changeDate" = $5,
       "sn_changeTime" = $6,
       "sn_changeUser" = $7
 WHERE "Primary_Key" = $1"#;

/// Takes the place off a row: the material is back in the warehouse.
const RETURN_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "where_place_id" = NULL,
       "where_issued_from" = NULL,
       "where_issued_until" = NULL,
       "Zustand" = COALESCE($5, "Zustand"),
       "sn_changeDate" = $2,
       "sn_changeTime" = $3,
       "sn_changeUser" = $4
 WHERE "Primary_Key" = $1"#;

/// Splits `$3` off a batch: the new row carries the amount, the source keeps the rest.
/// The part has no serial number of its own — the batch number stays at the source and
/// the history keeps the reference through the device.
const SPLIT_SQL: &str = r#"
INSERT INTO "Seriennummern"
       ("Artikel_PrimKey", "ArtikelNr", "Artikel", "SerienNr", "ohneSN", "Menge",
        "where_inventory", "where_rental", "where_sale", "where_used_sale",
        "Lager_Primkey", "Basislager_Primkey", "RegalFach", "Zustand",
        "Kaufpreis", "Kaufdatum", "LieferantenNr", "InventarNr", "InventarNr2",
        "Bemerkung", "Eingang_Date", "MandantenID", "Ausgesondert",
        "sn_createDate", "sn_createTime", "sn_createUser",
        "sn_changeDate", "sn_changeTime", "sn_changeUser")
SELECT "Artikel_PrimKey", "ArtikelNr", "Artikel", NULL, "ohneSN", $3,
       COALESCE("where_inventory", false), COALESCE("where_rental", false),
       COALESCE("where_sale", false), COALESCE("where_used_sale", false),
       "Lager_Primkey", "Basislager_Primkey", "RegalFach", "Zustand",
       "Kaufpreis", "Kaufdatum", "LieferantenNr", '', '', "Bemerkung",
       "Eingang_Date", "MandantenID", false, $4, $5, $6, $4, $5, $6
  FROM "Seriennummern"
 WHERE "Artikel_PrimKey" = $1 AND "Primary_Key" = $2
RETURNING "Primary_Key""#;

const SPLIT_SOURCE_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "Menge" = COALESCE("Menge", 1) - $2,
       "sn_changeDate" = $3,
       "sn_changeTime" = $4,
       "sn_changeUser" = $5
 WHERE "Artikel_PrimKey" = $1 AND "Primary_Key" = $6"#;

/// Row of the same warehouse the returned part can go back into, so the grid does not
/// grow a second line for the same batch.
const MERGE_TARGET_SQL: &str = r#"
SELECT "Primary_Key"
  FROM "Seriennummern"
 WHERE "Artikel_PrimKey" = $1
   AND "Lager_Primkey" = $2
   AND "Primary_Key" <> $3
   AND "where_place_id" IS NULL
   AND COALESCE("Ausgesondert", false) = false
   AND COALESCE("SerienNr", '') = ''
   AND COALESCE("RegalFach", '') = COALESCE($4, '')
   AND COALESCE("Zustand", '') = COALESCE($5, '')
 ORDER BY "Primary_Key"
 LIMIT 1"#;

const MERGE_ADD_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "Menge" = COALESCE("Menge", 1) + $2,
       "sn_changeDate" = $3,
       "sn_changeTime" = $4,
       "sn_changeUser" = $5
 WHERE "Primary_Key" = $1"#;

const ROW_DELETE_SQL: &str = r#"DELETE FROM "Seriennummern" WHERE "Primary_Key" = $1"#;

/// Moving a row to another warehouse keeps the row and its serial number.
const TRANSFER_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "Lager_Primkey" = $2,
       "Basislager_Primkey" = $2,
       "sn_changeDate" = $3,
       "sn_changeTime" = $4,
       "sn_changeUser" = $5
 WHERE "Artikel_PrimKey" = $1 AND "Primary_Key" = $6"#;

const PLACE_ACTIVE_SQL: &str =
    r#"SELECT "name" FROM "where_place" WHERE "id" = $1 AND COALESCE("active", true)"#;

/// Correction of a counted amount (`Inventur`).
const AMOUNT_CORRECT_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "Menge" = $2,
       "sn_changeDate" = $3,
       "sn_changeTime" = $4,
       "sn_changeUser" = $5
 WHERE "Primary_Key" = $1"#;

/// A device the stocktake did not find.
const DEVICE_RETIRE_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "Ausgesondert" = true,
       "sn_changeDate" = $2,
       "sn_changeTime" = $3,
       "sn_changeUser" = $4
 WHERE "Primary_Key" = $1"#;

/// Consumption out of a batch (`Verbrauchen`).
const CONSUME_SQL: &str = r#"
UPDATE "Seriennummern"
   SET "Menge" = COALESCE("Menge", 1) - $2,
       "sn_changeDate" = $3,
       "sn_changeTime" = $4,
       "sn_changeUser" = $5
 WHERE "Primary_Key" = $1"#;

/// Totals of the active devices of a warehouse — the basis of `"Artikel_Env"`.
/// The usage columns can overlap: a device counts in every usage
/// area it satisfies.
///
/// The withdrawal columns count the rows that stand at a place of the house: the piece
/// is still ours (so it stays in `total`), but it is no longer on the shelf, which is
/// what `Verfügbar` subtracts.
const TOTALS_SQL: &str = r#"
SELECT COALESCE(SUM(COALESCE("Menge", 1)), 0) AS total,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_inventory", false)), 0) AS inventory,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_sale", false)), 0) AS sale,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_rental", false)), 0) AS rental,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE COALESCE("where_used_sale", false)), 0) AS used_sale,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE "where_place_id" IS NOT NULL), 0) AS issued,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE "where_place_id" IS NOT NULL
                                                    AND COALESCE("where_rental", false)), 0) AS issued_rental,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE "where_place_id" IS NOT NULL
                                                    AND COALESCE("where_sale", false)), 0) AS issued_sale,
       COALESCE(SUM(COALESCE("Menge", 1)) FILTER (WHERE "where_place_id" IS NOT NULL
                                                    AND COALESCE("where_used_sale", false)), 0) AS issued_used_sale
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
       "Bestand_EntnahmeGesamt" = $9,
       "Bestand_EntnahmeMiete" = $10,
       "Bestand_EntnahmeNVerkauf" = $11,
       "Bestand_EntnahmeGVerkauf" = $12,
       "lastUpdate_TS" = $8,
       "Artikel_Nr" = (SELECT "ArtikelNr" FROM "Artikel" WHERE "Primary_Key" = $1)
 WHERE "Artikel_Primkey" = $1 AND "Lager_Primkey" = $2"#;

const ENV_INSERT_SQL: &str = r#"
INSERT INTO "Artikel_Env"
       ("Artikel_Primkey", "Lager_Primkey", "Artikel_Nr", "Bestand_Gesamt",
        "Bestand_GesamtMiete", "Bestand_GesamtNVerkauf", "Bestand_GesamtInventar",
        "Bestand_GesamtGVerkauf", "Bestand_Festblockade", "Bestand_EntnahmeGesamt",
        "Bestand_EntnahmeMiete", "Bestand_EntnahmeNVerkauf", "Bestand_EntnahmeGVerkauf",
        "Mandanten_ID", "lastUpdate_TS")
VALUES ($1, $2,
        (SELECT "ArtikelNr" FROM "Artikel" WHERE "Primary_Key" = $1),
        $3, $4, $5, $6, $7, 0, $9, $10, $11, $12, 0, $8)"#;

const ARTICLE_EXISTS_SQL: &str = r#"SELECT 1 FROM "Artikel" WHERE "Primary_Key" = $1"#;
const STOCK_EXISTS_SQL: &str = r#"SELECT 1 FROM "Storage_Lager" WHERE "Primary_Key" = $1"#;
const WAREHOUSES_SQL: &str = r#"
SELECT "Primary_Key" AS id, "Name" AS name, COALESCE("__DefaultLager", false) AS is_default,
       COALESCE("where_zugriff", false) AS zugriff
  FROM "Storage_Lager"
 ORDER BY "__DefaultLager" DESC NULLS LAST, "Name""#;

/// Warehouse of the "Lager" select of the package slip search.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WarehouseRow {
    pub id: i32,
    pub name: Option<String>,
    pub is_default: bool,
    /// The warehouse takes part in the working masks (`"where_zugriff"`).
    pub zugriff: bool,
}

#[derive(Clone)]
pub struct StockRepository {
    pool: PgPool,
}

/// One booking of the stock history, as the repository writes it.
#[derive(Debug, Clone, Default)]
struct Movement<'a> {
    kind: &'a str,
    serial_no: Option<&'a str>,
    quantity: Option<Decimal>,
    warehouse_from: Option<i32>,
    warehouse_to: Option<i32>,
    bin_from: Option<&'a str>,
    bin_to: Option<&'a str>,
    place_from: Option<i32>,
    place_to: Option<i32>,
    place_from_label: Option<&'a str>,
    place_to_label: Option<&'a str>,
    condition_from: Option<&'a str>,
    condition_to: Option<&'a str>,
    discarded_from: Option<bool>,
    discarded_to: Option<bool>,
    note: Option<&'a str>,
}

/// What `DEVICE_DELETE_SQL` returns: warehouse, serial number, quantity, bin, condition
/// and the discard flag of the removed device — the data the history line needs.
type DeletedDevice = (
    Option<i32>,
    Option<String>,
    Option<Decimal>,
    Option<String>,
    Option<String>,
    bool,
);

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

    /// Bin locations of one warehouse as the stock forms offer them.
    pub async fn place_names(&self, warehouse_id: i32) -> Result<Vec<String>, DomainError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"SELECT name FROM (
                   SELECT btrim(COALESCE(NULLIF(btrim("Name"), ''), "Prefix")) AS name
                     FROM "Lagerortung" WHERE "Lager_Primkey" = $1
                   UNION
                   SELECT btrim(COALESCE(NULLIF(btrim("Index"), ''), "Pfad")) AS name
                     FROM "Lagerortung2" WHERE "Lager_Primkey" = $1
               ) x
               WHERE name IS NOT NULL AND name <> ''
               ORDER BY name"#,
        )
        .bind(warehouse_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
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
    ///
    /// `actor` is the account that books; it ends up in the stock history.
    pub async fn goods_receipt(
        &self,
        article_id: i32,
        eingang: &GoodsReceipt,
        actor: &str,
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
        // `Bestandsführung` of the article: a service or a rented-in piece keeps no stock,
        // and an article captured per piece cannot arrive as a batch.
        let (kontrolle, ohne_bestand, einzel): (bool, bool, bool) = sqlx::query_as(
            r#"SELECT COALESCE("Bestandskontrolle", true),
                      COALESCE("OhneEigenbestand", false),
                      COALESCE("SerienNr_Flag", false)
                 FROM "Artikel" WHERE "Primary_Key" = $1"#,
        )
        .bind(article_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?
        .ok_or(DomainError::NotFound {
            entity: "Article",
            id: article_id,
        })?;
        if !kontrolle || ohne_bestand {
            return Err(DomainError::Conflict(
                "Für diesen Artikel wird kein Bestand geführt".into(),
            ));
        }
        if einzel && eingang.without_serial {
            return Err(DomainError::Validation(
                "Dieser Artikel wird einzeln erfasst — Seriennummer angeben".into(),
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

        let order_number = match eingang.order_line {
            Some(line_id) => Some(
                self.validate_order_line(article_id, line_id, quantity)
                    .await?,
            ),
            None => None,
        };
        // The note of the line and of the history names the order when no reason was typed.
        let bemerkung = match (&order_number, eingang.reason.as_deref()) {
            (Some(number), None) => Some(format!("Bestellung {number}")),
            (_, reason) => reason.map(str::to_string),
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
        self.require_target(eingang.stock_id).await?;

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
                .bind(bemerkung.as_deref())
                .bind(today)
                .bind(zeit)
                .bind(&user)
                .bind(eingang.order_line)
                .fetch_one(&mut *tx)
                .await
                .map_err(db_error)?;
            Self::record(
                &mut tx,
                article_id,
                id,
                actor,
                Movement {
                    kind: "receipt",
                    serial_no: nummer.as_deref(),
                    quantity: Some(menge_je_geraet),
                    warehouse_to: Some(eingang.stock_id),
                    bin_to: eingang.bin_location.as_deref(),
                    condition_to: eingang.condition.as_deref(),
                    note: bemerkung.as_deref(),
                    ..Movement::default()
                },
            )
            .await?;
            ids.push(id);
        }

        if eingang.order_line.is_some() {
            sqlx::query(ORDER_SETTLE_SQL)
                .bind(eingang.order_line)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
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
    ///
    /// `actor` is the account that books; it ends up in the stock history.
    pub async fn update_device(
        &self,
        article_id: i32,
        geraet_id: i32,
        update: &DeviceUpdate,
        actor: &str,
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
                // Moving a row to another warehouse is a transfer: the target has to be
                // released and the old one has to allow a withdrawal.
                if before.stock_id != Some(id) {
                    self.require_target(id).await?;
                    if let Some(old) = before.stock_id {
                        self.require_withdrawal(old).await?;
                    }
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
        let kind = Self::movement_kind(
            before.discarded,
            update.discarded,
            before.stock_id != Some(stock_id),
        );
        Self::record(
            &mut tx,
            article_id,
            geraet_id,
            actor,
            Movement {
                kind,
                serial_no: serial_no.as_deref().or(before.serial_no.as_deref()),
                quantity: before.quantity,
                warehouse_from: before.stock_id,
                warehouse_to: Some(stock_id),
                bin_from: before.bin_location.as_deref(),
                bin_to: update.bin_location.as_deref(),
                condition_from: before.condition.as_deref(),
                condition_to: update.condition.as_deref(),
                discarded_from: Some(before.discarded),
                discarded_to: Some(update.discarded),
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;

        self.device(article_id, geraet_id).await
    }

    /// Removes a device for good (form button "−"). To retire it, set
    /// `ausgesondert` instead. The stock history keeps the entry either way.
    ///
    /// `actor` is the account that books; it ends up in the stock history.
    pub async fn delete_device(
        &self,
        article_id: i32,
        geraet_id: i32,
        actor: &str,
    ) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let row: Option<DeletedDevice> = sqlx::query_as(DEVICE_DELETE_SQL)
            .bind(article_id)
            .bind(geraet_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        let (stock_id, serial_no, menge, bin, condition, discarded) =
            row.ok_or(DomainError::NotFound {
                entity: "Device",
                id: geraet_id,
            })?;
        if let Some(id) = stock_id {
            Self::recompute_totals(&mut tx, article_id, id).await?;
        }
        Self::record(
            &mut tx,
            article_id,
            geraet_id,
            actor,
            Movement {
                kind: "delete",
                serial_no: serial_no.as_deref(),
                quantity: menge,
                warehouse_from: stock_id,
                bin_from: bin.as_deref(),
                condition_from: condition.as_deref(),
                discarded_from: Some(discarded),
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Hands material out to a place of the house (`Ausgeben`).
    ///
    /// A device with a serial number keeps its row and gets the place. A batch is split:
    /// the part that stays behind keeps its own line, the part that goes out gets a new
    /// one, and `"Artikel_Env"` counts it as withdrawal — still our stock, but no longer
    /// on the shelf. Quantity articles may go below zero, because consumables are often
    /// handed out before the delivery is booked in; a device with a serial number cannot
    /// be in two places at once.
    ///
    /// `actor` is the account that books; it ends up in the stock history.
    pub async fn issue(
        &self,
        article_id: i32,
        geraet_id: i32,
        ausgabe: &Issue,
        actor: &str,
    ) -> Result<(), DomainError> {
        let before = self.device(article_id, geraet_id).await?;
        let stock_id = Self::warehouse_of(&before)?;
        if before.discarded {
            return Err(DomainError::Validation(
                "Ausgesondertes Material kann nicht ausgegeben werden".into(),
            ));
        }
        if before.is_issued() {
            let ort = before
                .place_name
                .clone()
                .unwrap_or_else(|| "einem Ort".into());
            return Err(DomainError::Conflict(format!(
                "Das Material steht bereits im {ort}"
            )));
        }
        self.require_withdrawal(stock_id).await?;
        let (ort,): (String,) = sqlx::query_as(PLACE_ACTIVE_SQL)
            .bind(ausgabe.place_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Place",
                id: ausgabe.place_id,
            })?;

        let menge = ausgabe.quantity.unwrap_or_else(|| before.amount());
        if menge <= Decimal::ZERO {
            return Err(DomainError::Validation(
                "Quantity must be greater than 0".into(),
            ));
        }
        if !before.without_serial && menge != before.amount() {
            return Err(DomainError::Validation(
                "Ein Gerät mit Seriennummer wird als Ganzes ausgegeben".into(),
            ));
        }
        let heute = Self::today();
        if ausgabe.until.map(|bis| bis < heute).unwrap_or(false) {
            return Err(DomainError::Validation(
                "Das Rückgabedatum liegt vor heute".into(),
            ));
        }

        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let (zeile, serial_no, anzahl) =
            Self::take_apart(&mut tx, article_id, &before, menge, heute, zeit, &user).await?;
        sqlx::query(ISSUE_SQL)
            .bind(zeile)
            .bind(ausgabe.place_id)
            .bind(heute)
            .bind(ausgabe.until)
            .bind(heute)
            .bind(zeit)
            .bind(&user)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        Self::recompute_totals(&mut tx, article_id, stock_id).await?;
        Self::record(
            &mut tx,
            article_id,
            zeile,
            actor,
            Movement {
                kind: "issue",
                serial_no: serial_no.as_deref(),
                quantity: Some(anzahl),
                warehouse_from: Some(stock_id),
                place_to: Some(ausgabe.place_id),
                place_to_label: Some(&ort),
                note: ausgabe.note.as_deref(),
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Takes issued material back into the warehouse (`Zurücknehmen`).
    /// A part that was split off a batch goes back into the line it came from, so the
    /// grid keeps one row per batch.
    ///
    /// `actor` is the account that books; it ends up in the stock history.
    pub async fn give_back(
        &self,
        article_id: i32,
        geraet_id: i32,
        condition: Option<&str>,
        note: Option<&str>,
        actor: &str,
    ) -> Result<(), DomainError> {
        let before = self.device(article_id, geraet_id).await?;
        let stock_id = Self::warehouse_of(&before)?;
        let place_id = before.place_id.ok_or_else(|| {
            DomainError::Validation("Das Material steht nicht an einem Ort".into())
        })?;
        // The material returns into its warehouse — that one has to be released.
        self.require_target(stock_id).await?;
        // A rated condition comes back with the piece and changes its row.
        let neue_zustand = condition
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string);

        let heute = Self::today();
        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;

        // A batch may only be merged into a row of the same condition; a rated return
        // keeps its own row so the rating is not mixed into the others.
        let merge = match (before.serial_no.as_deref().unwrap_or(""), &neue_zustand) {
            ("", None) => sqlx::query_as::<_, (i32,)>(MERGE_TARGET_SQL)
                .bind(article_id)
                .bind(stock_id)
                .bind(geraet_id)
                .bind(before.bin_location.as_deref())
                .bind(before.condition.as_deref())
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_error)?,
            _ => None,
        };

        match merge {
            Some((ziel,)) => {
                sqlx::query(MERGE_ADD_SQL)
                    .bind(ziel)
                    .bind(before.amount())
                    .bind(heute)
                    .bind(zeit)
                    .bind(&user)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
                sqlx::query(ROW_DELETE_SQL)
                    .bind(geraet_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
            None => {
                sqlx::query(RETURN_SQL)
                    .bind(geraet_id)
                    .bind(heute)
                    .bind(zeit)
                    .bind(&user)
                    .bind(neue_zustand.as_deref())
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
        }
        Self::recompute_totals(&mut tx, article_id, stock_id).await?;
        Self::record(
            &mut tx,
            article_id,
            geraet_id,
            actor,
            Movement {
                kind: "return",
                serial_no: before.serial_no.as_deref(),
                quantity: Some(before.amount()),
                warehouse_to: Some(stock_id),
                bin_to: before.bin_location.as_deref(),
                condition_from: before.condition.as_deref(),
                condition_to: neue_zustand.as_deref().or(before.condition.as_deref()),
                place_from: Some(place_id),
                place_from_label: before.place_name.as_deref(),
                note,
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Moves material to another warehouse of the house (`Umlegen`).
    ///
    /// The row keeps its serial number, condition and usage areas; only the warehouse
    /// changes. A partial amount splits the batch — what is moved is physically there,
    /// so more than the row carries is refused.
    ///
    /// `actor` is the account that books; it ends up in the stock history.
    pub async fn transfer(
        &self,
        article_id: i32,
        geraet_id: i32,
        to_stock_id: i32,
        quantity: Option<Decimal>,
        note: Option<&str>,
        actor: &str,
    ) -> Result<(), DomainError> {
        let before = self.device(article_id, geraet_id).await?;
        let from_stock = Self::warehouse_of(&before)?;
        Self::validate_stock_id(to_stock_id)?;
        if to_stock_id == from_stock {
            return Err(DomainError::Validation(
                "Das Material liegt schon in diesem Lager".into(),
            ));
        }
        let exists: Option<(i32,)> = sqlx::query_as(STOCK_EXISTS_SQL)
            .bind(to_stock_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if exists.is_none() {
            return Err(DomainError::NotFound {
                entity: "Stock",
                id: to_stock_id,
            });
        }
        self.require_target(to_stock_id).await?;
        self.require_withdrawal(from_stock).await?;
        if before.discarded {
            return Err(DomainError::Validation(
                "Ausgesondertes Material wird nicht umgelegt".into(),
            ));
        }
        if before.is_issued() {
            return Err(DomainError::Conflict(
                "Ausgegebenes Material zuerst zurücknehmen".into(),
            ));
        }

        let menge = quantity.unwrap_or_else(|| before.amount());
        if menge <= Decimal::ZERO {
            return Err(DomainError::Validation(
                "Quantity must be greater than 0".into(),
            ));
        }
        if menge > before.amount() {
            return Err(DomainError::Validation(format!(
                "Es sind nur {} vorhanden",
                before.amount().normalize()
            )));
        }
        if !before.without_serial && menge != before.amount() {
            return Err(DomainError::Validation(
                "Ein Gerät mit Seriennummer wird als Ganzes umgelegt".into(),
            ));
        }

        let heute = Self::today();
        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let (zeile, serial_no, anzahl) =
            Self::take_apart(&mut tx, article_id, &before, menge, heute, zeit, &user).await?;
        sqlx::query(TRANSFER_SQL)
            .bind(article_id)
            .bind(to_stock_id)
            .bind(heute)
            .bind(zeit)
            .bind(&user)
            .bind(zeile)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        Self::recompute_totals(&mut tx, article_id, from_stock).await?;
        Self::recompute_totals(&mut tx, article_id, to_stock_id).await?;
        Self::record(
            &mut tx,
            article_id,
            zeile,
            actor,
            Movement {
                kind: "move",
                serial_no: serial_no.as_deref(),
                quantity: Some(anzahl),
                warehouse_from: Some(from_stock),
                warehouse_to: Some(to_stock_id),
                bin_from: before.bin_location.as_deref(),
                condition_from: before.condition.as_deref(),
                note,
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Moves many devices of one article into another warehouse (`Sammel-Umlagerung`).
    ///
    /// Each row is its own booking, so a row that cannot move (issued material, a closed
    /// warehouse, a partial amount on a serial number) does not stop the others. The
    /// answer counts what moved and names what did not.
    pub async fn transfer_many(
        &self,
        article_id: i32,
        device_ids: &[i32],
        to_stock_id: i32,
        note: Option<&str>,
        actor: &str,
    ) -> Result<(i64, Vec<(i32, String)>), DomainError> {
        if device_ids.is_empty() {
            return Err(DomainError::Validation(
                "Es ist kein Gerät ausgewählt".into(),
            ));
        }
        if device_ids.len() > 500 {
            return Err(DomainError::Validation(
                "Mehr als 500 Geräte auf einmal werden nicht umgelegt".into(),
            ));
        }
        let mut moved = 0;
        let mut failed = Vec::new();
        for id in device_ids {
            match self
                .transfer(article_id, *id, to_stock_id, None, note, actor)
                .await
            {
                Ok(()) => moved += 1,
                Err(DomainError::NotFound { .. }) => failed.push((*id, "nicht gefunden".into())),
                Err(DomainError::Validation(msg)) | Err(DomainError::Conflict(msg)) => {
                    failed.push((*id, msg));
                }
                Err(other) => return Err(other),
            }
        }
        Ok((moved, failed))
    }

    /// Takes `menge` off a row: the row itself when the whole amount is meant, otherwise
    /// a split-off row. Returns the row that now carries the amount, its serial number
    /// (the batch number stays at the row it came from) and the amount.
    ///
    /// A quantity article may be taken out beyond what the row carries — the row that
    /// stays behind then goes negative, which is exactly what "consumables are handed
    /// out before the delivery is booked in" looks like. A device with a serial number
    /// never reaches that branch: its amount is 1.
    #[allow(clippy::too_many_arguments)]
    async fn take_apart(
        conn: &mut PgConnection,
        article_id: i32,
        row: &DeviceRow,
        menge: Decimal,
        date: NaiveDate,
        zeit: NaiveTime,
        user: &str,
    ) -> Result<(i32, Option<String>, Decimal), DomainError> {
        if menge == row.amount() {
            return Ok((row.id, row.serial_no.clone(), row.amount()));
        }
        let (neu,): (i32,) = sqlx::query_as(SPLIT_SQL)
            .bind(article_id)
            .bind(row.id)
            .bind(menge)
            .bind(date)
            .bind(zeit)
            .bind(user)
            .fetch_one(&mut *conn)
            .await
            .map_err(db_error)?;
        sqlx::query(SPLIT_SOURCE_SQL)
            .bind(article_id)
            .bind(menge)
            .bind(date)
            .bind(zeit)
            .bind(user)
            .bind(row.id)
            .execute(&mut *conn)
            .await
            .map_err(db_error)?;
        Ok((neu, None, menge))
    }

    /// Warehouse of a row — every booking needs one.
    fn warehouse_of(row: &DeviceRow) -> Result<i32, DomainError> {
        row.stock_id.ok_or_else(|| {
            DomainError::Validation("Device has no warehouse — a warehouse is required".into())
        })
    }

    /// Corrects the amount of a batch to what was counted (`Inventur`).
    ///
    /// The difference `counted − current` goes into the history as kind `count`; afterwards
    /// `"Artikel_Env"` is recomputed, so `Gesamt` and `Verfügbar` follow. Comparing against
    /// the **current** amount (not the one the stocktake started with) makes a repeated
    /// `Abschließen` harmless: what is already right books a difference of zero.
    pub(crate) async fn correct_amount(
        &self,
        article_id: i32,
        device_id: i32,
        counted: Decimal,
        note: Option<&str>,
        actor: &str,
    ) -> Result<(), DomainError> {
        if counted < Decimal::ZERO {
            return Err(DomainError::Validation(
                "Die gezählte Menge darf nicht negativ sein".into(),
            ));
        }
        let before = self.device(article_id, device_id).await?;
        let stock_id = Self::warehouse_of(&before)?;
        let difference = counted - before.amount();
        if difference.is_zero() {
            return Ok(());
        }

        let heute = Self::today();
        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(AMOUNT_CORRECT_SQL)
            .bind(device_id)
            .bind(counted)
            .bind(heute)
            .bind(zeit)
            .bind(&user)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        Self::recompute_totals(&mut tx, article_id, stock_id).await?;
        Self::record(
            &mut tx,
            article_id,
            device_id,
            actor,
            Movement {
                kind: "count",
                serial_no: before.serial_no.as_deref(),
                quantity: Some(difference),
                warehouse_to: Some(stock_id),
                bin_to: before.bin_location.as_deref(),
                note,
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Retires a device that the stocktake did not find (`Inventur`: counted 0).
    ///
    /// A piece with a serial number cannot be "0 of it" — either it is there or it is gone.
    /// It leaves the stock like every discarded device (out of the totals, out of every
    /// list, restorable) instead of being deleted.
    pub(crate) async fn retire_missing(
        &self,
        article_id: i32,
        device_id: i32,
        note: Option<&str>,
        actor: &str,
    ) -> Result<(), DomainError> {
        let before = self.device(article_id, device_id).await?;
        if before.discarded {
            return Ok(());
        }
        let stock_id = Self::warehouse_of(&before)?;
        let heute = Self::today();
        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(DEVICE_RETIRE_SQL)
            .bind(device_id)
            .bind(heute)
            .bind(zeit)
            .bind(&user)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        Self::recompute_totals(&mut tx, article_id, stock_id).await?;
        Self::record(
            &mut tx,
            article_id,
            device_id,
            actor,
            Movement {
                kind: "retire",
                serial_no: before.serial_no.as_deref(),
                quantity: before.quantity,
                warehouse_from: Some(stock_id),
                bin_from: before.bin_location.as_deref(),
                condition_from: before.condition.as_deref(),
                discarded_from: Some(false),
                discarded_to: Some(true),
                note,
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Consumes material out of a batch (`Verbrauchen`) — the piece is used up, it goes
    /// nowhere and comes back never. Only a batch without serial numbers can be consumed;
    /// a single device leaves the stock by being retired.
    ///
    /// `actor` is the account that books; it ends up in the stock history.
    pub async fn consume(
        &self,
        article_id: i32,
        device_id: i32,
        quantity: Decimal,
        note: Option<&str>,
        actor: &str,
    ) -> Result<(), DomainError> {
        let before = self.device(article_id, device_id).await?;
        let stock_id = Self::warehouse_of(&before)?;
        if !before.without_serial {
            return Err(DomainError::Validation(
                "Nur Material ohne Seriennummer wird verbraucht — ein Einzelgerät wird ausgesondert"
                    .into(),
            ));
        }
        if before.is_issued() {
            return Err(DomainError::Conflict(
                "Ausgegebenes Material zuerst zurücknehmen".into(),
            ));
        }
        self.require_withdrawal(stock_id).await?;
        if quantity <= Decimal::ZERO {
            return Err(DomainError::Validation(
                "Die Menge muss größer als 0 sein".into(),
            ));
        }
        if quantity > before.amount() {
            return Err(DomainError::Validation(format!(
                "Es sind nur {} vorhanden",
                before.amount().normalize()
            )));
        }

        let heute = Self::today();
        let (zeit, user) = Self::now();
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(CONSUME_SQL)
            .bind(device_id)
            .bind(quantity)
            .bind(heute)
            .bind(zeit)
            .bind(&user)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        Self::recompute_totals(&mut tx, article_id, stock_id).await?;
        Self::record(
            &mut tx,
            article_id,
            device_id,
            actor,
            Movement {
                kind: "consume",
                serial_no: before.serial_no.as_deref(),
                quantity: Some(quantity),
                warehouse_from: Some(stock_id),
                bin_from: before.bin_location.as_deref(),
                condition_from: before.condition.as_deref(),
                note,
                ..Movement::default()
            },
        )
        .await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Reorder level (`"VerkaufMindBestand"`) of an article.
    ///
    /// It belongs to the stock, not to the prices: the reorder suggestion of `WHERE - is my
    /// stuff?` reads it, so it must be settable without the rental/sale module.
    pub async fn reorder_level(&self, article_id: i32) -> Result<Option<Decimal>, DomainError> {
        self.validate_article(article_id).await?;
        let row: Option<(Option<Decimal>,)> = sqlx::query_as(
            r#"SELECT "VerkaufMindBestand" FROM "Artikel" WHERE "Primary_Key" = $1"#,
        )
        .bind(article_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        Ok(row
            .and_then(|(level,)| level)
            .filter(|d| *d > Decimal::ZERO))
    }

    pub async fn set_reorder_level(
        &self,
        article_id: i32,
        level: Option<Decimal>,
    ) -> Result<(), DomainError> {
        self.validate_article(article_id).await?;
        if level.map(|d| d < Decimal::ZERO).unwrap_or(false) {
            return Err(DomainError::Validation(
                "Der Meldebestand darf nicht negativ sein".into(),
            ));
        }
        sqlx::query(r#"UPDATE "Artikel" SET "VerkaufMindBestand" = $2 WHERE "Primary_Key" = $1"#)
            .bind(article_id)
            .bind(level)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    /// Stock history of an article — optionally only the lines of one device.
    /// Newest first, at most `limit` lines.
    pub async fn movements(
        &self,
        article_id: i32,
        device_id: Option<i32>,
        limit: i64,
    ) -> Result<Vec<MovementRow>, DomainError> {
        self.validate_article(article_id).await?;
        sqlx::query_as::<_, MovementRow>(MOVEMENTS_SQL)
            .bind(article_id)
            .bind(device_id)
            .bind(limit.clamp(1, 1000))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Which line the history gets for a change of device data: discarding and taking
    /// that back come first, then a move, everything else is a plain change.
    fn movement_kind(discarded_before: bool, discarded_after: bool, moved: bool) -> &'static str {
        if discarded_before != discarded_after {
            if discarded_after {
                "retire"
            } else {
                "reinstate"
            }
        } else if moved {
            "move"
        } else {
            "change"
        }
    }

    /// Writes one line of the stock history inside the given transaction.
    async fn record(
        conn: &mut PgConnection,
        article_id: i32,
        device_id: i32,
        actor: &str,
        entry: Movement<'_>,
    ) -> Result<(), DomainError> {
        let by = if actor.trim().is_empty() {
            "where"
        } else {
            actor.trim()
        };
        sqlx::query(MOVEMENT_INSERT_SQL)
            .bind(article_id)
            .bind(device_id)
            .bind(entry.serial_no)
            .bind(entry.kind)
            .bind(entry.quantity)
            .bind(entry.warehouse_from)
            .bind(entry.warehouse_to)
            .bind(entry.bin_from)
            .bind(entry.bin_to)
            .bind(entry.condition_from)
            .bind(entry.condition_to)
            .bind(entry.discarded_from)
            .bind(entry.discarded_to)
            .bind(entry.place_from)
            .bind(entry.place_to)
            .bind(entry.place_from_label)
            .bind(entry.place_to_label)
            .bind(entry.note)
            .bind(by)
            .execute(&mut *conn)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    /// Recomputes `"Artikel_Env"` from the devices of the warehouse.
    async fn recompute_totals(
        conn: &mut PgConnection,
        article_id: i32,
        stock_id: i32,
    ) -> Result<(), DomainError> {
        let (
            total,
            inventory,
            sale,
            rental,
            used_sale,
            issued,
            issued_rental,
            issued_sale,
            issued_used_sale,
        ): (
            Decimal,
            Decimal,
            Decimal,
            Decimal,
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
            .bind(issued)
            .bind(issued_rental)
            .bind(issued_sale)
            .bind(issued_used_sale)
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
                .bind(issued)
                .bind(issued_rental)
                .bind(issued_sale)
                .bind(issued_used_sale)
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

    /// The two switches of a warehouse: `("where_zugriff", "where_entnahme")`.
    async fn warehouse_flags(&self, stock_id: i32) -> Result<(bool, bool), DomainError> {
        let row: Option<(bool, bool)> = sqlx::query_as(
            r#"SELECT COALESCE("where_zugriff", false), COALESCE("where_entnahme", false)
                 FROM "Storage_Lager" WHERE "Primary_Key" = $1"#,
        )
        .bind(stock_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        row.ok_or(DomainError::NotFound {
            entity: "Stock",
            id: stock_id,
        })
    }

    /// A booking into this warehouse needs `Zugriff` — otherwise the warehouse is out of
    /// the working masks.
    async fn require_target(&self, stock_id: i32) -> Result<(), DomainError> {
        let (zugriff, _) = self.warehouse_flags(stock_id).await?;
        if !zugriff {
            return Err(DomainError::Conflict(
                "Dieses Lager ist für Buchungen gesperrt".into(),
            ));
        }
        Ok(())
    }

    /// Taking material out of this warehouse needs `Entnahme` — a display or consignment
    /// shelf keeps its stock.
    async fn require_withdrawal(&self, stock_id: i32) -> Result<(), DomainError> {
        let (_, entnahme) = self.warehouse_flags(stock_id).await?;
        if !entnahme {
            return Err(DomainError::Conflict(
                "Aus diesem Lager wird nicht entnommen".into(),
            ));
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

    /// Judges the purchase-order line a delivery claims to belong to and returns its
    /// order number.
    ///
    /// The line has to belong to **this** article, its order has to be open, and the
    /// delivery may not exceed what is still expected — more would turn the line into an
    /// over-delivery, which the buyer has to decide on.
    async fn validate_order_line(
        &self,
        article_id: i32,
        line_id: i32,
        quantity: Decimal,
    ) -> Result<String, DomainError> {
        let row: Option<(i32, Decimal, String, String, Decimal)> = sqlx::query_as(ORDER_LINE_SQL)
            .bind(line_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        let (line_article, ordered, state, number, received) =
            row.ok_or(DomainError::NotFound {
                entity: "PurchaseOrderLine",
                id: line_id,
            })?;
        if line_article != article_id {
            return Err(DomainError::Validation(
                "Diese Bestellposition gehört zu einem anderen Artikel".into(),
            ));
        }
        if state != "open" {
            return Err(DomainError::Conflict(format!(
                "Die Bestellung {number} ist abgeschlossen"
            )));
        }
        let offen = (ordered - received).max(Decimal::ZERO);
        if quantity > offen {
            return Err(DomainError::Validation(format!(
                "Auf der Bestellung {number} sind nur noch {} offen",
                offen.normalize()
            )));
        }
        Ok(number)
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
            zugriff: true,
            entnahme: true,
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
    fn movement_kind_follows_the_booking() {
        // Discarding wins over a move, taking it back too; an unchanged device is a change.
        assert_eq!(StockRepository::movement_kind(false, true, true), "retire");
        assert_eq!(StockRepository::movement_kind(true, true, true), "move");
        assert_eq!(
            StockRepository::movement_kind(true, false, false),
            "reinstate"
        );
        assert_eq!(StockRepository::movement_kind(false, false, true), "move");
        assert_eq!(
            StockRepository::movement_kind(false, false, false),
            "change"
        );
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
            place_id: None,
            place_name: None,
            issued_from: None,
            issued_until: None,
        };
        assert_eq!(
            device.usage_text(),
            "Inventar, Miete, Gebrauchtverkauf".to_string()
        );
    }

    #[test]
    fn status_names_the_place_while_issued() {
        let mut device = DeviceRow {
            id: 1,
            serial_no: Some("0815".into()),
            inventory_no: None,
            inventory_no2: None,
            quantity: d("1"),
            inventory: true,
            rental: false,
            sale: false,
            used_sale: false,
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
            place_id: None,
            place_name: None,
            issued_from: None,
            issued_until: None,
        };
        assert_eq!(device.status(), "im Lager");
        assert!(!device.is_issued());
        device.place_id = Some(4);
        device.place_name = Some("Foyer".into());
        assert_eq!(device.status(), "ausgegeben: Foyer");
        assert!(device.is_issued());
        device.place_id = None;
        device.discarded = true;
        assert_eq!(device.status(), "ausgesondert");
    }
}
