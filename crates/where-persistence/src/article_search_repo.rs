//! Article search — criteria of the form from `img/article-search.PNG`.
//!
//! The legacy search form has two tabs:
//! *General* with twelve criteria and *Pyro* with 27. The field names below follow the
//! process variables of the original form (`art_inklBeschreibung`, `art_inklBundleMat`,
//! `art_exklVirtuell`, `pyro_neq_von` …), the columns the legacy fields.
//!
//! The search is an *AND* combination of all criteria that are set; empty fields
//! do not restrict. The exception is `with_bundle` ("also show related items"): it
//! extends the hit set by articles that share a bundle with a hit
//! — either as material or as the main article.
//!
//! Several article groups or search index values are combined through an operator.
//! The labels come from the language pack (`std_oderEnthaelt`,
//! `std_undEnthaelt`, `std_enhaeltNicht`):
//! `contains` = at least one, `contains_all` = every value, `contains_none` = none.

use rust_decimal::Decimal;
use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;
use crate::rows::{ArticleSearchIndexNode, ArticleSearchRow, ArticleSummaryRow, SavedSearch};

/// Which articles the search looks at at all (the "article type" selector).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArticleType {
    /// "All articles (active)" — only active articles (default of the form).
    #[default]
    Active,
    /// "All inactive articles".
    Inactive,
    /// "All active and inactive articles".
    All,
}

impl ArticleType {
    pub fn key(self) -> &'static str {
        match self {
            ArticleType::Active => "active",
            ArticleType::Inactive => "inactive",
            ArticleType::All => "all",
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "active" => Some(ArticleType::Active),
            "inactive" => Some(ArticleType::Inactive),
            "all" => Some(ArticleType::All),
            _ => None,
        }
    }
}

/// How several values of one criterion are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SetOperator {
    /// "contains" — at least one of the values matches.
    #[default]
    Contains,
    /// "contains any" — like "contains", for several equivalent values.
    ContainsAny,
    /// "contains all" — every value must match.
    ContainsAll,
    /// "contains none" — no value may match.
    ContainsNone,
}

impl SetOperator {
    pub fn key(self) -> &'static str {
        match self {
            SetOperator::Contains => "contains",
            SetOperator::ContainsAny => "contains_any",
            SetOperator::ContainsAll => "contains_all",
            SetOperator::ContainsNone => "contains_none",
        }
    }

    pub fn from_key(value: &str) -> Option<Self> {
        match value {
            "contains" => Some(SetOperator::Contains),
            "contains_any" => Some(SetOperator::ContainsAny),
            "contains_all" => Some(SetOperator::ContainsAll),
            "contains_none" => Some(SetOperator::ContainsNone),
            _ => None,
        }
    }
}

/// Pyro tab of the search form; every field is a range or a text fragment.
#[derive(Debug, Clone, Default)]
pub struct PyroSearch {
    pub un_number: Option<String>,
    pub hazard_class: Option<String>,
    pub ce_nr: Option<String>,
    pub classification: Option<String>,
    pub category: Option<String>,
    pub approval_number: Option<String>,
    pub packing_group: Option<String>,
    pub tunnel_code: Option<String>,
    pub transport_category: Option<String>,
    pub neq_from: Option<Decimal>,
    pub neq_to: Option<Decimal>,
    pub un_points: Option<Decimal>,
    pub lateral_distance_from: Option<Decimal>,
    pub lateral_distance_to: Option<Decimal>,
    pub vertical_distance_from: Option<Decimal>,
    pub vertical_distance_to: Option<Decimal>,
    pub ascending_height_from: Option<Decimal>,
    pub ascending_height_to: Option<Decimal>,
    pub sound_pressure_level_from: Option<Decimal>,
    pub sound_pressure_level_to: Option<Decimal>,
    pub sound_pressure_range_from: Option<Decimal>,
    pub sound_pressure_range_to: Option<Decimal>,
    pub delay_time_from: Option<Decimal>,
    pub delay_time_to: Option<Decimal>,
    pub effect_duration_from: Option<Decimal>,
    pub effect_duration_to: Option<Decimal>,
}

