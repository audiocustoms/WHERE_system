use chrono::{Datelike, Utc};
use sqlx::PgPool;
use where_domain::{Article, ArticleTexts, DomainError, Page};

use crate::error::db_error;
use crate::number_series_repo::NumberSeriesRepository;
use crate::rows::{
    ArticlePriceRow, ArticleRow, ArticleSummaryRow, ArticleSupplierRow, ArticleTextRow,
    LanguageRow, ScaleCategoryRow, ScaleStepRow,
};

/// Search fields of the list: name, short name, match code, manufacturer, inventory no., EAN, number.
///
/// Stock comes from `"Artikel_Env"` (one row per article and warehouse) and is
/// summed across all warehouses, separated into rental and sale — that is what the
/// list columns "Bestand Miete" / "Bestand Verkauf" show. `NULL` means "no
/// booking", `0` means "booked but empty" — the list renders the two differently.
/// Prices come from the lowest client row of `"Art_MandantDaten"`, like the mask.
/// `$4` filters by article group; it includes the subgroups.
const LIST_SQL: &str = r#"
SELECT a."Primary_Key", a."ArtikelNr", a."Name", a."Hersteller", a."InventarNr",
       a."Aktiv", a."ZurMiete_Flag", a."ZumVerkauf_Flag",
       e.stock_on_hand, e.available_stock, e.rental_stock, e.sale_stock,
       p.rental_price, p.sale_price
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
 WHERE ($3::text IS NULL
        OR a."Name" ILIKE '%' || $3 || '%'
        OR a."Import_kurzname" ILIKE '%' || $3 || '%'
        OR a."Matchcode" ILIKE '%' || $3 || '%'
        OR a."Hersteller" ILIKE '%' || $3 || '%'
        OR a."InventarNr" ILIKE '%' || $3 || '%'
        OR a."EANCode" ILIKE '%' || $3 || '%'
        OR a."ArtikelNr"::text ILIKE '%' || $3 || '%')
   AND ($5::bool IS NULL OR COALESCE(a."Aktiv", false) = $5)
   AND ($4::int IS NULL
        OR ($4::int = 0 AND NOT EXISTS (
               SELECT 1 FROM "Artikel_Warengruppe" aw
                WHERE aw."Artikel_Primkey" = a."Primary_Key"))
        OR ($4::int <> 0 AND EXISTS (
               SELECT 1 FROM "Artikel_Warengruppe" aw
                WHERE aw."Artikel_Primkey" = a."Primary_Key"
                  AND aw."Storage_Primkey" IN (
                      WITH RECURSIVE sub AS (
                          SELECT "Primary_Key" FROM "Storage_Warengruppe" WHERE "Primary_Key" = $4
                          UNION ALL
                          SELECT w."Primary_Key"
                            FROM "Storage_Warengruppe" w
                            JOIN sub ON w."Parent_ID" = sub."Primary_Key"
                      )
                      SELECT "Primary_Key" FROM sub))))
 ORDER BY a."ArtikelNr"
 LIMIT $1 OFFSET $2"#;

const COUNT_SQL: &str = r#"
SELECT count(*) FROM "Artikel" a
 WHERE ($1::text IS NULL
        OR a."Name" ILIKE '%' || $1 || '%'
        OR a."Import_kurzname" ILIKE '%' || $1 || '%'
        OR a."Matchcode" ILIKE '%' || $1 || '%'
        OR a."Hersteller" ILIKE '%' || $1 || '%'
        OR a."InventarNr" ILIKE '%' || $1 || '%'
        OR a."EANCode" ILIKE '%' || $1 || '%'
        OR a."ArtikelNr"::text ILIKE '%' || $1 || '%')
   AND ($3::bool IS NULL OR COALESCE(a."Aktiv", false) = $3)
   AND ($2::int IS NULL
        OR ($2::int = 0 AND NOT EXISTS (
               SELECT 1 FROM "Artikel_Warengruppe" aw
                WHERE aw."Artikel_Primkey" = a."Primary_Key"))
        OR ($2::int <> 0 AND EXISTS (
               SELECT 1 FROM "Artikel_Warengruppe" aw
                WHERE aw."Artikel_Primkey" = a."Primary_Key"
                  AND aw."Storage_Primkey" IN (
                      WITH RECURSIVE sub AS (
                          SELECT "Primary_Key" FROM "Storage_Warengruppe" WHERE "Primary_Key" = $2
                          UNION ALL
                          SELECT w."Primary_Key"
                            FROM "Storage_Warengruppe" w
                            JOIN sub ON w."Parent_ID" = sub."Primary_Key"
                      )
                      SELECT "Primary_Key" FROM sub))))"#;

const GET_SQL: &str = r#"SELECT * FROM "Artikel" WHERE "Primary_Key" = $1"#;

