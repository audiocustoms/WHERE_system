//! The lists of the admin menu (`img/Admin Menü`), each one a flat legacy table.
//!
//! Every statement is built from the catalogue in this file: table and column names
//! come from code, never from a request, and values are bound as text and cast to
//! the column type. That keeps one implementation for eighteen masks without
//! letting a caller pick a table.

use sqlx::{PgPool, Row};
use where_domain::{
    DomainError, FieldKind, PrefValues, Prefs, SettingsArea, SettingsField, SettingsInput,
    SettingsList, SettingsRow, SettingsValues,
};

use crate::error::db_error;

const TEXT: FieldKind = FieldKind::Text;
const INTEGER: FieldKind = FieldKind::Integer;
const NUMBER: FieldKind = FieldKind::Number;
const BOOL: FieldKind = FieldKind::Bool;
const DATE: FieldKind = FieldKind::Date;

/// One column of a list.
fn field(column: &str, label: &str, kind: FieldKind) -> SettingsField {
    SettingsField {
        column: column.to_string(),
        label: label.to_string(),
        kind,
    }
}

/// One list: key, title, menu group, legacy table, description, fields.
fn area(
    key: &str,
    title: &str,
    group: &str,
    table: &str,
    description: &str,
    fields: Vec<SettingsField>,
) -> SettingsArea {
    let name_column = fields
        .first()
        .map(|field| field.column.clone())
        .unwrap_or_default();
    SettingsArea {
        key: key.to_string(),
        title: title.to_string(),
        group: group.to_string(),
        table: table.to_string(),
        name_column,
        description: description.to_string(),
        fields,
    }
}

/// Marks a column other than the first one as the name of an entry.
fn named(mut area: SettingsArea, column: &str) -> SettingsArea {
    area.name_column = column.to_string();
    area
}