impl PyroSearch {
    pub fn is_empty(&self) -> bool {
        let texts = [
            &self.un_number,
            &self.hazard_class,
            &self.ce_nr,
            &self.classification,
            &self.category,
            &self.approval_number,
            &self.packing_group,
            &self.tunnel_code,
            &self.transport_category,
        ];
        if texts.iter().any(|t| t.is_some()) {
            return false;
        }
        let numbers = [
            self.neq_from,
            self.neq_to,
            self.un_points,
            self.lateral_distance_from,
            self.lateral_distance_to,
            self.vertical_distance_from,
            self.vertical_distance_to,
            self.ascending_height_from,
            self.ascending_height_to,
            self.sound_pressure_level_from,
            self.sound_pressure_level_to,
            self.sound_pressure_range_from,
            self.sound_pressure_range_to,
            self.delay_time_from,
            self.delay_time_to,
            self.effect_duration_from,
            self.effect_duration_to,
        ];
        numbers.iter().all(|z| z.is_none())
    }
}

/// Criteria of the article search form.
#[derive(Debug, Clone, Default)]
pub struct ArticleSearch {
    pub article_type: ArticleType,
    /// "Name" — matches name, short name and match code.
    pub name: Option<String>,
    /// "include article descriptions" — additionally search the six text blocks.
    pub include_descriptions: bool,
    /// "Article no." — exact number; `0` and empty mean "any".
    pub article_no: Option<i32>,
    /// "Inventory no."
    pub inventory_no: Option<String>,
    /// "Serial no." — articles that have a device with this serial number.
    pub serial_no: Option<String>,
    /// "Bundle material: also show related items".
    pub with_bundle: bool,
    pub article_groups: Vec<i32>,
    pub article_groups_op: SetOperator,
    pub search_index: Vec<String>,
    pub search_index_op: SetOperator,
    /// "Supplier" — one or more values (the "+" of the form), combined with OR.
    pub suppliers: Vec<String>,
    pub manufacturer: Option<String>,
    /// "Owner" — articles whose devices belong to this owner.
    pub owner: Option<String>,
    /// "exclude virtual articles".
    pub exclude_virtual: bool,
    pub pyro: PyroSearch,
}

