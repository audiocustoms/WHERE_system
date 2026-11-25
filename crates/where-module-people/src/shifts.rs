//! Shifts of the duty planning.
//!
//! A shift is a time window with a break. Several shifts per day are allowed;
//! if the end lies before the start, the shift runs past midnight.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

/// Longest name of a shift.
pub const MAX_NAME: usize = 100;
/// Longest short name of a shift.
pub const MAX_SHORT_NAME: usize = 10;
/// Most positions one shift may exclude.
pub const MAX_EXCLUDED_POSITIONS: usize = 50;

/// Half of the day a shift belongs to.
///
/// The derivation of the demand needs it: an event of the disposition belongs to
/// the `early` or the `late` half of its day. `intermediate` are the shifts in
/// between — an emergency means for the days where neither half fits, never a
/// target of the derivation. `free` is no work at all: a whole free day as a
/// visible marker in the plan, without times, without hours in the actual time and
/// never asking for anybody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ShiftBand {
    Early,
    Late,
    Intermediate,
    Free,
}

impl Default for ShiftBand {
    fn default() -> Self {
        ShiftBand::Early
    }
}

impl ShiftBand {
    /// The value as it stands in the database.
    pub fn as_str(self) -> &'static str {
        match self {
            ShiftBand::Early => "early",
            ShiftBand::Late => "late",
            ShiftBand::Intermediate => "intermediate",
            ShiftBand::Free => "free",
        }
    }

    /// `true` for the band of a whole free day.
    pub fn is_free(self) -> bool {
        self == ShiftBand::Free
    }

    /// Reads the value of the database; an unknown value counts as `early`.
    pub fn parse(text: &str) -> Self {
        match text.trim().to_ascii_lowercase().as_str() {
            "late" => ShiftBand::Late,
            "intermediate" => ShiftBand::Intermediate,
            "free" => ShiftBand::Free,
            _ => ShiftBand::Early,
        }
    }
}

/// Whether a shift carries the weekend marker in its short name or its name
/// (`S-WE`, `S-WE-AZ`). The theatre marks its weekend shifts that way: `WE` as a
/// word of its own, so `Wochenende` in the name does not count.
pub fn weekend_marker(short_name: Option<&str>, name: &str) -> bool {
    short_name
        .into_iter()
        .chain(std::iter::once(name))
        .any(|text| {
            text.split(|zeichen: char| !zeichen.is_alphanumeric())
                .any(|teil| teil.eq_ignore_ascii_case("WE"))
        })
}

/// Whether a shift may be worked on Saturday and Sunday.
///
/// The regular late shift (`band` `spaet` without the weekend marker) may not;
/// the early shifts, the Zwischenschichten and the weekend late shifts may. Free
/// shifts are no work and are not part of that question.
pub fn works_weekend(band: ShiftBand, short_name: Option<&str>, name: &str) -> bool {
    band != ShiftBand::Late || weekend_marker(short_name, name)
}

impl Shift {
    /// Whether the shift may be worked on Saturday and Sunday.
    pub fn works_weekend(&self) -> bool {
        works_weekend(self.band, self.short_name.as_deref(), &self.name)
    }
}

/// A shift as the mask shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Shift {
    pub id: i64,
    pub name: String,
    pub short_name: Option<String>,
    /// Start as `HH:MM`.
    pub starts_at: String,
    /// End as `HH:MM`; read before the start, the shift runs past midnight.
    pub ends_at: String,
    /// Break in minutes that falls into the shift.
    pub break_minutes: i16,
    /// Half of the day the shift belongs to.
    #[serde(default = "default_band")]
    pub band: ShiftBand,
    /// Only apprentices may be planned into this shift.
    #[serde(default)]
    pub apprentice_only: bool,
    /// Colour in the plan as `#RRGGBB`.
    pub color: Option<String>,
    /// Department of the personnel master data that owns the shift; empty when the
    /// shift belongs to no particular one and every dispatcher may give it.
    #[serde(default)]
    pub department: Option<String>,
    pub active: bool,
    pub sort_order: i32,
    /// Positions of the personnel master data that do not occur in this shift.
    #[serde(default)]
    pub excluded_positions: Vec<String>,
}