/// Every list the admin menu of WHERE can show, in the order of the menu.
///
/// The first field is the name of an entry — it is used as its title and is
/// required for a new row. Tables that the original only reads (a warehouse that
/// belongs to a client, bank data) keep the columns it shows.
pub fn areas() -> Vec<SettingsArea> {
    vec![
        // ------------------------------------------------------------ Group "Adressen"
        named(
            area(
                "anreden",
                "Anreden & Anredeformen",
                "Adressen",
                "Prefs_SexSalutation",
                "Anreden je Sprache mit Höflichkeitsform, Geschlecht und „per Du“.",
                vec![
                    field("Type", "Höflichkeitsform", INTEGER),
                    field("Geschlecht", "Geschlecht", INTEGER),
                    field("Persoenlich", "Per Du", BOOL),
                    field("Anrede", "Anrede", TEXT),
                    field("Sprachen_ID", "Sprache", INTEGER),
                    field("Position", "Position", INTEGER),
                ],
            ),
            "Anrede",
        ),
        area(
            "anreden-kurz",
            "Anreden (Kurzliste)",
            "Adressen",
            "Anrede",
            "Anredeformen der Adressmasken (`Anrede` in `\"Adresse\"`).",
            vec![field("Anrede", "Anrede", TEXT)],
        ),
        area(
            "adressgruppen",
            "Adressgruppen",
            "Adressen",
            "AdressGruppe",
            "Gruppen der Adresssuche; `Type` trennt Kunden, Lieferanten und Interessenten.",
            vec![field("Name", "Gruppe", TEXT), field("Type", "Art", INTEGER)],
        ),
        area(
            "qualifikationen",
            "Mitarbeiterqualifikationen",
            "Adressen",
            "Storage_MAQualifikation",
            "Qualifikationen der Mitarbeiter; die Adressmaske zieht sie als Auswahlliste.",
            vec![field("Qualifikation", "Qualifikation", TEXT)],
        ),
        area(
            "status-location",
            "Status Location",
            "Adressen",
            "Storage_ELStatus",
            "Status eines Veranstaltungsorts (`Adresse : Location`).",
            vec![field("Status", "Status", TEXT)],
        ),
        area(
            "kategorie-location",
            "Kategorie Location",
            "Adressen",
            "Storage_ELKategorie",
            "Kategorie eines Veranstaltungsorts; sie ordnet die Locations in der Suche.",
            vec![field("Kategorie", "Kategorie", TEXT)],
        ),
        area(
            "stromanschluesse",
            "Stromanschlüsse",
            "Adressen",
            "Storage_ELStromplan",
            "Anschlusstypen, die an einer Location verfügbar sind.",
            vec![
                field("Anschlusstyp", "Anschluss", TEXT),
                field("Position", "Position", INTEGER),
            ],
        ),
        area(
            "merkmale-location",
            "Merkmale",
            "Adressen",
            "Storage_ELAttribute",
            "Merkmale einer Location, etwa Traversenpunkte oder Zufahrt.",
            vec![field("Attribut", "Merkmal", TEXT)],
        ),
        // -------------------------------------------------------------- Group "Artikel"
        area(
            "hersteller",
            "Hersteller",
            "Artikel",
            "Hersteller",
            "Hersteller der Artikelmaske.",
            vec![field("Hersteller", "Hersteller", TEXT)],
        ),
        area(
            "eigentuemer",
            "Eigentümer",
            "Artikel",
            "Eigentuemer",
            "Eigentümer der Artikelmaske (Mietartikel von Kunden).",
            vec![field("Eigentuemer", "Eigentümer", TEXT)],
        ),
        area(
            "artikelzustand",
            "Artikelzustand",
            "Artikel",
            "Artikel_Zustand",
            "Zustand, den die Gerätewartung am Gerät festhält.",
            vec![field("Zustand", "Zustand", TEXT)],
        ),
        area(
            "provisionsklassen",
            "Artikelseitige Provisionsklassen",
            "Artikel",
            "Storage_Proviklasse",
            "Provisionsklassen eines Artikels; `Artikel_Proviklasse` verknüpft sie mit dem Artikel.",
            vec![
                field("Index", "Bezeichnung", TEXT),
                field("Parent_ID", "Obergruppe", INTEGER),
                field("Child_ID", "Untergruppe", INTEGER),
            ],
        ),
        area(
            "artikel-suchindex",
            "Suchindex",
            "Artikel",
            "Storage_ArtikelSearchIndex",
            "Suchbegriffe für die Artikelrecherche; `Pfad` hält die Position im Baum.",
            vec![
                field("Index", "Suchbegriff", TEXT),
                field("Pfad", "Pfad", TEXT),
                field("Parent_ID", "Obergruppe", INTEGER),
                field("Child_ID", "Untergruppe", INTEGER),
            ],
        ),
        area(
            "preisklassen",
            "Preisklassen",
            "Artikel",
            "Preisklassen",
            "Preisklassen mit Kondition und Gültigkeit (`Preisklasse festlegen`).",
            vec![
                field("Name", "Bezeichnung", TEXT),
                field("KundenGruppe", "Kundengruppe", TEXT),
                field("ArtikelWG_Primkey", "Warengruppe", INTEGER),
                field("Verkaufsartikel", "Verkaufsartikel", BOOL),
                field("CalcType", "Berechnung", INTEGER),
                field("MargeProzent", "Marge in Prozent", NUMBER),
                field("MargeFest", "Faktor", NUMBER),
                field("PreisNetto", "Festpreis netto", NUMBER),
                field("Von", "gültig von", DATE),
                field("Bis", "gültig bis", DATE),
                field("Kunden_Auswahl", "Kundenauswahl", INTEGER),
                field("Artikel_Auswahl", "Artikelauswahl", INTEGER),
            ],
        ),
        // ---------------------------------------------------------- Group "Lager"
        area(
            "lager",
            "Lager",
            "Lager",
            "Storage_Lager",
            "Lager des Betriebs. Der Lagerbestand, die Entnahme und die Geräte hängen daran.",
            vec![
                field("Name", "Lager", TEXT),
                field("__DefaultLager", "Vorgabe", BOOL),
                field("SymetrischeLagerortung", "Symmetrische Lagerortung", BOOL),
                field("VordefinierteLagerortung", "Vordefinierte Lagerorte", BOOL),
                field("LiefAdress_Primkey", "Lieferadresse", INTEGER),
            ],
        ),
        area(
            "lagerplaetze",
            "Lagerplätze",
            "Lager",
            "Lagerortung",
            "Lagerplätze je Lager (`Lagerplatz (Symetrisch)` im Original).",
            vec![
                field("Name", "Lagerplatz", TEXT),
                field("Prefix", "Kürzel", TEXT),
                field("Lager_Primkey", "Lager", INTEGER),
                field("Position", "Position", INTEGER),
                field("Wertebereich", "Werte", TEXT),
            ],
        ),
        // -------------------------------------------------------- Group "Einstellungen"
        area(
            "sprachen",
            "Sprachen",
            "System",
            "Storage_Sprachen",
            "Sprachen der Artikeltexte und Standardtexte.",
            vec![
                field("Sprache", "Sprache", TEXT),
                field("ISO_639_1", "ISO 639-1", TEXT),
                field("ISO_639_2", "ISO 639-2", TEXT),
                field("Muttersprache", "Muttersprache", BOOL),
            ],
        ),
        area(
            "waehrungen",
            "Währungen",
            "System",
            "Storage_Waehrungen",
            "Währungen mit Kurs und Zahlenformat. Die `Waehrungs_ID` führt die Altanwendung getrennt vom Schlüssel; sie ist Pflicht und meist gleich dem Schlüssel.",
            vec![
                field("Waehrung_Kurzform", "Kürzel", TEXT),
                field("Waehrung_Langform", "Bezeichnung", TEXT),
                field("Wechselkurs_o", "Kurs", NUMBER),
                field("Waehrungs_ID", "Währungs-ID", INTEGER),
            ],
        ),
        area(
            "versandarten",
            "Versandarten",
            "System",
            "Storage_Versandart",
            "Versandarten der Aufträge und Packscheine.",
            vec![field("Versandart", "Versandart", TEXT)],
        ),
        area(
            "status",
            "Status",
            "System",
            "Storage_Status",
            "Statuslisten der Belege; `Type` wählt die Liste (Angebot/Auftrag, Rechnung, Packschein, Bestellung).",
            vec![
                field("Status", "Status", TEXT),
                field("Type", "Liste", INTEGER),
                field("Sort_ID", "Sortierung", INTEGER),
            ],
        ),
        // -------------------------------------------------------- Group "Buchhaltung"
        area(
            "banken",
            "Banken (SWIFT/BIC)",
            "Buchhaltung",
            "Storage_Banken",
            "Bankenliste für die Bankverbindungen der Adressen.",
            vec![
                field("Name", "Bank", TEXT),
                field("BIC", "BIC", TEXT),
                field("BLZ", "BLZ", TEXT),
                field("Ort", "Ort", TEXT),
                field("Land_ISO3", "Land", TEXT),
            ],
        ),
        area(
            "fibu-konten",
            "FiBu-Konten",
            "Buchhaltung",
            "FIBU_Konten",
            "Konten für FiBu und DATEV-Export.",
            vec![
                field("KontoNr", "Kontonummer", TEXT),
                field("Bezeichnung", "Bezeichnung", TEXT),
                field("Konto_Typ", "Kontoart", INTEGER),
                field("Datev_Automatikkonto", "DATEV-Automatikkonto", BOOL),
                field("Aktiv", "aktiv", BOOL),
            ],
        ),
        area(
            "zahlungskonditionen",
            "Zahlungskonditionen",
            "Buchhaltung",
            "Zahlungskondition",
            "Zahlungsziel, Vorkasse und Skonto der Belege.",
            vec![
                field("Text_ANMV", "Bezeichnung", TEXT),
                field("Zahlungsziel", "Zahlungsziel in Tagen", INTEGER),
                field("Vorkasse_Prozent", "Vorkasse in Prozent", INTEGER),
                field("Skonto1_Tage", "Skonto 1: Tage", INTEGER),
                field("Skonto1_Rabatt", "Skonto 1: Rabatt", NUMBER),
                field("SystemDefault", "Vorgabe", BOOL),
            ],
        ),
        // ------------------------------------------------------------- Group "Projekt"
        area(
            "projekte",
            "Projekte",
            "Projekt",
            "Projekt",
            "Projektstamm für die Zuordnung von Aufträgen und Zeiten.",
            vec![
                field("Name", "Name", TEXT),
                field("Name_kurz", "Kurzname", TEXT),
                field("Von_Date", "von", DATE),
                field("Bis_Date", "bis", DATE),
                field("Beschreibung", "Beschreibung", TEXT),
            ],
        ),
        // -------------------------------------------------------- Group "Administration"
        area(
            "mandanten",
            "Mandanten",
            "Administration",
            "Mandanten",
            "Mandanten des Betriebs. Ein Mandant bündelt Lager, Adressen und Belege.",
            vec![
                field("Mandanten_Name", "Mandant", TEXT),
                field("Shortname", "Kurzname", TEXT),
                field("Parent", "Obermandant", INTEGER),
            ],
        ),
    ]
}

