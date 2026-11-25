//! Public and school holidays of the tenant.
//!
//! The duty plan needs them for two things: `holiday_work_allowed` can only be
//! checked against a calendar, and the school holidays tell the planner when
//! vacation is asked for. Both come from the OpenHolidays API, which needs no
//! key and no manual address — the settings only say *what* is imported and for
//! which federal state.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

/// Name of the source that is written next to every imported row.
pub const SOURCE: &str = "openholidaysapi.org";

/// A holiday that applies to the whole country.
pub const SCOPE_NATIONAL: &str = "national";
/// A holiday that only applies to one federal state.
pub const SCOPE_REGIONAL: &str = "regional";

/// The 16 federal states with their code and their German name.
pub const STATES: [(&str, &str); 16] = [
    ("BW", "Baden-Württemberg"),
    ("BY", "Bayern"),
    ("BE", "Berlin"),
    ("BB", "Brandenburg"),
    ("HB", "Bremen"),
    ("HH", "Hamburg"),
    ("HE", "Hessen"),
    ("MV", "Mecklenburg-Vorpommern"),
    ("NI", "Niedersachsen"),
    ("NW", "Nordrhein-Westfalen"),
    ("RP", "Rheinland-Pfalz"),
    ("SL", "Saarland"),
    ("SN", "Sachsen"),
    ("ST", "Sachsen-Anhalt"),
    ("SH", "Schleswig-Holstein"),
    ("TH", "Thüringen"),
];

/// Whether `code` is one of the 16 federal states.
pub fn is_state(code: &str) -> bool {
    STATES.iter().any(|(key, _)| *key == code)
}

/// The German name of a federal state.
pub fn state_name(code: &str) -> Option<&'static str> {
    STATES
        .iter()
        .find(|(key, _)| *key == code)
        .map(|(_, name)| *name)
}

/// One public holiday.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Holiday {
    pub day: NaiveDate,
    pub name: String,
    /// `national` or `regional`.
    pub scope: String,
    /// Federal state of a regional holiday, `None` for a nationwide one.
    pub state: Option<String>,
}

/// One stretch of school holidays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SchoolHoliday {
    #[serde(rename = "starts_on")]
    pub starts_on: NaiveDate,
    #[serde(rename = "ends_on")]
    pub ends_on: NaiveDate,
    pub name: String,
    pub state: String,
}

/// What one import run did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HolidayImportReport {
    /// The year the rows belong to.
    pub year: i32,
    /// The federal state that was asked for, `None` when only nationwide
    /// holidays were imported.
    pub state: Option<String>,
    pub national_holidays: usize,
    pub regional_holidays: usize,
    pub school_holidays: usize,
    /// Sources that answered with something unusable; the rest was stored.
    pub errors: Vec<String>,
}

/// Both lists as the board and the settings read them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HolidayCalendar {
    pub holidays: Vec<Holiday>,
    pub school_holidays: Vec<SchoolHoliday>,
}

/// How many rows the import has stored so far.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct HolidayCounts {
    pub holidays: i64,
    pub school_holidays: i64,
}

/// One entry of the OpenHolidays answer — the fields the import uses.
#[derive(Debug, Deserialize)]
struct RawEntry {
    #[serde(rename = "startDate")]
    start_date: NaiveDate,
    #[serde(rename = "endDate")]
    end_date: NaiveDate,
    #[serde(default)]
    name: Vec<RawName>,
    #[serde(default)]
    nationwide: bool,
}

#[derive(Debug, Deserialize)]
struct RawName {
    #[serde(default)]
    language: String,
    text: String,
}

/// The German name, otherwise the first one the source offers.
fn german_name(names: &[RawName]) -> Option<String> {
    let chosen = names
        .iter()
        .find(|name| name.language.eq_ignore_ascii_case("DE"))
        .or_else(|| names.first())?;
    let text = chosen.text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Reads the `PublicHolidays` answer. Entries the source marks as nationwide
/// become `national`; the others belong to `state`.
pub fn parse_public(json: &str, state: Option<&str>) -> Result<Vec<Holiday>, DomainError> {
    let entries: Vec<RawEntry> = serde_json::from_str(json).map_err(|error| {
        DomainError::Upstream(format!(
            "PublicHolidays: the answer cannot be read ({error})."
        ))
    })?;
    let mut holidays = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(name) = german_name(&entry.name) else {
            continue;
        };
        holidays.push(if entry.nationwide {
            Holiday {
                day: entry.start_date,
                name,
                scope: SCOPE_NATIONAL.to_string(),
                state: None,
            }
        } else {
            Holiday {
                day: entry.start_date,
                name,
                scope: SCOPE_REGIONAL.to_string(),
                state: state.map(str::to_string),
            }
        });
    }
    holidays.sort_by(|left, right| left.day.cmp(&right.day).then(left.name.cmp(&right.name)));
    holidays.dedup_by(|left, right| left.day == right.day && left.name == right.name);
    Ok(holidays)
}