/// Prices of the tabs `Miete` and `Verkauf`. WHERE works with one client, so the
/// lowest client row of the article is used (`Mandanten_ID` empty sorts last).
const PRICE_SQL: &str = r#"
SELECT "Mietpreis", "BasisMietpreis", "Kautionsbetrag",
       "Mietpreis_Rolltage", "BasisPreis_Rolltage",
       "Mietpreis_OffDays", "BasisPreis_OffDays", "Mietpreis_WE",
       "VKPreisNetto", "VKPreisBrutto", "VKPreisEmpfehlung",
       "Kapitalbindung", "VerkaufEKNEtto",
       "EKNetto", "EKNetto_Rolltage", "EKNetto_OffDays", "EKNetto_WE",
       "MwStMietArtikel", "MwStVKArtikel", "VerkaufEKzuVKAufschlag",
       "FiBu_KtoErtragMiete", "FiBu_KtoErtragVerkauf",
       "FiBu_KST_Miete", "FiBu_KST_Verkauf",
       "ZRabStaffel_PK", "ZRabStaffelRollD_PK", "ZRabStaffelOffD_PK", "MengenStaffel_PK",
       "Gebrauchtpreis", "Lagerplatz"
  FROM "Art_MandantDaten"
 WHERE "Artikel_Primkey" = $1
 ORDER BY "Mandanten_ID" NULLS LAST, "Primary_key"
 LIMIT 1"#;

const PRICE_UPDATE_SQL: &str = r#"
UPDATE "Art_MandantDaten" SET
    "Mietpreis" = $2, "BasisMietpreis" = $3, "Kautionsbetrag" = $4,
    "Mietpreis_Rolltage" = $5, "BasisPreis_Rolltage" = $6,
    "Mietpreis_OffDays" = $7, "BasisPreis_OffDays" = $8, "Mietpreis_WE" = $9,
    "VKPreisNetto" = $10, "VKPreisBrutto" = $11, "VKPreisEmpfehlung" = $12,
    "Kapitalbindung" = $13, "VerkaufEKNEtto" = $14,
    "EKNetto" = $15, "EKNetto_Rolltage" = $16, "EKNetto_OffDays" = $17, "EKNetto_WE" = $18,
    "MwStMietArtikel" = $19, "MwStVKArtikel" = $20, "VerkaufEKzuVKAufschlag" = $21,
    "FiBu_KtoErtragMiete" = $22, "FiBu_KtoErtragVerkauf" = $23,
    "FiBu_KST_Miete" = $24, "FiBu_KST_Verkauf" = $25,
    "ZRabStaffel_PK" = $26, "ZRabStaffelRollD_PK" = $27, "ZRabStaffelOffD_PK" = $28,
    "MengenStaffel_PK" = $29, "Gebrauchtpreis" = $30, "Lagerplatz" = $31
 WHERE "Primary_key" = $1"#;

const PRICE_ROW_SQL: &str = r#"
SELECT "Primary_key" FROM "Art_MandantDaten"
 WHERE "Artikel_Primkey" = $1
 ORDER BY "Mandanten_ID" NULLS LAST, "Primary_key"
 LIMIT 1"#;

const PRICE_INSERT_SQL: &str = r#"
INSERT INTO "Art_MandantDaten" (
    "Artikel_Primkey", "Artikel_Nr", "Mandanten_ID",
    "Mietpreis", "BasisMietpreis", "Kautionsbetrag",
    "Mietpreis_Rolltage", "BasisPreis_Rolltage",
    "Mietpreis_OffDays", "BasisPreis_OffDays", "Mietpreis_WE",
    "VKPreisNetto", "VKPreisBrutto", "VKPreisEmpfehlung",
    "Kapitalbindung", "VerkaufEKNEtto",
    "EKNetto", "EKNetto_Rolltage", "EKNetto_OffDays", "EKNetto_WE",
    "MwStMietArtikel", "MwStVKArtikel", "VerkaufEKzuVKAufschlag",
    "FiBu_KtoErtragMiete", "FiBu_KtoErtragVerkauf",
    "FiBu_KST_Miete", "FiBu_KST_Verkauf",
    "ZRabStaffel_PK", "ZRabStaffelRollD_PK", "ZRabStaffelOffD_PK", "MengenStaffel_PK",
    "Gebrauchtpreis", "Lagerplatz"
) VALUES ($1,
    (SELECT "ArtikelNr" FROM "Artikel" WHERE "Primary_Key" = $1), 0,
    $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
    $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25,
    $26, $27, $28, $29, $30, $31)"#;

const SCALE_CATEGORIES_SQL: &str = r#"
SELECT "Primary_Key", "Name", "Verkauf", "Standard"
  FROM "StaffelKategorie"
 WHERE "Verkauf" = $1
 ORDER BY "Standard" DESC NULLS LAST, "Name""#;

const SCALE_STEPS_SQL: &str = r#"
SELECT "Miettage", "Faktor", "Rabatt"
  FROM "Staffelrabatt"
 WHERE "Kategorie_Primkey" = $1
 ORDER BY "Miettage""#;

/// Pulls the counter up to the highest number already issued when it lags behind —
/// otherwise the allocation loop would have to skip every taken number one by one.
const ALIGN_MAX_SQL: &str = r#"
UPDATE "where_number_series"
   SET "current_value" = GREATEST("current_value",
         COALESCE((SELECT MAX("ArtikelNr") FROM "Artikel"), 0)),
       "changed_at" = now()
 WHERE "key" = 'article'
   AND "yearly_reset" = false"#;