/// Column sorted by inside the group: the existing `Position`, otherwise the name
/// of an entry.
fn order_column(area: &SettingsArea) -> &str {
    area.field("Position")
        .map(|field| field.column.as_str())
        .unwrap_or(&area.name_column)
}

/// Column that identifies a row. Every list is keyed by `"Primary_Key"`; the
/// currencies additionally demand a `"Waehrungs_ID"`, which the mask shows as a field.
fn id_column() -> &'static str {
    "Primary_Key"
}

fn kind_sql(kind: FieldKind) -> &'static str {
    match kind {
        FieldKind::Text => "text",
        FieldKind::Integer => "integer",
        FieldKind::Number => "numeric",
        FieldKind::Bool => "boolean",
        FieldKind::Date => "date",
    }
}

#[derive(Clone)]
pub struct SettingsRepository {
    pool: PgPool,
}

impl SettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn areas(&self) -> Vec<SettingsArea> {
        areas()
    }

    pub fn area(&self, key: &str) -> Result<SettingsArea, DomainError> {
        areas()
            .into_iter()
            .find(|area| area.key == key)
            .ok_or_else(|| {
                DomainError::Validation(format!("Die Liste „{key}\u{201c} gibt es nicht."))
            })
    }

    /// All rows of a list. Values are read as text, so the view does not need one
    /// type per column.
    pub async fn rows(&self, area: &SettingsArea) -> Result<Vec<SettingsRow>, DomainError> {
        let columns = area
            .fields
            .iter()
            .map(|field| format!("\"{}\"::text AS \"{}\"", field.column, field.column))
            .collect::<Vec<_>>()
            .join(", ");
        // Sorting: first column (the groups of the list), then `Position`, otherwise
        // the name — that way a salutation list keeps the order of the template.
        let sql = format!(
            "SELECT \"{}\"::bigint AS id, {columns} FROM \"{}\" ORDER BY \"{}\" NULLS LAST, \"{}\" NULLS LAST",
            id_column(),
            area.table,
            area.fields[0].column,
            order_column(area)
        );
        let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let mut values = SettingsValues::new();
                for field in &area.fields {
                    values.insert(
                        field.column.clone(),
                        row.get::<Option<String>, _>(&*field.column),
                    );
                }
                SettingsRow {
                    id: row.get::<i64, _>("id"),
                    values,
                }
            })
            .collect())
    }

    pub async fn list(&self, key: &str) -> Result<SettingsList, DomainError> {
        let area = self.area(key)?;
        let rows = self.rows(&area).await?;
        Ok(SettingsList { area, rows })
    }

    pub async fn create(
        &self,
        key: &str,
        input: SettingsInput,
    ) -> Result<SettingsRow, DomainError> {
        let area = self.area(key)?;
        let values = input.prepare(&area)?;
        let name = area.name_field().map(|field| field.column.clone());
        if let Some(column) = name
            && values.get(&column).is_none_or(Option::is_none)
        {
            return Err(DomainError::Validation(format!(
                "Für „{}\u{201c} fehlt die Bezeichnung.",
                area.title
            )));
        }
        let columns = area
            .fields
            .iter()
            .map(|field| format!("\"{}\"", field.column))
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = area
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| format!("CAST(${} AS {})", index + 1, kind_sql(field.kind)))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "INSERT INTO \"{}\" ({columns}) VALUES ({placeholders}) RETURNING \"{}\"::bigint AS id",
            area.table,
            id_column()
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.clone()));
        for field in &area.fields {
            query = query.bind(values.get(&field.column).cloned().flatten());
        }
        let row = query.fetch_one(&self.pool).await.map_err(db_error)?;
        Ok(SettingsRow {
            id: row.get::<i64, _>("id"),
            values,
        })
    }

    pub async fn update(
        &self,
        key: &str,
        id: i64,
        input: SettingsInput,
    ) -> Result<SettingsRow, DomainError> {
        let area = self.area(key)?;
        let values = input.prepare(&area)?;
        let assignments = area
            .fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                format!(
                    "\"{}\" = CAST(${} AS {})",
                    field.column,
                    index + 2,
                    kind_sql(field.kind)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "UPDATE \"{}\" SET {assignments} WHERE \"{}\" = $1",
            area.table,
            id_column()
        );
        let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.clone())).bind(id);
        for field in &area.fields {
            query = query.bind(values.get(&field.column).cloned().flatten());
        }
        let changed = query
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Eintrag",
                id: id as i32,
            });
        }
        Ok(SettingsRow { id, values })
    }

    /// Removes the entry. The legacy schema has no foreign keys, so a reference that
    /// still exists in an article or an address keeps its number.
    pub async fn delete(&self, key: &str, id: i64) -> Result<(), DomainError> {
        let area = self.area(key)?;
        let sql = format!(
            "DELETE FROM \"{}\" WHERE \"{}\" = $1",
            area.table,
            id_column()
        );
        let changed = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Eintrag",
                id: id as i32,
            });
        }
        Ok(())
    }

    /// Options of the WHERE masks; `prefix` selects one form (`address.`).
    pub async fn prefs(&self, prefix: Option<&str>) -> Result<Prefs, DomainError> {
        let rows = sqlx::query(
            "SELECT \"key\", \"value\" FROM \"where_settings\" \
             WHERE $1::text IS NULL OR \"key\" LIKE $1 || '%' ORDER BY \"key\"",
        )
        .bind(prefix)
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        let mut values = PrefValues::new();
        for row in rows {
            let key: String = row.get("key");
            let value: Option<String> = row.get("value");
            values.insert(key, value.unwrap_or_default());
        }
        Ok(Prefs { values })
    }

    /// Stores one option; an empty value removes it, so the form falls back to its
    /// default.
    pub async fn save_pref(
        &self,
        key: &str,
        value: Option<&str>,
        user: &str,
    ) -> Result<(), DomainError> {
        let key = key.trim();
        where_domain::settings::check_pref_key(key)?;
        let value = value.map(str::trim).filter(|text| !text.is_empty());
        match value {
            Some(value) => {
                sqlx::query(
                    "INSERT INTO \"where_settings\" (\"key\", \"value\", \"changed_at\", \"changed_by\") \
                     VALUES ($1, $2, now(), $3) \
                     ON CONFLICT (\"key\") DO UPDATE SET \"value\" = $2, \"changed_at\" = now(), \"changed_by\" = $3",
                )
                .bind(key)
                .bind(value)
                .bind(user)
                .execute(&self.pool)
                .await
                .map_err(db_error)?;
            }
            None => {
                sqlx::query("DELETE FROM \"where_settings\" WHERE \"key\" = $1")
                    .bind(key)
                    .execute(&self.pool)
                    .await
                    .map_err(db_error)?;
            }
        }
        Ok(())
    }
}

