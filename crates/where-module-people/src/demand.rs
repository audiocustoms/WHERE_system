//! Staffing demand of the duty plan: how many employees of a position a shift
//! needs on a day.
//!
//! The board opens a date and enters a number per position and shift. A pair
//! without a number is not needed; the demand can be copied onto following days.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

/// Most employees one position can need in a shift.
pub const MAX_NEEDED: i32 = 99;
/// Longest position name that fits the dialog.
pub const MAX_POSITION_LEN: usize = 100;
/// Longest location name that fits the dialog.
pub const MAX_LOCATION_LEN: usize = 100;
/// A demand row that does not care which position covers it.
pub const ANY_POSITION: &str = "*";

/// The position a house needs when it is run by a foreman.
pub const FOREMAN_POSITION: &str = "Meister";

/// `true` when the position names the foreman; a house that needs no foreman
/// never plans one.
pub fn is_foreman(position: &str) -> bool {
    position.trim().eq_ignore_ascii_case(FOREMAN_POSITION)
}

/// How many employees of a position a shift needs on a day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DemandEntry {
    pub shift_id: i64,
    /// Position of the personnel master data, or `*` when the position does not
    /// matter and any employee of the shift covers the row.
    pub position: String,
    /// Location of the disposition the row belongs to; empty when the demand is
    /// not bound to one. Two events in two houses therefore stay two rows.
    #[serde(default)]
    pub location: String,
    pub needed: i32,
}

/// The demand of one day, ordered by shift id and position.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DayDemand {
    pub day: NaiveDate,
    /// Only pairs that need somebody.
    pub entries: Vec<DemandEntry>,
}

/// What the dialog sends for one day.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DemandInput {
    /// Pairs without a number are dropped, so sending the whole list is safe.
    #[serde(default)]
    pub entries: Vec<DemandEntryInput>,
}

/// One position of one shift in the dialog.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DemandEntryInput {
    pub shift_id: i64,
    /// Position of the personnel master data, or [`ANY_POSITION`] when the
    /// position does not matter.
    pub position: String,
    /// Location of the disposition, empty when the row is not bound to one.
    #[serde(default)]
    pub location: String,
    #[serde(default)]
    pub needed: i32,
}

/// Copies the demand of `from` onto the given days.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DemandCopyInput {
    pub from: NaiveDate,
    /// Target days; `from` itself and duplicates are ignored.
    #[serde(default)]
    pub to: Vec<NaiveDate>,
}

/// A checked demand for the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanDemandEntry {
    pub shift_id: i64,
    pub position: String,
    pub location: String,
    pub needed: i32,
}

/// Result of a copy: number of written days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DemandCopied {
    pub days: usize,
}

impl DemandInput {
    /// Trims the names, keeps one value per shift and position and drops the zeros.
    pub fn clean(&self) -> Result<Vec<CleanDemandEntry>, DomainError> {
        let mut clean: Vec<CleanDemandEntry> = Vec::new();
        for entry in &self.entries {
            if entry.shift_id <= 0 {
                return Err(DomainError::Validation("The shift is missing.".to_string()));
            }
            let position = entry.position.trim();
            if position.is_empty() {
                return Err(DomainError::Validation(
                    "The position is missing.".to_string(),
                ));
            }
            if position.chars().count() > MAX_POSITION_LEN {
                return Err(DomainError::Validation(format!(
                    "The position is longer than {MAX_POSITION_LEN} characters."
                )));
            }
            if !(0..=MAX_NEEDED).contains(&entry.needed) {
                return Err(DomainError::Validation(format!(
                    "The number of employees has to lie between 0 and {MAX_NEEDED}."
                )));
            }
            let location = entry.location.trim();
            if location.chars().count() > MAX_LOCATION_LEN {
                return Err(DomainError::Validation(format!(
                    "The location is longer than {MAX_LOCATION_LEN} characters."
                )));
            }
            match clean.iter_mut().find(|known| {
                known.shift_id == entry.shift_id
                    && known.position.eq_ignore_ascii_case(position)
                    && known.location.eq_ignore_ascii_case(location)
            }) {
                Some(known) => {
                    known.position = position.to_string();
                    known.location = location.to_string();
                    known.needed = entry.needed;
                }
                None => clean.push(CleanDemandEntry {
                    shift_id: entry.shift_id,
                    position: position.to_string(),
                    location: location.to_string(),
                    needed: entry.needed,
                }),
            }
        }
        clean.retain(|entry| entry.needed > 0);
        Ok(clean)
    }
}

