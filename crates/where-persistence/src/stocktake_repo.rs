//! Stocktaking (`Inventur`): counting one warehouse against the book.
//!
//! Starting a stocktake copies every device and batch of a warehouse into its lines
//! together with the amount the book says. Counting fills `counted_quantity`; finishing
//! books the difference as a correction into the stock history and recomputes
//! `"Artikel_Env"` — through `StockRepository`, so a correction looks exactly like every
//! other booking.
//!
//! Two rules keep the result honest:
//!
//! * A **batch** gets the counted amount; a negative book amount is possible and stays
//!   visible until the missing delivery is booked in.
//! * A **device with a serial number** is there or it is not. Counting 0 retires it
//!   (`ausgesondert`) instead of deleting it, so the history and the Prüfprotokolle keep
//!   their device.
//!
//! Lines that were not counted stay untouched — an incomplete stocktake corrects only what
//! somebody actually looked at.

use rust_decimal::Decimal;
use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;
use crate::stock_repo::StockRepository;

/// One stocktake (`where_stocktake`) with the counters the list shows.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StocktakeRow {
    pub id: i32,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub name: String,
    pub note: Option<String>,
    /// `open`, `closed` or `cancelled`.
    pub state: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub started_by: String,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub finished_by: Option<String>,
    pub lines: i64,
    pub counted: i64,
    pub differences: i64,
}

/// One line (`where_stocktake_line`): a device/batch as the book had it, plus what was
/// counted.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StocktakeLineRow {
    pub id: i32,
    pub device_id: i32,
    pub article_id: i32,
    pub article_no: Option<i32>,
    pub article_name: Option<String>,
    pub serial_no: Option<String>,
    pub bin_location: Option<String>,
    pub book_quantity: Decimal,
    pub counted_quantity: Option<Decimal>,
    pub counted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub counted_by: Option<String>,
    pub note: Option<String>,
    /// The device no longer exists (removed while the stocktake was open).
    pub device_gone: bool,
}

impl StocktakeLineRow {
    /// `counted − book`, `None` while the line was not counted.
    pub fn difference(&self) -> Option<Decimal> {
        self.counted_quantity.map(|c| c - self.book_quantity)
    }
}

const COLUMNS: &str = r#"
       s."id", s."warehouse_id", l."Name" AS warehouse_name, s."name", s."note",
       s."state", s."started_at", s."started_by", s."finished_at", s."finished_by",
       (SELECT count(*) FROM "where_stocktake_line" x WHERE x."stocktake_id" = s."id") AS lines,
       (SELECT count(*) FROM "where_stocktake_line" x
         WHERE x."stocktake_id" = s."id" AND x."counted_quantity" IS NOT NULL) AS counted,
       (SELECT count(*) FROM "where_stocktake_line" x
         WHERE x."stocktake_id" = s."id" AND x."counted_quantity" IS NOT NULL
           AND x."counted_quantity" <> x."book_quantity") AS differences
  FROM "where_stocktake" s
  LEFT JOIN "Storage_Lager" l ON l."Primary_Key" = s."warehouse_id""#;

const LIST_SQL: &str = r#"
SELECT%s
 ORDER BY s."started_at" DESC, s."id" DESC"#;

const GET_SQL: &str = r#"
SELECT%s
 WHERE s."id" = $1"#;

const INSERT_SQL: &str = r#"
INSERT INTO "where_stocktake" ("warehouse_id", "name", "note", "started_by")
VALUES ($1, $2, $3, $4) RETURNING "id""#;

/// Snapshot of the warehouse: every device and batch that is **on the shelf**, with the
/// article it belongs to.
///
/// Material that stands at a place of the house (`where_place_id`) is not in the shelf and
/// is therefore not counted here — counting it would report it as missing and retire it.
/// Devices of an inactive article stay out as well.
const FILL_SQL: &str = r#"
INSERT INTO "where_stocktake_line"
       ("stocktake_id", "device_id", "article_id", "article_no", "article_name",
        "serial_no", "bin_location", "book_quantity")
