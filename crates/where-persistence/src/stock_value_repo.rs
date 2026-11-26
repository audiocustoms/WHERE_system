//! Stock value (`Bestandswert`): what the shelf is worth, per warehouse and article group.
//!
//! The value of a row is `"Menge" × "Kaufpreis"` — the purchase price that was recorded
//! with the goods receipt. Material that stands at a place of the house
//! (`where_place_id`), discarded devices and devices of an inactive article are not part
//! of it: the report is about what lies in the shelf and can be sold or rented.
//!
//! An article can sit in several article groups. The report counts it under the **first**
//! group by name (`"Storage_Warengruppe"."Index"`), otherwise every article of two groups
//! would appear twice and the sums would be wrong.

use rust_decimal::Decimal;
use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;

/// One line of the report: a warehouse with one article group.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StockValueRow {
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    /// Empty when the article belongs to no group.
    pub group_name: String,
    /// Articles behind the line.
    pub articles: i64,
    pub amount: Decimal,
    pub value: Decimal,
}

const SQL: &str = r#"
WITH wert AS (
    SELECT g."Lager_Primkey" AS warehouse_id,
           g."Artikel_PrimKey" AS article_id,
           SUM(COALESCE(g."Menge", 0)) AS amount,
           SUM(COALESCE(g."Menge", 0) * COALESCE(g."Kaufpreis", 0)) AS value
      FROM "Seriennummern" g
     WHERE COALESCE(g."Ausgesondert", false) = false
       AND g."where_place_id" IS NULL
     GROUP BY g."Lager_Primkey", g."Artikel_PrimKey"
),
gruppe AS (
    SELECT DISTINCT ON (aw."Artikel_Primkey")
           aw."Artikel_Primkey" AS article_id,
           w."Index" AS group_name
      FROM "Artikel_Warengruppe" aw
      JOIN "Storage_Warengruppe" w ON w."Primary_Key" = aw."Storage_Primkey"
     ORDER BY aw."Artikel_Primkey", w."Index"
)
SELECT w."warehouse_id",
       COALESCE(NULLIF(l."Name", ''), 'Lager ' || w."warehouse_id") AS warehouse_name,
       COALESCE(gr."group_name", '') AS group_name,
       count(*) AS articles,
       SUM(w."amount") AS amount,
       SUM(w."value") AS value
  FROM wert w
  JOIN "Artikel" a ON a."Primary_Key" = w."article_id"
  LEFT JOIN gruppe gr ON gr."article_id" = w."article_id"
  LEFT JOIN "Storage_Lager" l ON l."Primary_Key" = w."warehouse_id"
 WHERE COALESCE(a."Aktiv", false)
   AND ($1::int IS NULL OR w."warehouse_id" = $1)
 GROUP BY w."warehouse_id", l."Name", COALESCE(gr."group_name", '')
 ORDER BY 6 DESC, warehouse_name, group_name"#;

#[derive(Clone)]
pub struct StockValueRepository {
    pool: PgPool,
}

impl StockValueRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Value per warehouse and article group, largest first.
    pub async fn report(
        &self,
        warehouse_id: Option<i32>,
    ) -> Result<Vec<StockValueRow>, DomainError> {
        sqlx::query_as::<_, StockValueRow>(SQL)
            .bind(warehouse_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_value_is_amount_times_purchase_price() {
        // 3 pieces at 12,50 — the row of the report sums what the receipt wrote.
        let value: Decimal = "37.50".parse().unwrap();
        let amount: Decimal = "3".parse().unwrap();
        let price: Decimal = "12.50".parse().unwrap();
        assert_eq!(amount * price, value);
    }
}
