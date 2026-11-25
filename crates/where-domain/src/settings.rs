//! Settings lists of the admin menu (`img/Admin Menü`).
//!
//! The original keeps its flat master data in small tables: salutations, address
//! groups, banks, currencies, warehouses. The masks behind them are all the same
//! shape — a list, sometimes with a tab strip — so WHERE describes each list once
//! in [`areas`] and renders it with one page.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::DomainError;

/// How a value is read from the database and written back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Text,
    Integer,
    Number,
    Bool,
    Date,
}

/// One column of a list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SettingsField {
    /// Column of the legacy table — at the same time the key in `values`.
    pub column: String,
    pub label: String,
    pub kind: FieldKind,
}

impl SettingsField {
    /// Checks a value of a new or changed row. Everything arrives as text; the kind
    /// decides what it has to look like before it reaches the database.
    fn check(&self, value: &Option<String>) -> Result<(), DomainError> {
        let Some(text) = value.as_deref() else {
            return Ok(());
        };
        match self.kind {
            FieldKind::Text => Ok(()),
            FieldKind::Integer => text
                .parse::<i64>()
                .map(|_| ())
                .map_err(|_| invalid(&self.label, "eine ganze Zahl erwartet")),
            FieldKind::Number => text
                .replace(',', ".")
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| invalid(&self.label, "eine Zahl erwartet")),
            FieldKind::Bool => match text {
                "true" | "false" => Ok(()),
                _ => Err(invalid(&self.label, "Ja oder Nein erwartet")),
            },
            FieldKind::Date => chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .map(|_| ())
                .map_err(|_| invalid(&self.label, "ein Datum JJJJ-MM-TT erwartet")),
        }
    }
}

fn invalid(label: &str, expected: &str) -> DomainError {
    DomainError::Validation(format!("„{label}\u{201c}: {expected}."))
}

/// A list of the admin menu.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SettingsArea {
    /// Address of the list in the API and in the page.
    pub key: String,
    pub title: String,
    /// Group of the admin menu the list belongs to (`Adressen`, `Artikel`, …).
    pub group: String,
    /// Legacy table, kept visible so the source of a value stays traceable.
    pub table: String,
    /// Column that names an entry — usually the first one, but not always
    /// (`Prefs_SexSalutation` leads with the salutation form).
    pub name_column: String,
    pub description: String,
    pub fields: Vec<SettingsField>,
}

impl SettingsArea {
    /// Field with this column, `None` when the list does not have it.
    pub fn field(&self, column: &str) -> Option<&SettingsField> {
        self.fields.iter().find(|field| field.column == column)
    }

    /// Field that names an entry — shown as its title and required for a new row.
    pub fn name_field(&self) -> Option<&SettingsField> {
        self.field(&self.name_column)
            .or_else(|| self.fields.first())
    }
}

/// Values of one row, keyed by column. Everything travels as text; the kind of the
/// field says how the view renders and validates it.
pub type SettingsValues = BTreeMap<String, Option<String>>;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SettingsRow {
    pub id: i64,
    pub values: SettingsValues,
}

/// A list with its rows — what the page draws.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SettingsList {
    pub area: SettingsArea,
    pub rows: Vec<SettingsRow>,
}

/// What the mask sends for a new or changed row.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SettingsInput {
    #[serde(default)]
    pub values: SettingsValues,
}
impl SettingsInput {
    /// Trims, drops empty text and checks the shape per field. Columns the area does
    /// not know are rejected instead of silently ignored.
    pub fn prepare(mut self, area: &SettingsArea) -> Result<SettingsValues, DomainError> {
        for column in self.values.keys() {
            if area.field(column).is_none() {
                return Err(DomainError::Validation(format!(
                    "Die Liste „{}\u{201c} kennt die Spalte „{column}\u{201c} nicht.",
                    area.title
                )));
            }
        }
        let mut values = SettingsValues::new();
        for field in &area.fields {
            let raw = self.values.remove(&field.column).flatten();
            let value = match raw {
                Some(text) => {
                    let text = text.trim().to_string();
                    if text.is_empty() {
                        None
                    } else if field.kind == FieldKind::Number {
                        Some(text.replace(',', "."))
                    } else {
                        Some(text)
                    }
                }
                None => None,
            };
            field.check(&value)?;
            values.insert(field.column.clone(), value);
        }
        if values.values().all(Option::is_none) {
            return Err(DomainError::Validation(format!(
                "Für „{}\u{201c} fehlt eine Eingabe.",
                area.title
            )));
        }
        Ok(values)
    }
}

/// Options of the WHERE masks (`"where_settings"`): the checkboxes and small entries
/// of a form, keyed by form and option (`address.brief.anrede`).
pub type PrefValues = BTreeMap<String, String>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Prefs {
    #[serde(default)]
    pub values: PrefValues,
}

/// What a form sends for one option.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PrefInput {
    /// `None` or an empty text removes the option — the form then uses its default.
    #[serde(default)]
    pub value: Option<String>,
}

/// Keys are lower case and may carry dots: `address.neukunde.indexpflicht`.
pub fn check_pref_key(key: &str) -> Result<(), DomainError> {
    let erlaubt =
        |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-');
    if key.is_empty() || key.len() > 120 || !key.chars().all(erlaubt) {
        return Err(DomainError::Validation(format!(
            "„{key}\u{201c} ist kein gültiger Schlüssel (klein, Ziffern, Punkt, _ und -)."
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pref_keys_are_checked() {
        assert!(check_pref_key("address.neukunde.indexpflicht").is_ok());
        assert!(check_pref_key("karten.maps.start").is_ok());
        assert!(check_pref_key("Address.Ort").is_err());
        assert!(check_pref_key("").is_err());
        assert!(check_pref_key("a b").is_err());
        assert!(check_pref_key(&"x".repeat(121)).is_err());
    }
}