SELECT $1, g."Primary_Key", g."Artikel_PrimKey", a."ArtikelNr", a."Name",
       NULLIF(btrim(COALESCE(g."SerienNr", '')), ''), g."RegalFach",
       COALESCE(g."Menge", 1)
  FROM "Seriennummern" g
  JOIN "Artikel" a ON a."Primary_Key" = g."Artikel_PrimKey"
 WHERE g."Lager_Primkey" = $2
   AND COALESCE(g."Ausgesondert", false) = false
   AND g."where_place_id" IS NULL
   AND COALESCE(a."Aktiv", false)
   AND COALESCE(a."Bestandskontrolle", true)
   AND NOT COALESCE(a."OhneEigenbestand", false)"#;

const LINE_LIST_SQL: &str = r#"
SELECT x."id", x."device_id", x."article_id", x."article_no", x."article_name",
       x."serial_no", x."bin_location", x."book_quantity", x."counted_quantity",
       x."counted_at", x."counted_by", x."note",
       NOT EXISTS (SELECT 1 FROM "Seriennummern" g WHERE g."Primary_Key" = x."device_id")
           AS device_gone
  FROM "where_stocktake_line" x
 WHERE x."stocktake_id" = $1
 ORDER BY x."article_no", x."serial_no" NULLS LAST, x."id""#;

const COUNT_SQL: &str = r#"
UPDATE "where_stocktake_line"
   SET "counted_quantity" = $3, "counted_at" = now(), "counted_by" = $4, "note" = $5
 WHERE "stocktake_id" = $1 AND "id" = $2"#;

const STATE_SQL: &str = r#"
UPDATE "where_stocktake"
   SET "state" = $2, "finished_at" = now(), "finished_by" = $3
 WHERE "id" = $1 AND "state" = 'open'"#;

#[derive(Clone)]
pub struct StocktakeRepository {
    pool: PgPool,
    stock: StockRepository,
}

