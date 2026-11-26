//! Purchase orders (`Bestellungen`): what was ordered for the shelf, and how much of it
//! has arrived.
//!
//! A purchase order is a **warehouse document**, not an order of the rental module: it
//! names a supplier, a date and lines of articles with a quantity. Nothing is reserved and
//! nothing is booked until the material arrives — the goods receipt of the article names
//! the line it belongs to.
//!
//! The received amount of a line is **derived**: `SUM("Menge")` over the devices that
//! carry `"Seriennummern"."where_order_line"`. That way it cannot drift away from the
//! stock, a correction of the stock corrects the order as well, and a partial delivery is
//! simply a smaller sum. The order itself only keeps a **state**: `open` while something
//! may still arrive, `done` when the rest is not coming, `cancelled` when nothing arrived
//! at all.

use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;
use crate::number_series_repo::reserve_on;

/// Number series of the purchase order number (`where_number_series`).
pub const NUMBER_SERIES: &str = "purchase_order";

/// One purchase order with the counters the list shows.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PurchaseOrderRow {
    pub id: i32,
    pub number: String,
    pub supplier_no: Option<i32>,
    pub supplier_name: Option<String>,
    pub ordered_at: NaiveDate,
    pub expected_at: Option<NaiveDate>,
    pub note: Option<String>,
    /// `open`, `done` or `cancelled`.
    pub state: String,
    pub created_at: chrono::DateTime<Utc>,
    pub created_by: String,
    pub lines: i64,
    /// Lines that are not delivered completely.
    pub open_lines: i64,
    pub ordered: Decimal,
    pub received: Decimal,
}

impl PurchaseOrderRow {
    /// How much is still to come over all lines.
    pub fn missing(&self) -> Decimal {
        (self.ordered - self.received).max(Decimal::ZERO)
    }

    pub fn is_open(&self) -> bool {
        self.state == "open"
    }
}

/// One line of an order: what was ordered, what arrived, what is missing.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PurchaseOrderLineRow {
    pub id: i32,
    pub order_id: i32,
    pub line_no: i32,
    pub article_id: i32,
    pub article_no: Option<i32>,
    pub article_name: Option<String>,
    pub quantity: Decimal,
    pub received: Decimal,
    /// `quantity − received`, never negative.
    pub open: Decimal,
    pub unit_price: Option<Decimal>,
    pub note: Option<String>,
}

/// A line of an order that still expects material — what the goods receipt offers.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OpenOrderLineRow {
    pub line_id: i32,
    pub order_id: i32,
    pub number: String,
    pub supplier_name: Option<String>,
    pub expected_at: Option<NaiveDate>,
    pub line_no: i32,
    pub article_id: i32,
    pub quantity: Decimal,
    pub received: Decimal,
    pub open: Decimal,
}

/// What was ordered on one line.
#[derive(Debug, Clone)]
pub struct NewOrderLine {
    pub article_id: i32,
    pub quantity: Decimal,
    pub unit_price: Option<Decimal>,
    pub note: Option<String>,
}

/// What may change on an open order: supplier, delivery date and note.
#[derive(Debug, Clone, Default)]
pub struct OrderHeaderUpdate {
    pub supplier_no: Option<i32>,
    pub expected_at: Option<NaiveDate>,
    pub note: Option<String>,
}

/// Header and lines of a new order.
#[derive(Debug, Clone)]
pub struct NewOrder {
    pub supplier_no: Option<i32>,
    pub ordered_at: Option<NaiveDate>,
    pub expected_at: Option<NaiveDate>,
    pub note: Option<String>,
    pub lines: Vec<NewOrderLine>,
}

/// Received amount of a line, computed the same way everywhere.
const RECEIVED: &str = r#"
COALESCE((SELECT SUM(COALESCE(g."Menge", 0))
            FROM "Seriennummern" g WHERE g."where_order_line" = l."id"), 0)"#;

fn line_sql() -> String {
    format!(
        r#"
SELECT l."id", l."order_id", l."line_no", l."article_id", l."article_no", l."article_name",
       l."quantity", l."unit_price", l."note",
       {received} AS received,
       GREATEST(l."quantity" - {received}, 0) AS open
  FROM "where_purchase_order_line" l"#,
        received = RECEIVED
    )
}

