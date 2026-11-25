//! Settings of the order mask (`img/Admin Menü/Aufträge`).
//!
//! WHERE has no order engine yet. The tabs of the "Auftragseinstellungen" window
//! describe what a later module will read: the switches and numbers of `Regeln` and
//! `Zeiten` live in `"where_settings"` (prefix `order.`), the `Zeitplan`, the
//! `Überschriften` and the `Makros` are lists and have tables of their own.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use where_domain::DomainError;

/// Longest text of a label; the window shows them in one line.
pub const MAX_LABEL: usize = 120;

/// Trims a label and rejects an empty or overlong one.
fn check_label(what: &str, text: &str) -> Result<String, DomainError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(DomainError::Validation(format!(
            "Für „{what}\u{201c} fehlt eine Bezeichnung."
        )));
    }
    if text.chars().count() > MAX_LABEL {
        return Err(DomainError::Validation(format!(
            "„{what}\u{201c} ist länger als {MAX_LABEL} Zeichen."
        )));
    }
    Ok(text.to_string())
}

/// One step of the default time plan ("Zeitplan" tab).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimelineEntry {
    /// Stable key, e.g. `dispo` — the later order module refers to it.
    pub key: String,
    pub label: String,
    pub position: i32,
    /// The "Anzeigen" column: shown in a new order by default.
    pub shown: bool,
}

/// What the mask sends for one step of the time plan.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct TimelineInput {
    /// `None` keeps the stored name.
    #[serde(default)]
    pub label: Option<String>,
    pub shown: bool,
}

impl TimelineInput {
    /// Trims the name; an empty one is refused, because the list has no unnamed row.
    pub fn prepare(self) -> Result<(Option<String>, bool), DomainError> {
        let label = match self.label {
            Some(label) if label.trim().is_empty() => None,
            Some(label) => Some(check_label("Timeline", &label)?),
            None => None,
        };
        Ok((label, self.shown))
    }
}

/// One of the nine headings of the printed order.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeadingEntry {
    /// 1 to 9, as in the window.
    pub number: i32,
    pub shown: bool,
    /// The text behind "Überschrift n"; empty means the heading stays hidden.
    pub label: Option<String>,
}

/// What the mask sends for one heading.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct HeadingInput {
    pub shown: bool,
    #[serde(default)]
    pub label: Option<String>,
}

impl HeadingInput {
    /// An empty text removes the heading text — the row keeps its number.
    pub fn prepare(self) -> Result<(bool, Option<String>), DomainError> {
        let label = match self.label {
            Some(label) if label.trim().is_empty() => None,
            Some(label) => Some(check_label("Überschrift", &label)?),
            None => None,
        };
        Ok((self.shown, label))
    }
}

/// One entry of the macro list.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrderMacro {
    pub id: i64,
    pub label: String,
    /// "Verkauf": the macro belongs to a sale rather than a rental.
    pub for_sale: bool,
    pub active: bool,
    pub created_by: Option<String>,
    /// The steps of the macro; the order module will interpret them.
    pub body: Option<String>,
    pub changed_at: Option<DateTime<Utc>>,
}

/// What the mask sends for a macro.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct MacroInput {
    pub label: String,
    #[serde(default)]
    pub for_sale: bool,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub body: Option<String>,
}

fn default_true() -> bool {
    true
}

impl MacroInput {
    pub fn prepare(self) -> Result<(String, bool, bool, Option<String>), DomainError> {
        let label = check_label("Bezeichnung", &self.label)?;
        let body = self
            .body
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty());
        Ok((label, self.for_sale, self.active, body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_label_is_trimmed_and_optional() {
        let input = TimelineInput {
            label: Some("  Packen  ".into()),
            shown: false,
        };
        let (label, shown) = input.prepare().unwrap();
        assert_eq!(label.as_deref(), Some("Packen"));
        assert!(!shown);

        let input = TimelineInput {
            label: Some("   ".into()),
            shown: true,
        };
        assert_eq!(input.prepare().unwrap().0, None);
    }

    #[test]
    fn headings_accept_an_empty_text() {
        let input = HeadingInput {
            shown: true,
            label: Some("  ".into()),
        };
        assert_eq!(input.prepare().unwrap(), (true, None));

        let input = HeadingInput {
            shown: false,
            label: Some(" Bühnenaufbau ".into()),
        };
        assert_eq!(
            input.prepare().unwrap(),
            (false, Some("Bühnenaufbau".to_string()))
        );
    }

    #[test]
    fn a_macro_needs_a_label() {
        let ohne = MacroInput {
            label: "  ".into(),
            for_sale: false,
            active: true,
            body: None,
        };
        assert!(ohne.prepare().is_err());

        let lang = MacroInput {
            label: "x".repeat(MAX_LABEL + 1),
            for_sale: false,
            active: true,
            body: None,
        };
        assert!(lang.prepare().is_err());

        let gut = MacroInput {
            label: "  Aufbau planen ".into(),
            for_sale: true,
            active: false,
            body: Some("  Schritt 1  ".into()),
        };
        let (label, for_sale, active, body) = gut.prepare().unwrap();
        assert_eq!(label, "Aufbau planen");
        assert!(for_sale);
        assert!(!active);
        assert_eq!(body.as_deref(), Some("Schritt 1"));
    }
}