/// For the unit test below: the SQL of an area as the repository builds it.
#[cfg(test)]
fn build_statements(area: &SettingsArea) -> (String, String) {
    let columns = area
        .fields
        .iter()
        .map(|field| format!("\"{}\"::text AS \"{}\"", field.column, field.column))
        .collect::<Vec<_>>()
        .join(", ");
    let select = format!(
        "SELECT \"{}\" AS id, {columns} FROM \"{}\"",
        id_column(),
        area.table
    );
    let placeholders = area
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| format!("CAST(${} AS {})", index + 1, kind_sql(field.kind)))
        .collect::<Vec<_>>()
        .join(", ");
    let insert = format!("INSERT INTO \"{}\" VALUES ({placeholders})", area.table);
    (select, insert)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn keys_and_columns_are_unique() {
        let mut keys = HashSet::new();
        let mut tables = HashSet::new();
        for area in areas() {
            assert!(
                keys.insert(area.key.clone()),
                "doppelter Schlüssel {}",
                area.key
            );
            assert!(
                tables.insert(area.table.clone()),
                "doppelte Tabelle {}",
                area.table
            );
            let mut columns = HashSet::new();
            for field in &area.fields {
                assert!(
                    columns.insert(field.column.clone()),
                    "doppelte Spalte {} in {}",
                    field.column,
                    area.key
                );
                assert!(!field.label.trim().is_empty());
            }
        }
    }

    #[test]
    fn statements_only_use_quoted_identifiers() {
        for area in areas() {
            let (select, insert) = build_statements(&area);
            assert!(select.contains(&format!("FROM \"{}\"", area.table)));
            assert!(insert.contains(&format!("INTO \"{}\"", area.table)));
            assert!(!select.contains('\'') && !insert.contains('\''));
        }
    }

    #[test]
    fn every_area_has_a_name_field() {
        for area in areas() {
            assert!(
                area.name_field().is_some(),
                "{} ohne Namensspalte",
                area.key
            );
        }
    }

    #[test]
    fn input_drops_unknown_columns() {
        let area = areas().into_iter().find(|a| a.key == "anreden").unwrap();
        let mut values = SettingsValues::new();
        values.insert("Anrede".into(), Some("  Frau  ".into()));
        values.insert("Geloescht".into(), Some("x".into()));
        let prepared = SettingsInput { values }.prepare(&area);
        assert!(prepared.is_err());
    }

    #[test]
    fn input_trims_and_validates() {
        let area = areas().into_iter().find(|a| a.key == "lager").unwrap();
        let mut values = SettingsValues::new();
        values.insert("Name".into(), Some("  Technik  ".into()));
        values.insert("__DefaultLager".into(), Some("ja".into()));
        assert!(SettingsInput { values }.prepare(&area).is_err());

        let mut values = SettingsValues::new();
        values.insert("Name".into(), Some("  Technik  ".into()));
        values.insert("__DefaultLager".into(), Some("true".into()));
        let prepared = SettingsInput { values }.prepare(&area).unwrap();
        assert_eq!(prepared["Name"].as_deref(), Some("Technik"));
        assert_eq!(prepared["__DefaultLager"].as_deref(), Some("true"));
    }

    #[test]
    fn numbers_accept_a_comma() {
        let area = areas()
            .into_iter()
            .find(|a| a.key == "preisklassen")
            .unwrap();
        let mut values = SettingsValues::new();
        values.insert("Name".into(), Some("Bühne".into()));
        values.insert("MargeProzent".into(), Some("12,5".into()));
        let prepared = SettingsInput { values }.prepare(&area).unwrap();
        assert_eq!(prepared["MargeProzent"].as_deref(), Some("12.5"));
    }

    #[test]
    fn dates_must_be_iso() {
        assert!(chrono::NaiveDate::parse_from_str("13.11.2026", "%Y-%m-%d").is_err());
        let area = areas().into_iter().find(|a| a.key == "projekte").unwrap();
        let mut values = SettingsValues::new();
        values.insert("Name".into(), Some("Umbau".into()));
        values.insert("Von_Date".into(), Some("13.11.2026".into()));
        assert!(SettingsInput { values }.prepare(&area).is_err());
    }
}