impl StocktakeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            stock: StockRepository::new(pool.clone()),
            pool,
        }
    }

    pub async fn list(&self) -> Result<Vec<StocktakeRow>, DomainError> {
        let sql = LIST_SQL.replacen("%s", COLUMNS, 1);
        // The text is assembled from two constants above — no user input is involved,
        // hence the `AssertSqlSafe` the query macros ask for.
        sqlx::query_as::<_, StocktakeRow>(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn get(&self, id: i32) -> Result<StocktakeRow, DomainError> {
        let sql = GET_SQL.replacen("%s", COLUMNS, 1);
        sqlx::query_as::<_, StocktakeRow>(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Stocktake",
                id,
            })
    }

    pub async fn lines(&self, id: i32) -> Result<Vec<StocktakeLineRow>, DomainError> {
        sqlx::query_as::<_, StocktakeLineRow>(LINE_LIST_SQL)
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Starts a stocktake: creates the header and fills the lines from the book.
    ///
    /// One open stocktake per warehouse — a second one would count the same shelf twice.
    pub async fn start(
        &self,
        warehouse_id: i32,
        name: &str,
        note: Option<&str>,
        actor: &str,
    ) -> Result<StocktakeRow, DomainError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Die Inventur braucht einen Namen".into(),
            ));
        }
        if warehouse_id <= 0 {
            return Err(DomainError::Validation("Ein Lager ist erforderlich".into()));
        }
        let warehouse: Option<(String, bool)> = sqlx::query_as(
            r#"SELECT COALESCE(NULLIF("Name", ''), 'Lager ' || "Primary_Key"),
                      COALESCE("where_zugriff", false)
                 FROM "Storage_Lager" WHERE "Primary_Key" = $1"#,
        )
        .bind(warehouse_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        let Some((_, zugriff)) = warehouse else {
            return Err(DomainError::NotFound {
                entity: "Stock",
                id: warehouse_id,
            });
        };
        if !zugriff {
            return Err(DomainError::Conflict(
                "Dieses Lager ist für Buchungen gesperrt".into(),
            ));
        }

        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let offen: Option<(i32,)> = sqlx::query_as(
            r#"SELECT "id" FROM "where_stocktake"
                WHERE "warehouse_id" = $1 AND "state" = 'open' LIMIT 1"#,
        )
        .bind(warehouse_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_error)?;
        if let Some((id,)) = offen {
            return Err(DomainError::Conflict(format!(
                "Für dieses Lager läuft schon eine Inventur (Nr. {id})"
            )));
        }

        let (id,): (i32,) = sqlx::query_as(INSERT_SQL)
            .bind(warehouse_id)
            .bind(name)
            .bind(note)
            .bind(if actor.trim().is_empty() {
                "where"
            } else {
                actor.trim()
            })
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;
        sqlx::query(FILL_SQL)
            .bind(id)
            .bind(warehouse_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    /// Records what was counted on one line.
    pub async fn count(
        &self,
        id: i32,
        line_id: i32,
        counted: Option<Decimal>,
        note: Option<&str>,
        actor: &str,
    ) -> Result<(), DomainError> {
        let row = self.get(id).await?;
        if row.state != "open" {
            return Err(DomainError::Conflict(
                "Diese Inventur ist abgeschlossen".into(),
            ));
        }
        if counted.map(|value| value < Decimal::ZERO).unwrap_or(false) {
            return Err(DomainError::Validation(
                "Die gezählte Menge darf nicht negativ sein".into(),
            ));
        }
        let by = if actor.trim().is_empty() {
            "where"
        } else {
            actor.trim()
        };
        let affected = sqlx::query(COUNT_SQL)
            .bind(id)
            .bind(line_id)
            .bind(counted)
            .bind(by)
            .bind(note)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "StocktakeLine",
                id: line_id,
            });
        }
        Ok(())
    }

    /// Closes the stocktake and books every counted difference.
    ///
    /// The lines are booked one by one, each as its own booking in the history. A break in
    /// the middle leaves the stocktake open; running it again books only what is still
    /// wrong, because the difference is computed against the current amount.
    pub async fn finish(&self, id: i32, actor: &str) -> Result<StocktakeRow, DomainError> {
        let row = self.get(id).await?;
        if row.state != "open" {
            return Err(DomainError::Conflict(
                "Diese Inventur ist bereits abgeschlossen".into(),
            ));
        }
        let note = format!("Inventur {id}");
        for line in self.lines(id).await? {
            let Some(counted) = line.counted_quantity else {
                continue;
            };
            if line.device_gone {
                continue;
            }
            let device = self.stock.device(line.article_id, line.device_id).await?;
            if device.without_serial {
                self.stock
                    .correct_amount(line.article_id, line.device_id, counted, Some(&note), actor)
                    .await?;
            } else if counted.is_zero() {
                self.stock
                    .retire_missing(line.article_id, line.device_id, Some(&note), actor)
                    .await?;
            }
        }
        let by = if actor.trim().is_empty() {
            "where"
        } else {
            actor.trim()
        };
        sqlx::query(STATE_SQL)
            .bind(id)
            .bind("closed")
            .bind(by)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        self.get(id).await
    }

    /// Drops a stocktake without booking anything.
    pub async fn cancel(&self, id: i32, actor: &str) -> Result<StocktakeRow, DomainError> {
        let row = self.get(id).await?;
        if row.state != "open" {
            return Err(DomainError::Conflict(
                "Diese Inventur ist bereits abgeschlossen".into(),
            ));
        }
        let by = if actor.trim().is_empty() {
            "where"
        } else {
            actor.trim()
        };
        sqlx::query(STATE_SQL)
            .bind(id)
            .bind("cancelled")
            .bind(by)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        self.get(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(book: &str, counted: Option<&str>) -> StocktakeLineRow {
        StocktakeLineRow {
            id: 1,
            device_id: 1,
            article_id: 1,
            article_no: Some(1001),
            article_name: Some("Scheinwerfer".into()),
            serial_no: None,
            bin_location: None,
            book_quantity: book.parse().unwrap(),
            counted_quantity: counted.map(|c| c.parse().unwrap()),
            counted_at: None,
            counted_by: None,
            note: None,
            device_gone: false,
        }
    }

    #[test]
    fn an_uncounted_line_has_no_difference() {
        assert_eq!(line("10", None).difference(), None);
    }

    #[test]
    fn the_difference_is_counted_minus_book() {
        assert_eq!(
            line("10", Some("8")).difference(),
            Some("-2".parse::<Decimal>().unwrap())
        );
        assert_eq!(line("10", Some("10")).difference(), Some(Decimal::ZERO));
    }
}