/// What the mask sends to change the excluded positions of a shift.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ShiftExclusionsInput {
    #[serde(default)]
    pub positions: Vec<String>,
}

impl ShiftExclusionsInput {
    /// Trims the names, keeps each one once and refuses the open position.
    pub fn clean(&self) -> Result<Vec<String>, DomainError> {
        let mut clean: Vec<String> = Vec::new();
        for position in &self.positions {
            let name = position.trim();
            if name.is_empty() {
                continue;
            }
            if name == crate::demand::ANY_POSITION {
                return Err(DomainError::Validation(
                    "The open position cannot be excluded.".to_string(),
                ));
            }
            if name.chars().count() > crate::demand::MAX_POSITION_LEN {
                return Err(DomainError::Validation(format!(
                    "The position must not be longer than {} characters.",
                    crate::demand::MAX_POSITION_LEN
                )));
            }
            if !clean.iter().any(|known| known.eq_ignore_ascii_case(name)) {
                clean.push(name.to_string());
            }
        }
        if clean.len() > MAX_EXCLUDED_POSITIONS {
            return Err(DomainError::Validation(format!(
                "A shift excludes at most {MAX_EXCLUDED_POSITIONS} positions."
            )));
        }
        Ok(clean)
    }
}

/// What the mask sends to create and change a shift.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ShiftInput {
    pub name: String,
    #[serde(default)]
    pub short_name: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    #[serde(default)]
    pub break_minutes: i16,
    /// Defaults to `frueh` when the mask sends nothing.
    #[serde(default)]
    pub band: Option<ShiftBand>,
    #[serde(default)]
    pub apprentice_only: bool,
    #[serde(default)]
    pub color: Option<String>,
    /// Department the shift belongs to; empty for everybody.
    #[serde(default)]
    pub department: Option<String>,
    #[serde(default = "crate::employee::active_default")]
    pub active: bool,
}

/// A checked shift for the database — names trimmed, times `HH:MM`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanShift {
    pub name: String,
    pub short_name: Option<String>,
    pub starts_at: String,
    pub ends_at: String,
    pub break_minutes: i16,
    pub band: ShiftBand,
    pub apprentice_only: bool,
    pub color: Option<String>,
    pub department: Option<String>,
    pub active: bool,
}

/// The band used when the mask does not name one.
pub fn default_band() -> ShiftBand {
    ShiftBand::Early
}

/// Reads `7:00`, `07:00` or `07:00:00` and returns `07:00`.
pub fn clock(text: &str) -> Option<String> {
    let mut parts = text.trim().split(':');
    let hours: u32 = parts.next()?.trim().parse().ok()?;
    let minutes: u32 = parts.next()?.trim().parse().ok()?;
    let seconds: u32 = match parts.next() {
        Some(rest) => rest.trim().parse().ok()?,
        None => 0,
    };
    if parts.next().is_some() || hours > 23 || minutes > 59 || seconds > 59 {
        return None;
    }
    Some(format!("{hours:02}:{minutes:02}"))
}

/// Duration of the shift in minutes; an end before the start counts past midnight.
pub fn duration_minutes(starts_at: &str, ends_at: &str) -> Option<i32> {
    let minutes = |text: &str| -> Option<i32> {
        let clock = clock(text)?;
        let (hours, minutes) = clock.split_once(':')?;
        Some(hours.parse::<i32>().ok()? * 60 + minutes.parse::<i32>().ok()?)
    };
    let start = minutes(starts_at)?;
    let end = minutes(ends_at)?;
    Some(if end > start {
        end - start
    } else {
        24 * 60 - start + end
    })
}