const INSERT_SQL: &str = r#"INSERT INTO "Artikel" (
    "ArtikelNr", "InventarNr", "Name", "Import_kurzname", "Matchcode", "Import_Einheit",
    "Aktiv", "Inventar_Flag", "ZurMiete_Flag", "ZumVerkauf_Flag", "Textsync_Flag",
    "Bestandskontrolle", "OhneEigenbestand", "SerienNr_Flag", "Dispokontrolle", "virtuellerArtikel",
    "EANCode", "HerstellerArtNr", "Hersteller", "Herstellerland", "Import_VKPreisEmpfehlung",
    "Zolltarifnummer_export", "Zolltarifnummer_import",
    "Import_VersicherungswertNetto", "Import_VersicherungswertBrutto",
    "TeamAgenda_Gruppe", "Proviklasse",
    "Breite_Netto", "Hoehe_Netto", "Tiefe_Netto", "Volumen_Netto", "Gewicht_Netto",
    "Breite_Brutto", "Hoehe_Brutto", "Tiefe_Brutto", "Volumen_Brutto", "Gewicht_Brutto",
    "BetriebsspannungVon", "BetriebsspannungBis", "Stromaufnahme", "Leistung",
    "Aufbauzeit", "Abbauzeit", "CheckOut_time", "CheckIn_time",
    "VDE_Schutzklasse", "VDECheck_Intervall", "VDE_test_time",
    "WM_FlagEinsatztage", "WM_maxEinsatztage", "WM_FlagWartung",
    "WM_FlagTage", "WM_FlagMonate", "WM_AnzahlTage", "WM_AnzahlMonate",
    "RolltagePreise_aktiv", "OffDayPreise_aktiv", "WEPreis_aktiv",
    "Verpackungseinheit", "RabattMOFF", "VersicherungMOFF", "NurMaterial", "Unikatartikel",
    "MieteForceSNScan", "MieteWiredBundle",
    "Sichtpruefung", "Funktionspruefung", "Verbrauchserfassung",
    "RabattVOFF", "KolliVielfaches", "KolliEinheit", "KolliGroesse", "VerkaufMindBestand",
    "audit_created_at", "audit_changed_at"
) VALUES (
    $1, $2, $3, $4, $5, $6,
    $7, $8, $9, $10, $11,
    $12, $13, $14, $15, $16,
    $17, $18, $19, $20, $21, $22, $23,
    $24, $25, $26, $27,
    $28, $29, $30, $31, $32,
    $33, $34, $35, $36, $37,
    $38, $39, $40, $41, $42, $43, $44, $45,
    $46, $47, $48,
    $49, $50, $51, $52, $53, $54, $55,
    $56, $57, $58, $59, $60, $61, $62, $63, $64, $65,
    $66, $67, $68,
    $69, $70, $71, $72, $73,
    $74, $75
)
RETURNING *"#;

const UPDATE_SQL: &str = r#"
UPDATE "Artikel" SET
    "ArtikelNr" = $2, "InventarNr" = $3, "Name" = $4, "Import_kurzname" = $5,
    "Matchcode" = $6, "Import_Einheit" = $7,
    "Aktiv" = $8, "Inventar_Flag" = $9, "ZurMiete_Flag" = $10, "ZumVerkauf_Flag" = $11,
    "Textsync_Flag" = $12,
    "Bestandskontrolle" = $13, "OhneEigenbestand" = $14, "SerienNr_Flag" = $15,
    "Dispokontrolle" = $16, "virtuellerArtikel" = $17,
    "EANCode" = $18, "HerstellerArtNr" = $19, "Hersteller" = $20, "Herstellerland" = $21,
    "Import_VKPreisEmpfehlung" = $22,
    "Zolltarifnummer_export" = $23, "Zolltarifnummer_import" = $24,
    "Import_VersicherungswertNetto" = $25, "Import_VersicherungswertBrutto" = $26,
    "TeamAgenda_Gruppe" = $27, "Proviklasse" = $28,
    "Breite_Netto" = $29, "Hoehe_Netto" = $30, "Tiefe_Netto" = $31,
    "Volumen_Netto" = $32, "Gewicht_Netto" = $33,
    "Breite_Brutto" = $34, "Hoehe_Brutto" = $35, "Tiefe_Brutto" = $36,
    "Volumen_Brutto" = $37, "Gewicht_Brutto" = $38,
    "BetriebsspannungVon" = $39, "BetriebsspannungBis" = $40,
    "Stromaufnahme" = $41, "Leistung" = $42,
    "Aufbauzeit" = $43, "Abbauzeit" = $44, "CheckOut_time" = $45, "CheckIn_time" = $46,
    "VDE_Schutzklasse" = $47, "VDECheck_Intervall" = $48, "VDE_test_time" = $49,
    "WM_FlagEinsatztage" = $50, "WM_maxEinsatztage" = $51, "WM_FlagWartung" = $52,
    "WM_FlagTage" = $53, "WM_FlagMonate" = $54,
    "WM_AnzahlTage" = $55, "WM_AnzahlMonate" = $56,
    "RolltagePreise_aktiv" = $57, "OffDayPreise_aktiv" = $58, "WEPreis_aktiv" = $59,
    "Verpackungseinheit" = $60, "RabattMOFF" = $61, "VersicherungMOFF" = $62,
    "NurMaterial" = $63, "Unikatartikel" = $64,
    "MieteForceSNScan" = $65, "MieteWiredBundle" = $66,
    "Sichtpruefung" = $67, "Funktionspruefung" = $68, "Verbrauchserfassung" = $69,
    "RabattVOFF" = $70, "KolliVielfaches" = $71,
    "KolliEinheit" = $72, "KolliGroesse" = $73, "VerkaufMindBestand" = $74,
    "audit_changed_at" = now()
 WHERE "Primary_Key" = $1