fn header_sql() -> String {
    format!(
        r#"
SELECT o."id", o."number", o."supplier_no", o."supplier_name", o."ordered_at",
       o."expected_at", o."note", o."state", o."created_at", o."created_by",
       (SELECT count(*) FROM "where_purchase_order_line" x WHERE x."order_id" = o."id") AS lines,
       (SELECT count(*) FROM "where_purchase_order_line" l
         WHERE l."order_id" = o."id" AND {received} < l."quantity") AS open_lines,
       (SELECT COALESCE(SUM(l."quantity"), 0) FROM "where_purchase_order_line" l
         WHERE l."order_id" = o."id") AS ordered,
       (SELECT COALESCE(SUM({received}), 0) FROM "where_purchase_order_line" l
         WHERE l."order_id" = o."id") AS received
  FROM "where_purchase_order" o"#,
        received = RECEIVED
    )
}

const INSERT_SQL: &str = r#"
INSERT INTO "where_purchase_order"
       ("number", "supplier_no", "supplier_name", "ordered_at", "expected_at", "note",
        "created_by")
VALUES ($1, $2, $3, COALESCE($4, CURRENT_DATE), $5, $6, $7)
RETURNING "id""#;

const LINE_INSERT_SQL: &str = r#"
INSERT INTO "where_purchase_order_line"
       ("order_id", "line_no", "article_id", "article_no", "article_name", "quantity",
        "unit_price", "note")
SELECT $1, $2, a."Primary_Key", a."ArtikelNr", a."Name", $3, $4, $5
  FROM "Artikel" a
 WHERE a."Primary_Key" = $6
RETURNING "id""#;

/// Received amounts of the lines of one order, per line id.
const RECEIVED_OF_LINE_SQL: &str = r#"
SELECT COALESCE(SUM(COALESCE(g."Menge", 0)), 0)
  FROM "Seriennummern" g
 WHERE g."where_order_line" = $1"#;

fn open_lines_sql() -> String {
    format!(
        r#"
SELECT l."id" AS line_id, l."order_id", o."number", o."supplier_name", o."expected_at",
       l."line_no", l."article_id", l."quantity",
       {received} AS received,
       GREATEST(l."quantity" - {received}, 0) AS open
  FROM "where_purchase_order_line" l
  JOIN "where_purchase_order" o ON o."id" = l."order_id"
 WHERE l."article_id" = $1
   AND o."state" = 'open'
   AND {received} < l."quantity"
 ORDER BY o."ordered_at", o."id", l."line_no""#,
        received = RECEIVED
    )
}

#[derive(Clone)]
pub struct PurchaseOrderRepository {
    pool: PgPool,
}