/// Reads the `SchoolHolidays` answer of one federal state.
pub fn parse_school(json: &str, state: &str) -> Result<Vec<SchoolHoliday>, DomainError> {
    let entries: Vec<RawEntry> = serde_json::from_str(json).map_err(|error| {
        DomainError::Upstream(format!(
            "SchoolHolidays: the answer cannot be read ({error})."
        ))
    })?;
    let mut holidays = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(name) = german_name(&entry.name) else {
            continue;
        };
        if entry.end_date < entry.start_date {
            continue;
        }
        holidays.push(SchoolHoliday {
            starts_on: entry.start_date,
            ends_on: entry.end_date,
            name,
            state: state.to_string(),
        });
    }
    holidays.sort_by(|left, right| {
        left.starts_on
            .cmp(&right.starts_on)
            .then(left.name.cmp(&right.name))
    });
    holidays.dedup_by(|left, right| {
        left.starts_on == right.starts_on
            && left.ends_on == right.ends_on
            && left.name == right.name
    });
    Ok(holidays)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An excerpt of `PublicHolidays?countryIsoCode=DE&subdivisionCode=DE-BY`,
    /// fetched from the source on 2026-11-20.
    const PUBLIC: &str = r#"[
        {"id":"1","startDate":"2026-01-01","endDate":"2026-01-01","type":"Public",
         "name":[{"language":"DE","text":"Neujahr"}],"nationwide":true},
        {"id":"2","startDate":"2026-01-06","endDate":"2026-01-06","type":"Public",
         "name":[{"language":"DE","text":"Heilige Drei Könige"}],"nationwide":false,
         "subdivisions":[{"code":"DE-BY","shortName":"BY"}]},
        {"id":"3","startDate":"2026-12-25","endDate":"2026-12-25","type":"Public",
         "name":[{"language":"EN","text":"Christmas Day"}],"nationwide":true}
    ]"#;

    /// An excerpt of `SchoolHolidays?countryIsoCode=DE&subdivisionCode=DE-BY`.
    const SCHOOL: &str = r#"[
        {"id":"a","startDate":"2026-02-16","endDate":"2026-02-20","type":"School",
         "name":[{"language":"DE","text":"Frühjahrsferien"}],
         "subdivisions":[{"code":"DE-BY","shortName":"BY"}]},
        {"id":"b","startDate":"2025-12-22","endDate":"2026-01-05","type":"School",
         "name":[{"language":"DE","text":"Weihnachtsferien"}],
         "subdivisions":[{"code":"DE-BY","shortName":"BY"}]}
    ]"#;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn the_nationwide_and_the_regional_holidays_are_told_apart() {
        let holidays = parse_public(PUBLIC, Some("BY")).unwrap();
        assert_eq!(holidays.len(), 3);
        assert_eq!(holidays[0].name, "Neujahr");
        assert_eq!(holidays[0].scope, SCOPE_NATIONAL);
        assert_eq!(holidays[0].state, None);
        assert_eq!(holidays[1].name, "Heilige Drei Könige");
        assert_eq!(holidays[1].scope, SCOPE_REGIONAL);
        assert_eq!(holidays[1].state.as_deref(), Some("BY"));
    }

    #[test]
    fn a_holiday_without_german_name_takes_the_first_name() {
        let holidays = parse_public(PUBLIC, None).unwrap();
        assert_eq!(holidays[2].name, "Christmas Day");
        assert_eq!(holidays[2].scope, SCOPE_NATIONAL);
    }

    #[test]
    fn the_school_holidays_keep_their_stretch() {
        let holidays = parse_school(SCHOOL, "BY").unwrap();
        assert_eq!(holidays.len(), 2);
        assert_eq!(holidays[0].name, "Weihnachtsferien");
        assert_eq!(holidays[0].starts_on, date("2025-12-22"));
        assert_eq!(holidays[0].ends_on, date("2026-01-05"));
        assert_eq!(holidays[0].state, "BY");
        assert_eq!(holidays[1].name, "Frühjahrsferien");
    }

    #[test]
    fn an_answer_that_is_not_a_list_is_reported() {
        assert!(parse_public("{\"error\":\"nope\"}", None).is_err());
        assert!(parse_school("[]", "BY").unwrap().is_empty());
    }

    #[test]
    fn the_federal_states_are_known_by_their_codes() {
        assert!(is_state("BY"));
        assert!(is_state("TH"));
        assert!(!is_state("DE"));
        assert!(!is_state("by"));
        assert_eq!(state_name("BW"), Some("Baden-Württemberg"));
        assert_eq!(STATES.len(), 16);
    }
}