/// Empty text fields count as "not set", the number 0 as "any".
fn text(value: Option<String>) -> Option<String> {
    value
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

impl ArticleSearch {
    /// Cleans the input: drop empty texts, drop the number 0, remove duplicates.
    pub fn normalisiert(mut self) -> Self {
        self.name = text(self.name);
        self.inventory_no = text(self.inventory_no);
        self.serial_no = text(self.serial_no);
        self.suppliers = self
            .suppliers
            .into_iter()
            .filter_map(|s| text(Some(s)))
            .collect();
        self.suppliers.sort();
        self.suppliers.dedup();
        self.manufacturer = text(self.manufacturer);
        self.owner = text(self.owner);
        self.article_no = self.article_no.filter(|n| *n > 0);
        self.article_groups.sort_unstable();
        self.article_groups.dedup();
        self.search_index = std::mem::take(&mut self.search_index)
            .into_iter()
            .filter_map(|s| text(Some(s)))
            .collect();
        self.search_index.sort();
        self.search_index.dedup();

        let p = &mut self.pyro;
        p.un_number = text(p.un_number.take());
        p.hazard_class = text(p.hazard_class.take());
        p.ce_nr = text(p.ce_nr.take());
        p.classification = text(p.classification.take());
        p.category = text(p.category.take());
        p.approval_number = text(p.approval_number.take());
        p.packing_group = text(p.packing_group.take());
        p.tunnel_code = text(p.tunnel_code.take());
        p.transport_category = text(p.transport_category.take());
        self
    }
}

/// Binding order of [`SEARCH_SQL`]; every change here must be mirrored there.
///
/// `$1` limit, `$2` offset, `$3` bundle extension, `$4` article type, `$5` name,
/// `$6` descriptions, `$7` article number, `$8` inventory no., `$9` serial no.,
/// `$10` article groups, `$11` article group operator, `$12` search index,
/// `$13` search index operator, `$14` supplier, `$15` manufacturer, `$16` owner,
/// `$17` exclude virtual, `$18`–`$26` pyro texts, `$27`–`$43` pyro numbers.
const SEARCH_SQL: &str = r#"
WITH basis AS (
    SELECT a."Primary_Key" AS k
      FROM "Artikel" a
     WHERE ($4::text IS NULL
            OR ($4 = 'active' AND COALESCE(a."Aktiv", false))
            OR ($4 = 'inactive' AND NOT COALESCE(a."Aktiv", false))
            OR $4 = 'all')
       AND ($5::text IS NULL
            OR a."Name" ILIKE '%' || $5 || '%'
            OR a."Import_kurzname" ILIKE '%' || $5 || '%'
            OR a."Matchcode" ILIKE '%' || $5 || '%'
            OR ($6::bool AND EXISTS (
                   SELECT 1 FROM "Artikel_Beschreibung" t
                    WHERE t."Artikel_Primkey" = a."Primary_Key"
                      AND (t."Artikelname" ILIKE '%' || $5 || '%'
                        OR t."Name_kurz" ILIKE '%' || $5 || '%'
                        OR t."Beschreibung_short" ILIKE '%' || $5 || '%'
                        OR t."Beschreibung_long" ILIKE '%' || $5 || '%'
                        OR t."Beschreibung_tech" ILIKE '%' || $5 || '%'
                        OR t."Beschreibung_delivery" ILIKE '%' || $5 || '%'
                        OR t."Beschreibung_package" ILIKE '%' || $5 || '%'
                        OR t."Beschreibung_webshop" ILIKE '%' || $5 || '%'))))
       AND ($7::int IS NULL OR a."ArtikelNr" = $7)
       AND ($8::text IS NULL OR a."InventarNr" ILIKE '%' || $8 || '%')
       AND ($9::text IS NULL OR EXISTS (
              SELECT 1 FROM "Seriennummern" g
               WHERE g."Artikel_PrimKey" = a."Primary_Key"
                 AND g."SerienNr" ILIKE '%' || $9 || '%'))
       AND ($10::int[] IS NULL OR CASE $11::text
              WHEN 'contains_all' THEN
                   (SELECT count(DISTINCT w."Storage_Primkey")
                      FROM "Artikel_Warengruppe" w
                     WHERE w."Artikel_Primkey" = a."Primary_Key"
                       AND w."Storage_Primkey" = ANY($10)) = cardinality($10)
              WHEN 'contains_none' THEN NOT EXISTS (
                   SELECT 1 FROM "Artikel_Warengruppe" w
                    WHERE w."Artikel_Primkey" = a."Primary_Key"
                      AND w."Storage_Primkey" = ANY($10))
              ELSE EXISTS (
                   SELECT 1 FROM "Artikel_Warengruppe" w
                    WHERE w."Artikel_Primkey" = a."Primary_Key"
                      AND w."Storage_Primkey" = ANY($10))
            END)
       AND ($12::text[] IS NULL OR CASE $13::text
              WHEN 'contains_all' THEN
                   (SELECT count(DISTINCT i."Artikelindex")
                      FROM "Artikel_Searchindex" i
                     WHERE i."Artikel_Key" = a."Primary_Key"
                       AND i."Artikelindex" = ANY($12)) = cardinality($12)
              WHEN 'contains_none' THEN NOT EXISTS (
                   SELECT 1 FROM "Artikel_Searchindex" i
                    WHERE i."Artikel_Key" = a."Primary_Key"
                      AND i."Artikelindex" = ANY($12))
              ELSE EXISTS (
                   SELECT 1 FROM "Artikel_Searchindex" i
                    WHERE i."Artikel_Key" = a."Primary_Key"
                      AND i."Artikelindex" = ANY($12))
            END)
       AND ($14::text[] IS NULL
            OR a."Lieferant_o" ILIKE ANY (SELECT '%' || x || '%' FROM unnest($14) AS x)
            OR EXISTS (SELECT 1 FROM "Artikel_Lieferanten" al
                         JOIN "Adresse" ad ON ad."Primary_key" = al."Lieferant_Primkey"
                        WHERE al."Artikel_PrimKey" = a."Primary_Key"
                          AND (ad."Name1" ILIKE ANY (SELECT '%' || x || '%' FROM unnest($14) AS x)
                            OR ad."KundenNr"::text = ANY($14))))
       AND ($15::text IS NULL OR a."Hersteller" ILIKE '%' || $15 || '%')
       AND ($16::text IS NULL OR EXISTS (
              SELECT 1 FROM "Seriennummern" g
               WHERE g."Artikel_PrimKey" = a."Primary_Key"
                 AND g."Eigentuemer" ILIKE '%' || $16 || '%'))
       AND ($17::bool IS NOT TRUE OR COALESCE(a."virtuellerArtikel", false) = false)
       AND ($18::text IS NULL OR a."pyro_UN_number" ILIKE '%' || $18 || '%')
       AND ($19::text IS NULL OR a."pyro_hazard_class_o" ILIKE '%' || $19 || '%')
       AND ($20::text IS NULL OR a."pyro_bam_id" ILIKE '%' || $20 || '%')
       AND ($21::text IS NULL OR a."pyro_categorisation" ILIKE '%' || $21 || '%')
       AND ($22::text IS NULL OR a."pyro_category" ILIKE '%' || $22 || '%')
       AND ($23::text IS NULL OR a."pyro_approval_number" ILIKE '%' || $23 || '%')
       AND ($24::text IS NULL OR a."pyro_packing_group" ILIKE '%' || $24 || '%')
       AND ($25::text IS NULL OR a."pyro_tunnel_delimitation_code" ILIKE '%' || $25 || '%')
       AND ($26::text IS NULL OR a."pyro_transport_category" ILIKE '%' || $26 || '%')
       AND ($27::numeric IS NULL OR a."pyro_neq" >= $27)
       AND ($28::numeric IS NULL OR a."pyro_neq" <= $28)
       AND ($29::numeric IS NULL OR a."pyro_UN_points" = $29)
       AND ($30::numeric IS NULL OR a."pyro_lateral_safety_distance" >= $30)
       AND ($31::numeric IS NULL OR a."pyro_lateral_safety_distance" <= $31)
       AND ($32::numeric IS NULL OR a."pyro_vertical_safety_distance" >= $32)
       AND ($33::numeric IS NULL OR a."pyro_vertical_safety_distance" <= $33)
       AND ($34::numeric IS NULL OR a."pyro_ascending_height" >= $34)
       AND ($35::numeric IS NULL OR a."pyro_ascending_height" <= $35)
       AND ($36::numeric IS NULL OR a."pyro_sound_pressure_level" >= $36)
       AND ($37::numeric IS NULL OR a."pyro_sound_pressure_level" <= $37)
       AND ($38::numeric IS NULL OR a."pyro_sound_pressure_range" >= $38)
       AND ($39::numeric IS NULL OR a."pyro_sound_pressure_range" <= $39)
       AND ($40::numeric IS NULL OR a."pyro_delay_time" >= $40)
       AND ($41::numeric IS NULL OR a."pyro_delay_time" <= $41)
       AND ($42::numeric IS NULL OR a."pyro_duration_time" >= $42)
       AND ($43::numeric IS NULL OR a."pyro_duration_time" <= $43)
),
hits AS (
    SELECT k FROM basis
    UNION
    SELECT b."MainArtikel_PrimKey" FROM "Artikel_Bundle" b
     WHERE $3::bool AND b."MainArtikel_PrimKey" IS NOT NULL
       AND b."Material_PrimKey" IN (SELECT k FROM basis)
    UNION
    SELECT b."Material_PrimKey" FROM "Artikel_Bundle" b
     WHERE $3::bool AND b."Material_PrimKey" IS NOT NULL
       AND b."MainArtikel_PrimKey" IN (SELECT k FROM basis)
)
SELECT a."Primary_Key", a."ArtikelNr", a."Name", a."Hersteller", a."InventarNr",
       a."Aktiv", a."ZurMiete_Flag", a."ZumVerkauf_Flag",
       e.stock_on_hand, e.available_stock, e.rental_stock, e.sale_stock,
       p.rental_price, p.sale_price,
       count(*) OVER () AS total
  FROM "Artikel" a
  LEFT JOIN LATERAL (
      SELECT SUM("Bestand_Gesamt") AS stock_on_hand,
             SUM("Bestand_Gesamt"
                 - COALESCE("Bestand_Festblockade", 0)
                 - COALESCE("Bestand_EntnahmeGesamt", 0)) AS available_stock,
             SUM(COALESCE("Bestand_GesamtMiete", 0)) AS rental_stock,
             SUM(COALESCE("Bestand_GesamtNVerkauf", 0)
                 + COALESCE("Bestand_GesamtGVerkauf", 0)) AS sale_stock
        FROM "Artikel_Env"
       WHERE "Artikel_Primkey" = a."Primary_Key"
  ) e ON true
  LEFT JOIN LATERAL (
      SELECT m."Mietpreis" AS rental_price, m."VKPreisNetto" AS sale_price
        FROM "Art_MandantDaten" m
       WHERE m."Artikel_Primkey" = a."Primary_Key"
       ORDER BY m."Mandanten_ID" NULLS LAST, m."Primary_key"
       LIMIT 1
  ) p ON true
 WHERE a."Primary_Key" IN (SELECT k FROM hits)
 ORDER BY a."ArtikelNr"
 LIMIT $1 OFFSET $2"#;

const SAVED_SEARCHES_SQL: &str = r#"
SELECT "id", "name", "criteria"::text AS criteria,
       "created_at", "changed_at"
  FROM "where_article_search"
 ORDER BY lower(btrim("name"))"#;

/// Stores under the name; an existing entry is overwritten.
const SEARCH_SAVE_SQL: &str = r#"
INSERT INTO "where_article_search" ("name", "criteria")
VALUES ($1, $2::jsonb)
ON CONFLICT (lower(btrim("name"))) DO UPDATE
   SET "criteria" = EXCLUDED."criteria",
       "changed_at" = now()
RETURNING "id", "name", "criteria"::text AS criteria,
          "created_at", "changed_at""#;

const SEARCH_DELETE_SQL: &str = r#"DELETE FROM "where_article_search" WHERE "id" = $1"#;

/// Search index tree (the "select index" dialog).
const SEARCH_INDEX_SQL: &str = r#"
SELECT "Primary_Key" AS id,
       "Index" AS name,
       NULLIF("Parent_ID", 0) AS parent_id
  FROM "Storage_ArtikelSearchIndex"
 ORDER BY "Index""#;

/// Repository for the article search and the saved searches.
#[derive(Clone)]
pub struct ArticleSearchRepository {
    pool: PgPool,
}

impl ArticleSearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Runs the search and returns the page together with the total count.
    pub async fn search(
        &self,
        search: &ArticleSearch,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<ArticleSummaryRow>, i64), DomainError> {
        let article_groups = if search.article_groups.is_empty() {
            None
        } else {
            Some(search.article_groups.clone())
        };
        let search_index = if search.search_index.is_empty() {
            None
        } else {
            Some(search.search_index.clone())
        };
        let suppliers = if search.suppliers.is_empty() {
            None
        } else {
            Some(search.suppliers.clone())
        };
        let p = &search.pyro;

        let rows = sqlx::query_as::<_, ArticleSearchRow>(SEARCH_SQL)
            .bind(limit)
            .bind(offset)
            .bind(search.with_bundle)
            .bind(search.article_type.key())
            .bind(search.name.as_deref())
            .bind(search.include_descriptions)
            .bind(search.article_no)
            .bind(search.inventory_no.as_deref())
            .bind(search.serial_no.as_deref())
            .bind(article_groups)
            .bind(search.article_groups_op.key())
            .bind(search_index)
            .bind(search.search_index_op.key())
            .bind(suppliers)
            .bind(search.manufacturer.as_deref())
            .bind(search.owner.as_deref())
            .bind(search.exclude_virtual)
            .bind(p.un_number.as_deref())
            .bind(p.hazard_class.as_deref())
            .bind(p.ce_nr.as_deref())
            .bind(p.classification.as_deref())
            .bind(p.category.as_deref())
            .bind(p.approval_number.as_deref())
            .bind(p.packing_group.as_deref())
            .bind(p.tunnel_code.as_deref())
            .bind(p.transport_category.as_deref())
            .bind(p.neq_from)
            .bind(p.neq_to)
            .bind(p.un_points)
            .bind(p.lateral_distance_from)
            .bind(p.lateral_distance_to)
            .bind(p.vertical_distance_from)
            .bind(p.vertical_distance_to)
            .bind(p.ascending_height_from)
            .bind(p.ascending_height_to)
            .bind(p.sound_pressure_level_from)
            .bind(p.sound_pressure_level_to)
            .bind(p.sound_pressure_range_from)
            .bind(p.sound_pressure_range_to)
            .bind(p.delay_time_from)
            .bind(p.delay_time_to)
            .bind(p.effect_duration_from)
            .bind(p.effect_duration_to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;

        let total = rows.first().map(|r| r.total).unwrap_or(0);
        Ok((rows.into_iter().map(Into::into).collect(), total))
    }

    pub async fn saved(&self) -> Result<Vec<SavedSearch>, DomainError> {
        sqlx::query_as::<_, SavedSearch>(SAVED_SEARCHES_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    /// Stores the search under `name` or replaces it; `criteria` is the JSON body.
    pub async fn save(&self, name: &str, criteria: &str) -> Result<SavedSearch, DomainError> {
        sqlx::query_as::<_, SavedSearch>(SEARCH_SAVE_SQL)
            .bind(name)
            .bind(criteria)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        let result = sqlx::query(SEARCH_DELETE_SQL)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "Saved search",
                id,
            });
        }
        Ok(())
    }

    /// Search index tree for the "select index" dialog.
    pub async fn search_index(&self) -> Result<Vec<ArticleSearchIndexNode>, DomainError> {
        sqlx::query_as::<_, ArticleSearchIndexNode>(SEARCH_INDEX_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_texts_and_zero_number_do_not_count() {
        let s = ArticleSearch {
            name: Some("   ".into()),
            inventory_no: Some(" INV-1 ".into()),
            article_no: Some(0),
            ..Default::default()
        }
        .normalisiert();
        assert_eq!(s.name, None);
        assert_eq!(s.inventory_no.as_deref(), Some("INV-1"));
        assert_eq!(s.article_no, None);
    }

    #[test]
    fn article_groups_are_sorted_and_deduplicated() {
        let s = ArticleSearch {
            article_groups: vec![7, 3, 7],
            search_index: vec!["B".into(), "A".into(), "A".into()],
            ..Default::default()
        }
        .normalisiert();
        assert_eq!(s.article_groups, vec![3, 7]);
        assert_eq!(s.search_index, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn operators_and_article_types_roundtrip() {
        for op in [
            SetOperator::Contains,
            SetOperator::ContainsAny,
            SetOperator::ContainsAll,
            SetOperator::ContainsNone,
        ] {
            assert_eq!(SetOperator::from_key(op.key()), Some(op));
        }
        for typ in [ArticleType::Active, ArticleType::Inactive, ArticleType::All] {
            assert_eq!(ArticleType::from_key(typ.key()), Some(typ));
        }
        assert_eq!(ArticleType::from_key("vielleicht"), None);
        assert_eq!(SetOperator::from_key("vielleicht"), None);
    }

    #[test]
    fn pyro_is_empty_when_nothing_is_set() {
        assert!(PyroSearch::default().is_empty());
        let p = PyroSearch {
            un_number: Some("0336".into()),
            ..Default::default()
        };
        assert!(!p.is_empty());
        let p = PyroSearch {
            effect_duration_to: Some(Decimal::new(5, 0)),
            ..Default::default()
        };
        assert!(!p.is_empty());
    }
}