impl PurchaseOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Orders newest first; `state` filters `open`, `done` or `cancelled`.
    pub async fn list(&self, state: Option<&str>) -> Result<Vec<PurchaseOrderRow>, DomainError> {
        let sql = format!(
            "{} WHERE ($1::text IS NULL OR o.\"state\" = $1)\n ORDER BY o.\"ordered_at\" DESC, o.\"id\" DESC",
            header_sql()
        );
        sqlx::query_as::<_, PurchaseOrderRow>(sqlx::AssertSqlSafe(sql))
            .bind(state)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn get(&self, id: i32) -> Result<PurchaseOrderRow, DomainError> {
        let sql = format!("{} WHERE o.\"id\" = $1", header_sql());
        sqlx::query_as::<_, PurchaseOrderRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "PurchaseOrder",
                id,
            })
    }

    pub async fn lines(&self, id: i32) -> Result<Vec<PurchaseOrderLineRow>, DomainError> {
        let sql = format!(
            "{} WHERE l.\"order_id\" = $1 ORDER BY l.\"line_no\"",
            line_sql()
        );
        sqlx::query_as::<_, PurchaseOrderLineRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Open lines of one article — the offer of the goods receipt.
    pub async fn open_lines_of_article(
        &self,
        article_id: i32,
    ) -> Result<Vec<OpenOrderLineRow>, DomainError> {
        sqlx::query_as::<_, OpenOrderLineRow>(sqlx::AssertSqlSafe(open_lines_sql()))
            .bind(article_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// How much of one line is still expected.
    pub async fn open_of_line(&self, line_id: i32) -> Result<Decimal, DomainError> {
        let (received,): (Decimal,) = sqlx::query_as(RECEIVED_OF_LINE_SQL)
            .bind(line_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        let (quantity,): (Decimal,) =
            sqlx::query_as(r#"SELECT "quantity" FROM "where_purchase_order_line" WHERE "id" = $1"#)
                .bind(line_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_error)?
                .ok_or(DomainError::NotFound {
                    entity: "PurchaseOrderLine",
                    id: line_id,
                })?;
        Ok((quantity - received).max(Decimal::ZERO))
    }

    /// Creates the order with its lines; the number comes from the series `purchase_order`.
    pub async fn create(
        &self,
        order: &NewOrder,
        actor: &str,
    ) -> Result<PurchaseOrderRow, DomainError> {
        if order.lines.is_empty() {
            return Err(DomainError::Validation(
                "Eine Bestellung braucht mindestens eine Position".into(),
            ));
        }
        for line in &order.lines {
            if line.quantity <= Decimal::ZERO {
                return Err(DomainError::Validation(
                    "Die Menge muss größer als 0 sein".into(),
                ));
            }
            if line.unit_price.map(|p| p < Decimal::ZERO).unwrap_or(false) {
                return Err(DomainError::Validation(
                    "Der Preis darf nicht negativ sein".into(),
                ));
            }
        }
        // Without an order date the database takes today (`COALESCE($4, CURRENT_DATE)`), so
        // the delivery date is compared against that as well.
        let ordered = order
            .ordered_at
            .unwrap_or_else(|| chrono::Utc::now().date_naive());
        if let Some(to) = order.expected_at
            && to < ordered
        {
            return Err(DomainError::Validation(
                "Der Liefertermin liegt vor dem Bestelldatum".into(),
            ));
        }

        let supplier_name = match order.supplier_no {
            Some(no) => {
                let name: Option<(Option<String>,)> = sqlx::query_as(
                    r#"SELECT COALESCE("Name1", "Name2", "Name3") FROM "Adresse"
                        WHERE "Primary_key" = $1"#,
                )
                .bind(no)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_error)?;
                match name {
                    Some((name,)) => name,
                    None => {
                        return Err(DomainError::NotFound {
                            entity: "Address",
                            id: no,
                        });
                    }
                }
            }
            None => None,
        };

        let by = if actor.trim().is_empty() {
            "where"
        } else {
            actor.trim()
        };
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let number = reserve_on(&mut tx, NUMBER_SERIES, Utc::now().year())
            .await
            .map_err(db_error)?
            .map(|raw| {
                crate::number_series_repo::format(
                    &raw.prefix,
                    &raw.format,
                    raw.digits,
                    raw.value,
                    Utc::now().date_naive(),
                )
            })
            .ok_or_else(|| {
                DomainError::Conflict(
                    "Der Nummernkreis „Bestellnummer“ fehlt oder ist inaktiv".into(),
                )
            })?;
        let (id,): (i32,) = sqlx::query_as(INSERT_SQL)
            .bind(&number)
            .bind(order.supplier_no)
            .bind(supplier_name.as_deref())
            .bind(order.ordered_at)
            .bind(order.expected_at)
            .bind(order.note.as_deref())
            .bind(by)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;
        for (index, line) in order.lines.iter().enumerate() {
            let inserted: Option<(i32,)> = sqlx::query_as(LINE_INSERT_SQL)
                .bind(id)
                .bind(index as i32 + 1)
                .bind(line.quantity)
                .bind(line.unit_price)
                .bind(line.note.as_deref())
                .bind(line.article_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_error)?;
            if inserted.is_none() {
                return Err(DomainError::NotFound {
                    entity: "Article",
                    id: line.article_id,
                });
            }
        }
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    /// Changes supplier, delivery date or note of an open order.
    pub async fn update_header(
        &self,
        order_id: i32,
        update: &OrderHeaderUpdate,
    ) -> Result<PurchaseOrderRow, DomainError> {
        let order = self.get(order_id).await?;
        if !order.is_open() {
            return Err(DomainError::Conflict(
                "Diese Bestellung ist abgeschlossen".into(),
            ));
        }
        if update
            .expected_at
            .map(|to| to < order.ordered_at)
            .unwrap_or(false)
        {
            return Err(DomainError::Validation(
                "Der Liefertermin liegt vor dem Bestelldatum".into(),
            ));
        }
        let supplier_name = match update.supplier_no {
            Some(no) => {
                let name: Option<(Option<String>,)> = sqlx::query_as(
                    r#"SELECT COALESCE("Name1", "Name2", "Name3") FROM "Adresse"
                        WHERE "Primary_key" = $1"#,
                )
                .bind(no)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_error)?;
                match name {
                    Some((name,)) => name,
                    None => {
                        return Err(DomainError::NotFound {
                            entity: "Address",
                            id: no,
                        });
                    }
                }
            }
            None => None,
        };
        sqlx::query(
            r#"UPDATE "where_purchase_order"
                  SET "supplier_no" = $2, "supplier_name" = $3, "expected_at" = $4,
                      "note" = $5, "changed_at" = now()
                WHERE "id" = $1"#,
        )
        .bind(order_id)
        .bind(update.supplier_no)
        .bind(supplier_name.as_deref())
        .bind(update.expected_at)
        .bind(update.note.as_deref())
        .execute(&self.pool)
        .await
        .map_err(db_error)?;
        self.get(order_id).await
    }

    /// Adds a line to an open order.
    pub async fn add_line(
        &self,
        order_id: i32,
        article_id: i32,
        quantity: Decimal,
        unit_price: Option<Decimal>,
        note: Option<&str>,
    ) -> Result<PurchaseOrderRow, DomainError> {
        if quantity <= Decimal::ZERO {
            return Err(DomainError::Validation(
                "Die Menge muss größer als 0 sein".into(),
            ));
        }
        let order = self.get(order_id).await?;
        if !order.is_open() {
            return Err(DomainError::Conflict(
                "Diese Bestellung ist abgeschlossen".into(),
            ));
        }
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let (next,): (i32,) = sqlx::query_as(
            r#"SELECT COALESCE(MAX("line_no"), 0) + 1 FROM "where_purchase_order_line"
                WHERE "order_id" = $1"#,
        )
        .bind(order_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        let inserted: Option<(i32,)> = sqlx::query_as(LINE_INSERT_SQL)
            .bind(order_id)
            .bind(next)
            .bind(quantity)
            .bind(unit_price)
            .bind(note)
            .bind(article_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?;
        if inserted.is_none() {
            return Err(DomainError::NotFound {
                entity: "Article",
                id: article_id,
            });
        }
        sqlx::query(r#"UPDATE "where_purchase_order" SET "changed_at" = now() WHERE "id" = $1"#)
            .bind(order_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        self.get(order_id).await
    }

    /// Changes amount, price or note of a line.
    ///
    /// The amount may not fall below what has already arrived — that would turn a
    /// delivered line into an over-delivery.
    pub async fn update_line(
        &self,
        order_id: i32,
        line_id: i32,
        quantity: Decimal,
        unit_price: Option<Decimal>,
        note: Option<&str>,
    ) -> Result<PurchaseOrderRow, DomainError> {
        if quantity <= Decimal::ZERO {
            return Err(DomainError::Validation(
                "Die Menge muss größer als 0 sein".into(),
            ));
        }
        if unit_price.map(|p| p < Decimal::ZERO).unwrap_or(false) {
            return Err(DomainError::Validation(
                "Der Preis darf nicht negativ sein".into(),
            ));
        }
        let order = self.get(order_id).await?;
        if !order.is_open() {
            return Err(DomainError::Conflict(
                "Diese Bestellung ist abgeschlossen".into(),
            ));
        }
        let (received,): (Decimal,) = sqlx::query_as(RECEIVED_OF_LINE_SQL)
            .bind(line_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        if quantity < received {
            return Err(DomainError::Validation(format!(
                "Es sind bereits {} eingegangen",
                received.normalize()
            )));
        }
        let affected = sqlx::query(
            r#"UPDATE "where_purchase_order_line"
                  SET "quantity" = $3, "unit_price" = $4, "note" = $5
                WHERE "id" = $2 AND "order_id" = $1"#,
        )
        .bind(order_id)
        .bind(line_id)
        .bind(quantity)
        .bind(unit_price)
        .bind(note)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "PurchaseOrderLine",
                id: line_id,
            });
        }
        self.get(order_id).await
    }

    /// Removes a line that has not delivered anything yet.
    pub async fn remove_line(
        &self,
        order_id: i32,
        line_id: i32,
    ) -> Result<PurchaseOrderRow, DomainError> {
        let order = self.get(order_id).await?;
        if !order.is_open() {
            return Err(DomainError::Conflict(
                "Diese Bestellung ist abgeschlossen".into(),
            ));
        }
        let (received,): (Decimal,) = sqlx::query_as(RECEIVED_OF_LINE_SQL)
            .bind(line_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        if received > Decimal::ZERO {
            return Err(DomainError::Conflict(
                "Auf diese Position ist schon Ware eingegangen".into(),
            ));
        }
        let affected = sqlx::query(
            r#"DELETE FROM "where_purchase_order_line" WHERE "id" = $2 AND "order_id" = $1"#,
        )
        .bind(order_id)
        .bind(line_id)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "PurchaseOrderLine",
                id: line_id,
            });
        }
        self.get(order_id).await
    }

    /// Closes the order: the rest is not coming. A line that already delivered keeps its
    /// devices.
    pub async fn close(&self, order_id: i32) -> Result<PurchaseOrderRow, DomainError> {
        self.set_state(order_id, "done").await
    }

    /// Drops the order. Only possible while nothing has arrived.
    pub async fn cancel(&self, order_id: i32) -> Result<PurchaseOrderRow, DomainError> {
        let order = self.get(order_id).await?;
        if order.received > Decimal::ZERO {
            return Err(DomainError::Conflict(
                "Auf diese Bestellung ist schon Ware eingegangen".into(),
            ));
        }
        self.set_state(order_id, "cancelled").await
    }

    async fn set_state(&self, order_id: i32, state: &str) -> Result<PurchaseOrderRow, DomainError> {
        let affected = sqlx::query(
            r#"UPDATE "where_purchase_order"
                  SET "state" = $2, "changed_at" = now()
                WHERE "id" = $1 AND "state" = 'open'"#,
        )
        .bind(order_id)
        .bind(state)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if affected == 0 {
            let order = self.get(order_id).await?;
            return Err(DomainError::Conflict(format!(
                "Diese Bestellung ist bereits {}",
                match order.state.as_str() {
                    "done" => "abgeschlossen",
                    "cancelled" => "storniert",
                    _ => "geschlossen",
                }
            )));
        }
        self.get(order_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(ordered: &str, received: &str) -> PurchaseOrderRow {
        PurchaseOrderRow {
            id: 1,
            number: "4711".into(),
            supplier_no: None,
            supplier_name: None,
            ordered_at: NaiveDate::from_ymd_opt(2026, 3, 7).unwrap(),
            expected_at: None,
            note: None,
            state: "open".into(),
            created_at: Utc::now(),
            created_by: "where".into(),
            lines: 1,
            open_lines: 1,
            ordered: ordered.parse().unwrap(),
            received: received.parse().unwrap(),
        }
    }

    #[test]
    fn the_missing_amount_is_never_negative() {
        assert_eq!(order("10", "4").missing(), "6".parse::<Decimal>().unwrap());
        assert_eq!(order("10", "10").missing(), Decimal::ZERO);
        assert_eq!(order("10", "12").missing(), Decimal::ZERO);
    }

    #[test]
    fn only_an_open_order_takes_more_material() {
        assert!(order("10", "4").is_open());
        let mut closed = order("10", "4");
        closed.state = "done".into();
        assert!(!closed.is_open());
    }
}