RETURNING *"#;

const DELETE_SQL: &str = r#"DELETE FROM "Artikel" WHERE "Primary_Key" = $1"#;
const TEXTS_SQL: &str = r#"
SELECT "Beschreibung_short", "Beschreibung_long", "Beschreibung_tech",
       "Beschreibung_delivery", "Beschreibung_package", "Beschreibung_webshop"
  FROM "Artikel_Beschreibung"
 WHERE "Artikel_Primkey" = $1
 ORDER BY "Muttersprache" DESC NULLS LAST, "Primary_Key"
 LIMIT 1"#;

const TEXTS_UPDATE_SQL: &str = r#"
UPDATE "Artikel_Beschreibung" SET
    "Artikelname" = $3, "Name_kurz" = $4,
    "Beschreibung_short" = $5, "Beschreibung_long" = $6, "Beschreibung_tech" = $7,
    "Beschreibung_delivery" = $8, "Beschreibung_package" = $9, "Beschreibung_webshop" = $10
 WHERE "Artikel_Primkey" = $1 AND "Sprache_Primkey" = $2"#;

const TEXTS_INSERT_SQL: &str = r#"
INSERT INTO "Artikel_Beschreibung" (
    "Artikel_Primkey", "Sprache_Primkey", "Artikelname", "Name_kurz",
    "Beschreibung_short", "Beschreibung_long", "Beschreibung_tech",
    "Beschreibung_delivery", "Beschreibung_package", "Beschreibung_webshop",
    "Muttersprache", "Position"
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true, 0)"#;

const ARTICLE_GROUPS_SQL: &str = r#"SELECT "Storage_Primkey" FROM "Artikel_Warengruppe" WHERE "Artikel_Primkey" = $1 ORDER BY 1"#;

const ARTICLE_GROUPS_DELETE_SQL: &str =
    r#"DELETE FROM "Artikel_Warengruppe" WHERE "Artikel_Primkey" = $1"#;

const ARTICLE_GROUPS_INSERT_SQL: &str = r#"
INSERT INTO "Artikel_Warengruppe" ("Artikel_Primkey", "Storage_Primkey", "Verkaufsartikel")
VALUES ($1, $2, false)"#;

/// Pyro register of the form: the `pyro_*` columns of `"Artikel"`, one row per article.
const PYRO_SAVE_SQL: &str = r#"
UPDATE "Artikel" SET
    "pyro_UN_number" = $2, "pyro_UN_description_o" = $3,
    "pyro_hazard_class_o" = $4, "pyro_caliber" = $5,
    "pyro_lateral_safety_distance" = $6, "pyro_vertical_safety_distance" = $7,
    "pyro_ascending_height" = $8, "pyro_sound_pressure_level" = $9,
    "pyro_sound_pressure_range" = $10, "pyro_tunnel_delimitation_code" = $11,
    "pyro_delay_time" = $12, "pyro_duration_time" = $13,
    "pyro_category" = $14, "pyro_bam_id" = $15, "pyro_categorisation" = $16,
    "pyro_packing_group" = $17, "pyro_approval_number" = $18,
    "pyro_neq" = $19, "pyro_neq_einheit" = $20, "pyro_UN_points" = $21,
    "pyro_transport_category" = $22,
    "audit_changed_at" = now()
 WHERE "Primary_Key" = $1"#;

/// Suppliers of an article, joined with the address for number and name.
const SUPPLIERS_SQL: &str = r#"
SELECT l."Lieferant_Primkey" AS supplier_id,
       a."lief_LieferantenNr" AS supplier_no,
       COALESCE(a."Name1", a."Name2", a."Name3") AS name,
       l."lfd_Pos" AS position,
       l."EK_Preis" AS purchase_price,
       l."Lieferzeit_Tage" AS delivery_days,
       l."Favorit" AS favorite,
       l."Verkaufsartikel" AS sales_article,
       l."ArtLiefNr" AS supplier_article_no
  FROM "Artikel_Lieferanten" l
  LEFT JOIN "Adresse" a ON a."Primary_key" = l."Lieferant_Primkey"
 WHERE l."Artikel_PrimKey" = $1
 ORDER BY l."lfd_Pos" NULLS LAST, l."Primary_Key""#;

const SUPPLIERS_DELETE_SQL: &str =
    r#"DELETE FROM "Artikel_Lieferanten" WHERE "Artikel_PrimKey" = $1"#;

/// `$1` article, `$2` supplier, `$3` position, `$4` purchase price, `$5` delivery days,
/// `$6` favourite, `$7` sales article, `$8` supplier's article number.
const SUPPLIERS_INSERT_SQL: &str = r#"
INSERT INTO "Artikel_Lieferanten"
    ("Artikel_PrimKey", "Lieferant_Primkey", "lfd_Pos", "EK_Preis",
     "Lieferzeit_Tage", "Favorit", "Verkaufsartikel", "ArtLiefNr",
     "last_Update_Date")
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, current_date)"#;

const LANGUAGES_SQL: &str = r#"
SELECT "Primary_Key", "Sprache", "Muttersprache", "ISO_639_1"
  FROM "Storage_Sprachen"
 ORDER BY "Muttersprache" DESC NULLS LAST, "Sprache""#;