impl ShiftInput {
    /// Checks the input of the mask and returns the version for the database.
    pub fn clean(&self) -> Result<CleanShift, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "The name of the shift must not be empty.".to_string(),
            ));
        }
        if name.chars().count() > MAX_NAME {
            return Err(DomainError::Validation(format!(
                "The name must not be longer than {MAX_NAME} characters."
            )));
        }
        let short_name = self
            .short_name
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        if let Some(short) = &short_name
            && short.chars().count() > MAX_SHORT_NAME
        {
            return Err(DomainError::Validation(format!(
                "The short name must not be longer than {MAX_SHORT_NAME} characters."
            )));
        }
        let color = match self.color.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(text) if is_color(text) => Some(text.to_ascii_uppercase()),
            Some(_) => {
                return Err(DomainError::Validation(
                    "The colour is given as #RRGGBB.".to_string(),
                ));
            }
        };
        let band = self.band.unwrap_or_default();
        let department = self
            .department
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_string);
        if let Some(name) = &department
            && name.chars().count() > MAX_NAME
        {
            return Err(DomainError::Validation(format!(
                "The department must not be longer than {MAX_NAME} characters."
            )));
        }
        // A free day is no work: it keeps no times, no break and takes nobody.
        if band.is_free() {
            return Ok(CleanShift {
                name: name.to_string(),
                short_name,
                starts_at: FREE_DAY_START.to_string(),
                ends_at: FREE_DAY_END.to_string(),
                break_minutes: 0,
                band,
                apprentice_only: false,
                color,
                department,
                active: self.active,
            });
        }
        let starts_at = clock(&self.starts_at)
            .ok_or_else(|| DomainError::Validation("The start is not a time.".to_string()))?;
        let ends_at = clock(&self.ends_at)
            .ok_or_else(|| DomainError::Validation("The end is not a time.".to_string()))?;
        if starts_at == ends_at {
            return Err(DomainError::Validation(
                "The start and the end of the shift are equal.".to_string(),
            ));
        }
        if !(0..=720).contains(&self.break_minutes) {
            return Err(DomainError::Validation(
                "The break is between 0 and 720 minutes.".to_string(),
            ));
        }
        let duration = duration_minutes(&starts_at, &ends_at).unwrap_or_default();
        if i32::from(self.break_minutes) >= duration {
            return Err(DomainError::Validation(
                "The break is longer than the shift.".to_string(),
            ));
        }
        Ok(CleanShift {
            name: name.to_string(),
            short_name,
            starts_at,
            ends_at,
            break_minutes: self.break_minutes,
            band,
            apprentice_only: self.apprentice_only,
            color,
            department,
            active: self.active,
        })
    }
}

/// The window a free day is stored with. It covers the whole day; the hours are
/// never counted, so the exact end only satisfies the clock constraints.
pub const FREE_DAY_START: &str = "00:00";
pub const FREE_DAY_END: &str = "23:59";