impl DemandCopyInput {
    /// The target days without the source, duplicates removed.
    pub fn clean(&self) -> Result<Vec<NaiveDate>, DomainError> {
        let mut days: Vec<NaiveDate> = Vec::new();
        for day in &self.to {
            if *day == self.from || days.contains(day) {
                continue;
            }
            days.push(*day);
        }
        let max = crate::duty::MAX_RANGE_DAYS as usize;
        if days.len() > max {
            return Err(DomainError::Validation(format!(
                "At most {max} days can be filled at once."
            )));
        }
        Ok(days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn input(entries: &[(i64, &str, i32)]) -> DemandInput {
        DemandInput {
            entries: entries
                .iter()
                .map(|(shift_id, position, needed)| DemandEntryInput {
                    shift_id: *shift_id,
                    position: (*position).to_string(),
                    location: String::new(),
                    needed: *needed,
                })
                .collect(),
        }
    }

    #[test]
    fn names_are_trimmed_and_zeros_dropped() {
        let clean = input(&[(3, "  Stellwerker  ", 2), (3, "Meister", 0)])
            .clean()
            .unwrap();
        assert_eq!(
            clean,
            vec![CleanDemandEntry {
                shift_id: 3,
                position: "Stellwerker".to_string(),
                location: String::new(),
                needed: 2
            }]
        );
    }

    #[test]
    fn the_same_position_of_two_shifts_stays_two_rows() {
        let clean = input(&[(1, "Meister", 1), (2, "Meister", 3)])
            .clean()
            .unwrap();
        assert_eq!(clean.len(), 2);
        assert_eq!(clean[0].shift_id, 1);
        assert_eq!(clean[1].shift_id, 2);
    }

    #[test]
    fn a_pair_is_kept_once_with_its_last_value() {
        let clean = input(&[(1, "Meister", 1), (1, "meister", 4)])
            .clean()
            .unwrap();
        assert_eq!(
            clean,
            vec![CleanDemandEntry {
                shift_id: 1,
                position: "meister".to_string(),
                location: String::new(),
                needed: 4
            }]
        );
    }

    #[test]
    fn the_same_position_of_two_locations_stays_two_rows() {
        let clean = DemandInput {
            entries: vec![
                DemandEntryInput {
                    shift_id: 1,
                    position: "Meister".to_string(),
                    location: "Großes Haus".to_string(),
                    needed: 1,
                },
                DemandEntryInput {
                    shift_id: 1,
                    position: "Meister".to_string(),
                    location: " Podium ".to_string(),
                    needed: 1,
                },
            ],
        }
        .clean()
        .unwrap();
        assert_eq!(clean.len(), 2);
        assert_eq!(clean[0].location, "Großes Haus");
        assert_eq!(clean[1].location, "Podium");
    }

    #[test]
    fn numbers_and_names_are_checked() {
        assert!(input(&[(1, "Meister", -1)]).clean().is_err());
        assert!(input(&[(1, "Meister", MAX_NEEDED + 1)]).clean().is_err());
        assert!(input(&[(1, "Meister", MAX_NEEDED)]).clean().is_ok());
        assert!(input(&[(0, "Meister", 1)]).clean().is_err());
        assert!(input(&[(1, "   ", 1)]).clean().is_err());
        let long = "x".repeat(MAX_POSITION_LEN + 1);
        assert!(input(&[(1, &long, 1)]).clean().is_err());
    }

    #[test]
    fn an_open_position_is_star() {
        let clean = input(&[(1, " * ", 2), (1, "Meister", 1)]).clean().unwrap();
        assert_eq!(clean.len(), 2);
        assert_eq!(clean[0].position, ANY_POSITION);
        assert_eq!(clean[0].needed, 2);
        // Only one open row per shift, like every other position.
        let doppelt = input(&[(1, "*", 2), (1, "*", 3)]).clean().unwrap();
        assert_eq!(doppelt.len(), 1);
        assert_eq!(doppelt[0].needed, 3);
    }

    #[test]
    fn a_copy_skips_the_source_and_duplicates() {
        let copy = DemandCopyInput {
            from: date("2026-11-20"),
            to: vec![
                date("2026-11-20"),
                date("2026-11-21"),
                date("2026-11-21"),
                date("2026-11-22"),
            ],
        };
        let days = copy.clean().unwrap();
        assert_eq!(days, vec![date("2026-11-21"), date("2026-11-22")]);
    }

    #[test]
    fn a_copy_stays_below_the_limit() {
        let start = date("2026-01-01");
        let mut to = Vec::new();
        for offset in 0..=(crate::duty::MAX_RANGE_DAYS + 1) {
            to.push(start + chrono::Days::new(offset as u64));
        }
        let copy = DemandCopyInput {
            from: date("2025-12-31"),
            to,
        };
        assert!(copy.clean().is_err());
    }
}
