//! Reorder suggestions (`Beschaffung`): what is below its reorder level, and how much of
//! it is missing.
//!
//! The reorder level is the article's `"VerkaufMindBestand"` (the field `Meldebestand` of
//! the article form). It is compared with the **available** amount of one warehouse —
//! `Gesamt − Festblockade − Entnahme` from `"Artikel_Env"` — so material standing in the
//! foyer counts as gone for the shelf, exactly like everywhere else.
//!
//! Two things come along that make the list usable:
//!
//! * the **consumption** of the last months out of the stock history (`consume` and the
//!   negative corrections of a stocktake), so a level that is too low becomes visible;
//! * the **lead time** of the supplier, so a long delivery does not surprise anybody.
//!
//! No new table: everything is derived from what the bookings have written.

use rust_decimal::Decimal;
use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;

/// One suggestion line: an article in a warehouse below its reorder level.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SuggestionRow {
    pub article_id: i32,
    pub article_no: Option<i32>,
    pub article_name: Option<String>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    /// `Gesamt − Festblockade − Entnahme` of that warehouse.
    pub available: Decimal,
    /// `"VerkaufMindBestand"`.
    pub reorder_level: Decimal,
    /// `reorder_level − available`, never negative.
    pub missing: Decimal,
    /// What was consumed within the window (all warehouses of the article).
    pub used: Decimal,
    /// `"Artikel_Lieferanten"."Lieferzeit_Tage"` of the favourite supplier.
    pub lead_time_days: Option<i32>,
    pub supplier_no: Option<i32>,
    pub supplier_name: Option<String>,
    /// Article number at the supplier (`"ArtLiefNr"`).
    pub supplier_article_no: Option<String>,
}

/// Availability per (article, warehouse). Only warehouses that carry a row in
/// `"Artikel_Env"` appear here.
const AVAILABILITY_SQL: &str = r#"
WITH env AS (
    SELECT e."Artikel_Primkey" AS article_id,
           e."Lager_Primkey" AS warehouse_id,
           SUM(COALESCE(e."Bestand_Gesamt", 0)
               - COALESCE(e."Bestand_Festblockade", 0)
               - COALESCE(e."Bestand_EntnahmeGesamt", 0)) AS available
      FROM "Artikel_Env" e
     GROUP BY e."Artikel_Primkey", e."Lager_Primkey"
),
-- An article that has no row anywhere is out of stock everywhere: it is placed in the
-- default warehouse, where the order would go.
empty AS (
    SELECT a."Primary_Key" AS article_id,
           (SELECT l."Primary_Key" FROM "Storage_Lager" l
             ORDER BY COALESCE(l."__DefaultLager", false) DESC, l."Primary_Key"
             LIMIT 1) AS warehouse_id,
           0::numeric AS available
      FROM "Artikel" a
     WHERE COALESCE(a."Aktiv", false)
       AND COALESCE(a."VerkaufMindBestand", 0) > 0
       AND NOT EXISTS (SELECT 1 FROM "Artikel_Env" e WHERE e."Artikel_Primkey" = a."Primary_Key")
),
basis AS (
    SELECT * FROM env
    UNION ALL
    SELECT * FROM empty
),
-- Consumption of the article, over all its warehouses: `consume` takes material out, a
-- negative correction of a stocktake is shrinkage.
used AS (
    SELECT m."article_id",
           SUM(CASE WHEN m."kind" = 'consume' THEN COALESCE(m."quantity", 0)
                    WHEN m."kind" = 'count' AND COALESCE(m."quantity", 0) < 0
                    THEN -COALESCE(m."quantity", 0)
                    ELSE 0 END) AS used
      FROM "where_stock_movement" m
     WHERE m."created_at" >= now() - make_interval(days => $1)
       AND m."kind" IN ('consume', 'count')
     GROUP BY m."article_id"
),
-- The supplier the article is bought from: the favourite first.
supplier AS (
    SELECT DISTINCT ON (al."Artikel_PrimKey")
           al."Artikel_PrimKey" AS article_id,
           al."Lieferant_Primkey" AS supplier_no,
           NULLIF(al."Lieferzeit_Tage", 0) AS lead_time_days,
           NULLIF(btrim(COALESCE(al."ArtLiefNr", '')), '') AS supplier_article_no
      FROM "Artikel_Lieferanten" al
     WHERE al."Lieferant_Primkey" > 0
     ORDER BY al."Artikel_PrimKey", COALESCE(al."Favorit", false) DESC,
              COALESCE(al."EK_Preis", 0) DESC, al."lfd_Pos"
)
SELECT a."Primary_Key" AS article_id,
       a."ArtikelNr" AS article_no,
       a."Name" AS article_name,
       b.warehouse_id,
       l."Name" AS warehouse_name,
       b.available,
       a."VerkaufMindBestand" AS reorder_level,
       (a."VerkaufMindBestand" - b.available) AS missing,
       COALESCE(u.used, 0) AS used,
       s.lead_time_days,
       s.supplier_no,
       COALESCE(ad."Name1", ad."Name2", ad."Name3") AS supplier_name,
       s.supplier_article_no
  FROM basis b
  JOIN "Artikel" a ON a."Primary_Key" = b.article_id
  LEFT JOIN "Storage_Lager" l ON l."Primary_Key" = b.warehouse_id
  LEFT JOIN used u ON u.article_id = a."Primary_Key"
  LEFT JOIN supplier s ON s.article_id = a."Primary_Key"
  LEFT JOIN "Adresse" ad ON ad."Primary_key" = s.supplier_no
 WHERE COALESCE(a."Aktiv", false)
   AND COALESCE(a."Bestandskontrolle", true)
   AND NOT COALESCE(a."OhneEigenbestand", false)
   AND COALESCE(a."VerkaufMindBestand", 0) > 0
   AND b.available < a."VerkaufMindBestand"
 ORDER BY missing DESC, a."ArtikelNr", b.warehouse_id"#;

#[derive(Clone)]
pub struct ProcurementRepository {
    pool: PgPool,
}

impl ProcurementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Articles below their reorder level, largest gap first.
    ///
    /// `days` is the window of the consumption figures (30, 90, 365 …).
    pub async fn suggestions(&self, days: i32) -> Result<Vec<SuggestionRow>, DomainError> {
        let days = days.clamp(1, 3650);
        sqlx::query_as::<_, SuggestionRow>(AVAILABILITY_SQL)
            .bind(days)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gap is what has to be ordered — never a negative number.
    #[test]
    fn the_gap_follows_the_reorder_level() {
        let row = SuggestionRow {
            article_id: 1,
            article_no: Some(1001),
            article_name: Some("Gaffa".into()),
            warehouse_id: 1,
            warehouse_name: Some("Hauptlager".into()),
            available: "2".parse().unwrap(),
            reorder_level: "10".parse().unwrap(),
            missing: "8".parse().unwrap(),
            used: Decimal::ZERO,
            lead_time_days: Some(3),
            supplier_no: None,
            supplier_name: None,
            supplier_article_no: None,
        };
        assert_eq!(row.missing, row.reorder_level - row.available);
        assert!(row.missing > Decimal::ZERO);
    }
}