pub(crate) fn is_color(text: &str) -> bool {
    text.len() == 7 && text.starts_with('#') && text[1..].chars().all(|c| c.is_ascii_hexdigit())
}
#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ShiftInput {
        ShiftInput {
            name: "Frühschicht".into(),
            short_name: Some("F".into()),
            starts_at: "06:00".into(),
            ends_at: "14:30".into(),
            break_minutes: 30,
            band: None,
            apprentice_only: false,
            color: Some("#1f6feb".into()),
            department: None,
            active: true,
        }
    }

    #[test]
    fn clocks_are_read_in_every_usual_form() {
        assert_eq!(clock("7:00").as_deref(), Some("07:00"));
        assert_eq!(clock(" 07:05 ").as_deref(), Some("07:05"));
        assert_eq!(clock("07:05:59").as_deref(), Some("07:05"));
        assert_eq!(clock("24:00"), None);
        assert_eq!(clock("07:60"), None);
        assert_eq!(clock("seven"), None);
        assert_eq!(clock("07:00:00:00"), None);
    }

    #[test]
    fn the_regular_late_shift_keeps_away_from_the_weekend() {
        // The catalogue marks the weekend shifts with `WE`: the regular late
        // shift may not be worked on Saturday or Sunday, the weekend one may.
        assert!(!works_weekend(ShiftBand::Late, Some("S"), "Spätschicht"));
        assert!(works_weekend(
            ShiftBand::Late,
            Some("S-WE"),
            "Spätschicht Wochenende"
        ));
        assert!(works_weekend(
            ShiftBand::Late,
            Some("S-WE-AZ"),
            "Spätschicht Wochenende Azubi"
        ));
        // The early shift and the Zwischenschichten are no late shift.
        assert!(works_weekend(ShiftBand::Early, Some("F"), "Frühschicht"));
        assert!(works_weekend(
            ShiftBand::Intermediate,
            Some("ZS1"),
            "Zwischenschicht 1"
        ));
        // `Wochenende` in the name is no marker, `WE` as a word is.
        assert!(!weekend_marker(None, "Spätschicht Wochenende"));
        assert!(weekend_marker(Some("we"), "Spätschicht"));
    }

    #[test]
    fn a_free_day_keeps_no_times_and_no_break() {
        let mut draft = input();
        draft.band = Some(ShiftBand::Free);
        // The mask disables the fields; whatever still arrives is dropped.
        draft.break_minutes = 30;
        draft.apprentice_only = true;
        let clean = draft.clean().unwrap();
        assert_eq!(clean.starts_at, FREE_DAY_START);
        assert_eq!(clean.ends_at, FREE_DAY_END);
        assert_eq!(clean.break_minutes, 0);
        assert_eq!(clean.band, ShiftBand::Free);
        // A free day takes nobody, so it cannot be an apprentice-only shift.
        assert!(!clean.apprentice_only);
    }

    #[test]
    fn the_free_band_reads_and_writes_itself() {
        assert_eq!(ShiftBand::Free.as_str(), "free");
        assert_eq!(ShiftBand::parse(" Free "), ShiftBand::Free);
        assert!(ShiftBand::Free.is_free());
        assert!(!ShiftBand::Early.is_free());
    }

    #[test]
    fn a_shift_may_cross_midnight() {
        assert_eq!(duration_minutes("06:00", "14:30"), Some(510));
        assert_eq!(duration_minutes("22:00", "06:00"), Some(480));
    }

    #[test]
    fn a_shift_needs_a_name_and_two_times() {
        let mut draft = input();
        assert!(draft.clean().is_ok());
        draft.name = "   ".into();
        assert!(draft.clean().is_err());
        let mut draft = input();
        draft.starts_at = "at six".into();
        assert!(draft.clean().is_err());
        let mut draft = input();
        draft.ends_at = "06:00".into();
        assert!(draft.clean().is_err());
    }

    #[test]
    fn the_break_fits_into_the_shift() {
        let mut draft = input();
        draft.break_minutes = -1;
        assert!(draft.clean().is_err());
        let mut draft = input();
        draft.break_minutes = 510;
        assert!(draft.clean().is_err());
        let mut draft = input();
        draft.break_minutes = 720;
        assert!(draft.clean().is_err());
    }

    #[test]
    fn the_colour_is_a_hex_triplet_or_empty() {
        let mut draft = input();
        let clean = draft.clean().unwrap();
        assert_eq!(clean.color.as_deref(), Some("#1F6FEB"));
        draft.color = Some("  ".into());
        assert_eq!(draft.clean().unwrap().color, None);
        draft.color = Some("red".into());
        assert!(draft.clean().is_err());
    }

    #[test]
    fn excluded_positions_are_trimmed_and_counted_once() {
        let exclusions = ShiftExclusionsInput {
            positions: vec![
                " Meister ".to_string(),
                "meister".to_string(),
                "  ".to_string(),
                "Brücke".to_string(),
            ],
        };
        assert_eq!(
            exclusions.clean().unwrap(),
            vec!["Meister".to_string(), "Brücke".to_string()]
        );

        let open = ShiftExclusionsInput {
            positions: vec!["*".to_string()],
        };
        assert!(open.clean().is_err());

        let long = ShiftExclusionsInput {
            positions: vec!["x".repeat(crate::demand::MAX_POSITION_LEN + 1)],
        };
        assert!(long.clean().is_err());

        let many = ShiftExclusionsInput {
            positions: (0..=MAX_EXCLUDED_POSITIONS)
                .map(|index| format!("P{index}"))
                .collect(),
        };
        assert!(many.clean().is_err());
    }
}