const NATIVE_LANGUAGE_SQL: &str = r#"
SELECT "Primary_Key" FROM "Storage_Sprachen"
 ORDER BY "Muttersprache" DESC NULLS LAST, "Primary_Key" LIMIT 1"#;

/// Repository for the article master data.
#[derive(Clone)]
pub struct ArticleRepository {
    pool: PgPool,
}

impl ArticleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
        search: Option<&str>,
        group: Option<i32>,
        active: Option<bool>,
    ) -> Result<Page<ArticleSummaryRow>, DomainError> {
        let total: i64 = sqlx::query_scalar(COUNT_SQL)
            .bind(search)
            .bind(group)
            .bind(active)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;

        let rows = sqlx::query_as::<_, ArticleSummaryRow>(LIST_SQL)
            .bind(limit)
            .bind(offset)
            .bind(search)
            .bind(group)
            .bind(active)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;

        Ok(Page {
            items: rows,
            total,
            limit,
            offset,
        })
    }

    /// Complete article including text blocks and article groups.
    pub async fn get(&self, id: i32) -> Result<Article, DomainError> {
        let row = sqlx::query_as::<_, ArticleRow>(GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Article",
                id,
            })?;

        let mut article: Article = row.into();
        article.texts = self.load_texts(id).await?;
        article.article_groups = self.load_article_groups(id).await?;
        article.suppliers = self.load_suppliers(id).await?;
        self.load_price_data(id, &mut article.pricing).await?;
        Ok(article)
    }

    /// Suppliers of the article together with their address number and name.
    async fn load_suppliers(
        &self,
        id: i32,
    ) -> Result<Vec<where_domain::ArticleSupplier>, DomainError> {
        let rows = sqlx::query_as::<_, ArticleSupplierRow>(SUPPLIERS_SQL)
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Prices and accounts of the tabs `Miete` and `Verkauf`. Missing rows simply leave
    /// the empty defaults in place.
    async fn load_price_data(
        &self,
        id: i32,
        pricing: &mut where_domain::ArticlePricing,
    ) -> Result<(), DomainError> {
        let row = sqlx::query_as::<_, ArticlePriceRow>(PRICE_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if let Some(row) = row {
            row.apply(pricing);
        }
        Ok(())
    }

    /// Scales of the form: factor scales of the rental or quantity scales of the sale.
    pub async fn scale_categories(
        &self,
        for_sale: bool,
    ) -> Result<Vec<where_domain::ScaleCategory>, DomainError> {
        let rows = sqlx::query_as::<_, ScaleCategoryRow>(SCALE_CATEGORIES_SQL)
            .bind(for_sale)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Steps of one scale, ordered by the value they start at.
    pub async fn scale_steps(
        &self,
        category_id: i32,
    ) -> Result<Vec<where_domain::ScaleStep>, DomainError> {
        let rows = sqlx::query_as::<_, ScaleStepRow>(SCALE_STEPS_SQL)
            .bind(category_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Writes the prices into `"Art_MandantDaten"` — one row per article and client.
    async fn save_price_data(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i32,
        p: &where_domain::ArticlePricing,
    ) -> Result<(), DomainError> {
        let existing: Option<(i32,)> = sqlx::query_as(PRICE_ROW_SQL)
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db_error)?;

        let Some((row_id,)) = existing else {
            sqlx::query(PRICE_INSERT_SQL)
                .bind(id)
                .bind(p.rental_price_per_day)
                .bind(p.rental_base_price)
                .bind(p.deposit)
                .bind(p.rental_price_per_day_roll_days)
                .bind(p.rental_base_price_roll_days)
                .bind(p.rental_price_per_day_off_days)
                .bind(p.rental_base_price_off_days)
                .bind(p.rental_price_weekend)
                .bind(p.sale_price_net)
                .bind(p.sale_price_gross)
                .bind(p.sale_retail_price)
                .bind(p.capital_binding)
                .bind(p.sale_cost_price_net)
                .bind(p.rental_cost_price)
                .bind(p.rental_cost_price_roll_days)
                .bind(p.rental_cost_price_off_days)
                .bind(p.rental_cost_price_weekend)
                .bind(p.rental_vat_rate)
                .bind(p.sale_vat_rate)
                .bind(p.sale_markup_percent)
                .bind(p.rental_revenue_account.as_deref())
                .bind(p.sale_revenue_account.as_deref())
                .bind(p.rental_cost_center.as_deref())
                .bind(p.sale_cost_center.as_deref())
                .bind(p.factor_scale)
                .bind(p.factor_scale_roll_days)
                .bind(p.factor_scale_off_days)
                .bind(p.quantity_scale)
                .bind(p.used_sale_price)
                .bind(p.approximate_location.as_deref())
                .execute(&mut **tx)
                .await
                .map_err(db_error)?;
            return Ok(());
        };

        sqlx::query(PRICE_UPDATE_SQL)
            .bind(row_id)
            .bind(p.rental_price_per_day)
            .bind(p.rental_base_price)
            .bind(p.deposit)
            .bind(p.rental_price_per_day_roll_days)
            .bind(p.rental_base_price_roll_days)
            .bind(p.rental_price_per_day_off_days)
            .bind(p.rental_base_price_off_days)
            .bind(p.rental_price_weekend)
            .bind(p.sale_price_net)
            .bind(p.sale_price_gross)
            .bind(p.sale_retail_price)
            .bind(p.capital_binding)
            .bind(p.sale_cost_price_net)
            .bind(p.rental_cost_price)
            .bind(p.rental_cost_price_roll_days)
            .bind(p.rental_cost_price_off_days)
            .bind(p.rental_cost_price_weekend)
            .bind(p.rental_vat_rate)
            .bind(p.sale_vat_rate)
            .bind(p.sale_markup_percent)
            .bind(p.rental_revenue_account.as_deref())
            .bind(p.sale_revenue_account.as_deref())
            .bind(p.rental_cost_center.as_deref())
            .bind(p.sale_cost_center.as_deref())
            .bind(p.factor_scale)
            .bind(p.factor_scale_roll_days)
            .bind(p.factor_scale_off_days)
            .bind(p.quantity_scale)
            .bind(p.used_sale_price)
            .bind(p.approximate_location.as_deref())
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    async fn load_texts(&self, id: i32) -> Result<ArticleTexts, DomainError> {
        let row = sqlx::query_as::<_, ArticleTextRow>(TEXTS_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row.map(Into::into).unwrap_or_default())
    }

    async fn load_article_groups(&self, id: i32) -> Result<Vec<i32>, DomainError> {
        let rows: Vec<(i32,)> = sqlx::query_as(ARTICLE_GROUPS_SQL)
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(|(g,)| g).collect())
    }

    /// Creates an article. `article_no == 0` assigns the next number from the
    /// `artikel` number series; manually assigned numbers stay untouched.
    pub async fn create(&self, input: &Article) -> Result<Article, DomainError> {
        input.validate()?;

        let mut tx = self.pool.begin().await.map_err(db_error)?;

        let nr = if input.article_no > 0 {
            input.article_no
        } else {
            let year = Utc::now().year();
            sqlx::query(ALIGN_MAX_SQL)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            // The series can lag behind manually assigned numbers; then keep counting
            // until a free number is found.
            let mut candidate = None;
            for _ in 0..100 {
                let raw = crate::number_series_repo::reserve_on(&mut tx, "article", year)
                    .await
                    .map_err(db_error)?
                    .ok_or_else(|| {
                        DomainError::Conflict(
                            "Number series 'article' does not exist or is inactive".into(),
                        )
                    })?;
                let taken: Option<(i32,)> =
                    sqlx::query_as(r#"SELECT 1 FROM "Artikel" WHERE "ArtikelNr" = $1"#)
                        .bind(raw.value as i32)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(db_error)?;
                if taken.is_none() {
                    candidate = Some(raw.value as i32);
                    break;
                }
            }
            candidate.ok_or_else(|| {
                DomainError::Conflict("No free numbers left in the number series 'article'".into())
            })?
        };

        let now = Utc::now();
        let row = sqlx::query_as::<_, ArticleRow>(INSERT_SQL)
            .bind(nr)
            .bind(input.inventory_no.as_deref())
            .bind(input.name.trim())
            .bind(input.short_name.as_deref())
            .bind(input.match_code.as_deref())
            .bind(input.unit.as_deref())
            .bind(input.active)
            .bind(input.inventory)
            .bind(input.rental)
            .bind(input.sale)
            .bind(input.text_sync)
            .bind(input.stock_tracking)
            .bind(input.no_own_stock)
            .bind(input.individual_items)
            .bind(input.allocation_control)
            .bind(input.virtual_article)
            .bind(input.ean_code.as_deref())
            .bind(input.manufacturer_no.as_deref())
            .bind(input.manufacturer.as_deref())
            .bind(input.manufacturer_country.as_deref())
            .bind(input.retail_price)
            .bind(input.customs_tariff_export.as_deref())
            .bind(input.customs_tariff_import.as_deref())
            .bind(input.insurance_value_net)
            .bind(input.insurance_value_gross)
            .bind(input.team_group.as_deref())
            .bind(input.commission_class.as_deref())
            .bind(input.width_net)
            .bind(input.height_net)
            .bind(input.depth_net)
            .bind(input.volume_net)
            .bind(input.weight_net)
            .bind(input.width_gross)
            .bind(input.height_gross)
            .bind(input.depth_gross)
            .bind(input.volume_gross)
            .bind(input.weight_gross)
            .bind(input.voltage_from)
            .bind(input.voltage_to)
            .bind(input.current_draw)
            .bind(input.power)
            .bind(input.setup_minutes)
            .bind(input.teardown_minutes)
            .bind(input.check_out_seconds)
            .bind(input.check_in_seconds)
            .bind(input.inspection_class)
            .bind(input.inspection_interval_months)
            .bind(input.inspection_minutes)
            .bind(input.workshop_by_usage_days)
            .bind(input.workshop_max_usage_days)
            .bind(input.workshop_fixed_intervals)
            .bind(input.workshop_by_days)
            .bind(input.workshop_by_months)
            .bind(input.workshop_days)
            .bind(input.workshop_months)
            .bind(input.pricing.rental_roll_days_active)
            .bind(input.pricing.rental_off_days_active)
            .bind(input.pricing.rental_weekend_active)
            .bind(input.pricing.packaging_release)
            .bind(input.pricing.rental_no_discount)
            .bind(input.pricing.rental_not_insurable)
            .bind(input.pricing.material_only)
            .bind(input.pricing.unique_article)
            .bind(input.pricing.checkout_requires_serial)
            .bind(input.pricing.fixed_installation)
            .bind(input.pricing.inspection_on_checkin)
            .bind(input.pricing.function_check_on_checkin)
            .bind(input.pricing.consumption_recording)
            .bind(input.pricing.sale_no_discount)
            .bind(input.pricing.sell_full_packages)
            .bind(input.pricing.package_unit.as_deref())
            .bind(input.pricing.package_size)
            .bind(input.pricing.reorder_level)
            .bind(now)
            .bind(now)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;

        let id = row.id;
        Self::align_series(&mut tx, input.article_no).await?;
        Self::save_texts(&mut tx, id, input).await?;
        Self::save_article_groups(&mut tx, id, &input.article_groups).await?;
        Self::save_price_data(&mut tx, id, &input.pricing).await?;
        Self::save_pyro(&mut tx, id, &input.pyro).await?;
        Self::save_suppliers(&mut tx, id, &input.suppliers).await?;

        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(&self, id: i32, input: &Article) -> Result<Article, DomainError> {
        input.validate()?;

        let mut tx = self.pool.begin().await.map_err(db_error)?;

        let updated = sqlx::query_as::<_, ArticleRow>(UPDATE_SQL)
            .bind(id)
            .bind(input.article_no)
            .bind(input.inventory_no.as_deref())
            .bind(input.name.trim())
            .bind(input.short_name.as_deref())
            .bind(input.match_code.as_deref())
            .bind(input.unit.as_deref())
            .bind(input.active)
            .bind(input.inventory)
            .bind(input.rental)
            .bind(input.sale)
            .bind(input.text_sync)
            .bind(input.stock_tracking)
            .bind(input.no_own_stock)
            .bind(input.individual_items)
            .bind(input.allocation_control)
            .bind(input.virtual_article)
            .bind(input.ean_code.as_deref())
            .bind(input.manufacturer_no.as_deref())
            .bind(input.manufacturer.as_deref())
            .bind(input.manufacturer_country.as_deref())
            .bind(input.retail_price)
            .bind(input.customs_tariff_export.as_deref())
            .bind(input.customs_tariff_import.as_deref())
            .bind(input.insurance_value_net)
            .bind(input.insurance_value_gross)
            .bind(input.team_group.as_deref())
            .bind(input.commission_class.as_deref())
            .bind(input.width_net)
            .bind(input.height_net)
            .bind(input.depth_net)
            .bind(input.volume_net)
            .bind(input.weight_net)
            .bind(input.width_gross)
            .bind(input.height_gross)
            .bind(input.depth_gross)
            .bind(input.volume_gross)
            .bind(input.weight_gross)
            .bind(input.voltage_from)
            .bind(input.voltage_to)
            .bind(input.current_draw)
            .bind(input.power)
            .bind(input.setup_minutes)
            .bind(input.teardown_minutes)
            .bind(input.check_out_seconds)
            .bind(input.check_in_seconds)
            .bind(input.inspection_class)
            .bind(input.inspection_interval_months)
            .bind(input.inspection_minutes)
            .bind(input.workshop_by_usage_days)
            .bind(input.workshop_max_usage_days)
            .bind(input.workshop_fixed_intervals)
            .bind(input.workshop_by_days)
            .bind(input.workshop_by_months)
            .bind(input.workshop_days)
            .bind(input.workshop_months)
            .bind(input.pricing.rental_roll_days_active)
            .bind(input.pricing.rental_off_days_active)
            .bind(input.pricing.rental_weekend_active)
            .bind(input.pricing.packaging_release)
            .bind(input.pricing.rental_no_discount)
            .bind(input.pricing.rental_not_insurable)
            .bind(input.pricing.material_only)
            .bind(input.pricing.unique_article)
            .bind(input.pricing.checkout_requires_serial)
            .bind(input.pricing.fixed_installation)
            .bind(input.pricing.inspection_on_checkin)
            .bind(input.pricing.function_check_on_checkin)
            .bind(input.pricing.consumption_recording)
            .bind(input.pricing.sale_no_discount)
            .bind(input.pricing.sell_full_packages)
            .bind(input.pricing.package_unit.as_deref())
            .bind(input.pricing.package_size)
            .bind(input.pricing.reorder_level)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Article",
                id,
            })?;

        // Safety net against a shifted parameter list: the row must carry
        // the same number and name that we wrote.
        if updated.article_no != input.article_no {
            return Err(DomainError::Internal(
                "Article number was not persisted".into(),
            ));
        }

        Self::align_series(&mut tx, input.article_no).await?;
        Self::save_texts(&mut tx, id, input).await?;
        Self::save_article_groups(&mut tx, id, &input.article_groups).await?;
        Self::save_price_data(&mut tx, id, &input.pricing).await?;
        Self::save_pyro(&mut tx, id, &input.pyro).await?;
        Self::save_suppliers(&mut tx, id, &input.suppliers).await?;

        tx.commit().await.map_err(db_error)?;
        self.get(id).await
    }

    async fn save_pyro(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i32,
        p: &where_domain::ArticlePyro,
    ) -> Result<(), DomainError> {
        sqlx::query(PYRO_SAVE_SQL)
            .bind(id)
            .bind(p.un_number.as_deref())
            .bind(p.un_description.as_deref())
            .bind(p.hazard_class.as_deref())
            .bind(p.caliber)
            .bind(p.lateral_safety_distance)
            .bind(p.vertical_safety_distance)
            .bind(p.ascending_height)
            .bind(p.sound_pressure_level)
            .bind(p.sound_pressure_range)
            .bind(p.tunnel_restriction_code.as_deref())
            .bind(p.delay_time)
            .bind(p.effect_duration)
            .bind(p.category.as_deref())
            .bind(p.bam_id.as_deref())
            .bind(p.classification.as_deref())
            .bind(p.packing_group.as_deref())
            .bind(p.approval_number.as_deref())
            .bind(p.neq)
            .bind(p.neq_unit.as_deref())
            .bind(p.un_points)
            .bind(p.transport_category.as_deref())
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    /// Replaces the supplier list of the article (positions are renumbered from 1).
    async fn save_suppliers(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i32,
        suppliers: &[where_domain::ArticleSupplier],
    ) -> Result<(), DomainError> {
        sqlx::query(SUPPLIERS_DELETE_SQL)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        for (index, s) in suppliers.iter().enumerate() {
            sqlx::query(SUPPLIERS_INSERT_SQL)
                .bind(id)
                .bind(s.supplier_id)
                .bind(s.position.unwrap_or(index as i32 + 1))
                .bind(s.purchase_price)
                .bind(s.delivery_days)
                .bind(s.favorite)
                .bind(s.sales_article)
                .bind(s.supplier_article_no.as_deref())
                .execute(&mut **tx)
                .await
                .map_err(db_error)?;
        }
        Ok(())
    }

    /// Keeps the `article` series in step with a number that was sent by hand.
    async fn align_series(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        article_no: i32,
    ) -> Result<(), DomainError> {
        if article_no <= 0 {
            return Ok(());
        }
        crate::number_series_repo::align_on(tx, "article", article_no, Utc::now().year())
            .await
            .map_err(db_error)
    }

    async fn save_texts(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i32,
        input: &Article,
    ) -> Result<(), DomainError> {
        let language = sqlx::query_scalar::<_, i32>(NATIVE_LANGUAGE_SQL)
            .fetch_optional(&mut **tx)
            .await
            .map_err(db_error)?
            .ok_or_else(|| {
                DomainError::Conflict(
                    "no language present — was migration 1000_article_master.sql applied?".into(),
                )
            })?;

        let t = &input.texts;
        let affected = sqlx::query(TEXTS_UPDATE_SQL)
            .bind(id)
            .bind(language)
            .bind(input.name.trim())
            .bind(input.short_name.as_deref())
            .bind(t.short.as_deref())
            .bind(t.long.as_deref())
            .bind(t.technical_details.as_deref())
            .bind(t.scope_of_delivery.as_deref())
            .bind(t.packaging_info.as_deref())
            .bind(t.webshop.as_deref())
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;

        if affected.rows_affected() == 0 {
            sqlx::query(TEXTS_INSERT_SQL)
                .bind(id)
                .bind(language)
                .bind(input.name.trim())
                .bind(input.short_name.as_deref())
                .bind(t.short.as_deref())
                .bind(t.long.as_deref())
                .bind(t.technical_details.as_deref())
                .bind(t.scope_of_delivery.as_deref())
                .bind(t.packaging_info.as_deref())
                .bind(t.webshop.as_deref())
                .execute(&mut **tx)
                .await
                .map_err(db_error)?;
        }
        Ok(())
    }

    async fn save_article_groups(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: i32,
        groups: &[i32],
    ) -> Result<(), DomainError> {
        sqlx::query(ARTICLE_GROUPS_DELETE_SQL)
            .bind(id)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        for g in groups {
            sqlx::query(ARTICLE_GROUPS_INSERT_SQL)
                .bind(id)
                .bind(g)
                .execute(&mut **tx)
                .await
                .map_err(db_error)?;
        }
        Ok(())
    }

    /// Deletes an article together with its register rows.
    ///
    /// `"Artikel_Warengruppe"` and `"Artikel_Lieferanten"` carry no foreign key to
    /// `"Artikel"`, so the links would stay behind and keep counting as usage of the
    /// group or the supplier. Both are removed in the same transaction.
    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(SUPPLIERS_DELETE_SQL)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        sqlx::query(ARTICLE_GROUPS_DELETE_SQL)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        let result = sqlx::query(DELETE_SQL)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        if result.rows_affected() == 0 {
            tx.rollback().await.map_err(db_error)?;
            return Err(DomainError::NotFound {
                entity: "Article",
                id,
            });
        }
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    /// Next free article number without saving (for the form "new").
    pub async fn next_number(&self) -> Result<i32, DomainError> {
        let series = NumberSeriesRepository::new(self.pool.clone())
            .get("article")
            .await?;
        let today = Utc::now().date_naive();
        let value = series
            .next_value(today)
            .ok_or_else(|| DomainError::Conflict("Number series 'article' is inactive".into()))?;
        Ok(value as i32)
    }

    /// Languages for the language switcher ("German" in the form).
    pub async fn languages(&self) -> Result<Vec<LanguageRow>, DomainError> {
        sqlx::query_as::<_, LanguageRow>(LANGUAGES_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }
}
