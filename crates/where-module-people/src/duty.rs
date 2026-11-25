//! Duty plans: who works on which day — in a shift, in a free time window, sick
//! or on vacation.
//!
//! There is at most one entry per employee and day. The planning replaces it on
//! save and removes it when the cell is emptied.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use where_domain::DomainError;

/// Longest range one query returns at once.
pub const MAX_RANGE_DAYS: i64 = 400;

/// The cell holds a shift of the master data.
pub const KIND_SHIFT: &str = "shift";
/// The cell holds a sick note.
pub const KIND_SICK: &str = "sick";
/// The cell holds vacation.
pub const KIND_VACATION: &str = "vacation";
/// The cell holds a time window of its own (`starts_at`, `ends_at`, `break_minutes`).
pub const KIND_FREE: &str = "free";

/// Longest break of a free time window.
pub const MAX_FREE_BREAK: i16 = 720;

/// Longest series one call writes at once.
pub const MAX_SERIES_DAYS: i64 = 366;

/// Longest note of one cell.
pub const MAX_NOTE_CHARS: usize = 200;

/// Kind, shift and the times of one cell, already checked.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanKind {
    kind: String,
    shift_id: Option<i64>,
    starts_at: Option<String>,
    ends_at: Option<String>,
    break_minutes: Option<i16>,
}

/// Checks kind, shift and — for a free time window — its two clocks and the
/// break; `None` means: clear the cell.
fn clean_kind(
    kind: Option<&str>,
    shift_id: Option<i64>,
    starts_at: Option<&str>,
    ends_at: Option<&str>,
    break_minutes: Option<i16>,
) -> Result<Option<CleanKind>, DomainError> {
    let Some(kind) = kind.map(str::trim) else {
        return Ok(None);
    };
    if kind.is_empty() {
        return Ok(None);
    }
    if !matches!(kind, KIND_SHIFT | KIND_SICK | KIND_VACATION | KIND_FREE) {
        return Err(DomainError::Validation(
            "Unknown kind in the duty plan (shift, sick, vacation or free).".to_string(),
        ));
    }
    let mut clean = CleanKind {
        kind: kind.to_string(),
        shift_id: None,
        starts_at: None,
        ends_at: None,
        break_minutes: None,
    };
    if kind == KIND_SHIFT {
        clean.shift_id = Some(shift_id.ok_or_else(|| {
            DomainError::Validation("The shift id is missing for a shift.".to_string())
        })?);
    }
    if kind == KIND_FREE {
        let start = starts_at
            .and_then(crate::shifts::clock)
            .ok_or_else(|| DomainError::Validation("The start is not a time.".to_string()))?;
        let end = ends_at
            .and_then(crate::shifts::clock)
            .ok_or_else(|| DomainError::Validation("The end is not a time.".to_string()))?;
        if start == end {
            return Err(DomainError::Validation(
                "The start and the end of a free time are equal.".to_string(),
            ));
        }
        let pause = break_minutes.unwrap_or(0);
        if !(0..=MAX_FREE_BREAK).contains(&pause) {
            return Err(DomainError::Validation(format!(
                "The break is between 0 and {MAX_FREE_BREAK} minutes."
            )));
        }
        if let Some(duration) = crate::shifts::duration_minutes(&start, &end)
            && i32::from(pause) >= duration
        {
            return Err(DomainError::Validation(
                "The break is longer than the free time.".to_string(),
            ));
        }
        clean.starts_at = Some(start);
        clean.ends_at = Some(end);
        clean.break_minutes = Some(pause);
    }
    Ok(Some(clean))
}

/// Checks the note of a cell: trimmed, empty is no note, and it stays short.
fn clean_note(note: Option<&str>) -> Result<Option<String>, DomainError> {
    let Some(text) = note.map(str::trim).filter(|text| !text.is_empty()) else {
        return Ok(None);
    };
    if text.chars().count() > MAX_NOTE_CHARS {
        return Err(DomainError::Validation(format!(
            "A note holds at most {MAX_NOTE_CHARS} characters."
        )));
    }
    Ok(Some(text.to_string()))
}

/// One entry of the duty plan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DutyEntry {
    pub id: i64,
    pub employee_id: i64,
    /// `shift`, `sick`, `vacation` or `free`.
    pub kind: String,
    /// Only set for `kind = shift`.
    pub shift_id: Option<i64>,
    /// Only set for `kind = free`.
    #[serde(default)]
    pub starts_at: Option<String>,
    /// Only set for `kind = free`.
    #[serde(default)]
    pub ends_at: Option<String>,
    /// Only set for `kind = free`.
    #[serde(default)]
    pub break_minutes: Option<i16>,
    /// Free text of the cell — shown under the shift, on the board and on the sheet.
    #[serde(default)]
    pub note: Option<String>,
    pub day: NaiveDate,
}

/// What the planning board sends for one cell.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DutyInput {
    pub employee_id: i64,
    pub day: NaiveDate,
    /// `shift`, `sick`, `vacation`, `free`; without a value the cell is cleared.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub shift_id: Option<i64>,
    /// Start of a free time window as `HH:MM`.
    #[serde(default)]
    pub starts_at: Option<String>,
    /// End of a free time window as `HH:MM`.
    #[serde(default)]
    pub ends_at: Option<String>,
    /// Break of a free time window in minutes.
    #[serde(default)]
    pub break_minutes: Option<i16>,
    /// Free text of the cell, at most [`MAX_NOTE_CHARS`] characters.
    #[serde(default)]
    pub note: Option<String>,
}

/// A checked cell for the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanDuty {
    pub employee_id: i64,
    pub day: NaiveDate,
    pub kind: String,
    pub shift_id: Option<i64>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub break_minutes: Option<i16>,
    pub note: Option<String>,
}

impl CleanDuty {
    /// The time window of a free cell, as the rule check needs it.
    pub fn plan_shift(&self) -> Option<crate::duty_rules::PlanShift> {
        let (Some(start), Some(end)) = (self.starts_at.as_deref(), self.ends_at.as_deref()) else {
            return None;
        };
        crate::duty_rules::PlanShift::new(start, end, self.break_minutes.unwrap_or(0))
    }
}

/// What the board sends when a cell is written onto several days in a row.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DutySeriesInput {
    pub employee_id: i64,
    /// First day of the series.
    pub day: NaiveDate,
    /// Days in a row, the first day included.
    pub days: i64,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub shift_id: Option<i64>,
    /// Start of a free time window as `HH:MM`.
    #[serde(default)]
    pub starts_at: Option<String>,
    /// End of a free time window as `HH:MM`.
    #[serde(default)]
    pub ends_at: Option<String>,
    /// Break of a free time window in minutes.
    #[serde(default)]
    pub break_minutes: Option<i16>,
}

/// A checked series for the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanSeries {
    pub employee_id: i64,
    pub day: NaiveDate,
    /// Days in a row, the first day included.
    pub days: i64,
    pub kind: String,
    pub shift_id: Option<i64>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub break_minutes: Option<i16>,
}

impl CleanSeries {
    /// The time window of a free series, as the rule check needs it.
    pub fn plan_shift(&self) -> Option<crate::duty_rules::PlanShift> {
        let (Some(start), Some(end)) = (self.starts_at.as_deref(), self.ends_at.as_deref()) else {
            return None;
        };
        crate::duty_rules::PlanShift::new(start, end, self.break_minutes.unwrap_or(0))
    }
}

/// What a series wrote and where the planning rules stopped it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DutySeriesResult {
    /// The days that were written, in the order of their days.
    pub entries: Vec<DutyEntry>,
    /// Days the board asked for.
    pub requested: i64,
    /// First day that stayed free; `None` when every day was written.
    #[serde(default)]
    pub blocked: Option<DutyBlocked>,
}

/// The day a series stopped at and the rule that stopped it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DutyBlocked {
    pub day: NaiveDate,
    /// `sunday`, `absence`, `max_consecutive_days`, `min_rest_hours`,
    /// `max_daily_hours`, `max_weekly_hours` or `min_days_off_per_week`.
    pub reason: String,
}

/// The range of the board.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DutyQuery {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

/// The range of the board together with the department its view is narrowed to.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct BoardQuery {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Only the houses and events of this department — the `Zeige Abteilung` filter of
    /// the board; empty shows every department the session may dispose.
    #[serde(default)]
    pub department: Option<String>,
}

impl BoardQuery {
    /// Checks the range like the board itself does.
    pub fn validate(&self) -> Result<(), DomainError> {
        DutyQuery {
            from: self.from,
            to: self.to,
        }
        .validate()
    }

    /// The department the view asks for, lower case; `None` when it names none.
    pub fn department(&self) -> Option<String> {
        self.department
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_lowercase)
    }
}

/// The department a view is narrowed to, without a range.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct DepartmentQuery {
    /// Only this department; empty shows every one the session may dispose.
    #[serde(default)]
    pub department: Option<String>,
}

impl DepartmentQuery {
    /// The department the view asks for, lower case; `None` when it names none.
    pub fn department(&self) -> Option<String> {
        self.department
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_lowercase)
    }
}

impl DutyQuery {
    /// Checks the range so a query does not grow arbitrarily large.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.to < self.from {
            return Err(DomainError::Validation(
                "The end of the range lies before the start.".to_string(),
            ));
        }
        if (self.to - self.from).num_days() + 1 > MAX_RANGE_DAYS {
            return Err(DomainError::Validation(format!(
                "The range covers at most {MAX_RANGE_DAYS} days."
            )));
        }
        Ok(())
    }
}

/// The month a reset asks for.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct MonthQuery {
    /// Month, 1 to 12.
    pub month: u32,
    /// Year, 1900 to 2999.
    pub year: i32,
}

impl MonthQuery {
    /// The first and the last day of the month.
    pub fn range(&self) -> Result<(NaiveDate, NaiveDate), DomainError> {
        month_range(self.month, self.year)
    }
}

/// The first and the last day of a month.
pub fn month_range(month: u32, year: i32) -> Result<(NaiveDate, NaiveDate), DomainError> {
    if !(1..=12).contains(&month) {
        return Err(DomainError::Validation(
            "The month has to lie between 1 and 12.".to_string(),
        ));
    }
    if !(1900..=2999).contains(&year) {
        return Err(DomainError::Validation(
            "The year has to lie between 1900 and 2999.".to_string(),
        ));
    }
    let first = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| DomainError::Validation("Unknown month.".to_string()))?;
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| DomainError::Validation("Unknown month.".to_string()))?;
    Ok((first, next.pred_opt().unwrap_or(first)))
}

/// What a reset removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ResetReport {
    /// Rows that were removed.
    pub cleared: usize,
}

/// One cell of the plan as an exchange file carries it — already resolved to the
/// ids of the master data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreCell {
    pub employee_id: i64,
    pub day: NaiveDate,
    pub kind: String,
    pub shift_id: Option<i64>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub break_minutes: Option<i16>,
    pub note: Option<String>,
}

impl DutyInput {
    /// Checks the cell; `None` means: clear it.
    pub fn clean(&self) -> Result<Option<CleanDuty>, DomainError> {
        let Some(clean) = clean_kind(
            self.kind.as_deref(),
            self.shift_id,
            self.starts_at.as_deref(),
            self.ends_at.as_deref(),
            self.break_minutes,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(CleanDuty {
            employee_id: self.employee_id,
            day: self.day,
            kind: clean.kind,
            shift_id: clean.shift_id,
            starts_at: clean.starts_at,
            ends_at: clean.ends_at,
            break_minutes: clean.break_minutes,
            note: clean_note(self.note.as_deref())?,
        }))
    }
}

impl DutySeriesInput {
    /// Checks the series; it always needs a kind — a series is never a deletion.
    pub fn clean(&self) -> Result<CleanSeries, DomainError> {
        if !(1..=MAX_SERIES_DAYS).contains(&self.days) {
            return Err(DomainError::Validation(format!(
                "A series covers 1 to {MAX_SERIES_DAYS} days."
            )));
        }
        let Some(clean) = clean_kind(
            self.kind.as_deref(),
            self.shift_id,
            self.starts_at.as_deref(),
            self.ends_at.as_deref(),
            self.break_minutes,
        )?
        else {
            return Err(DomainError::Validation(
                "A series needs a shift, a free time, sick or vacation.".to_string(),
            ));
        };
        Ok(CleanSeries {
            employee_id: self.employee_id,
            day: self.day,
            days: self.days,
            kind: clean.kind,
            shift_id: clean.shift_id,
            starts_at: clean.starts_at,
            ends_at: clean.ends_at,
            break_minutes: clean.break_minutes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn draft(kind: Option<&str>, shift_id: Option<i64>) -> DutyInput {
        DutyInput {
            employee_id: 7,
            day: date("2026-11-23"),
            kind: kind.map(str::to_string),
            shift_id,
            starts_at: None,
            ends_at: None,
            break_minutes: None,
            note: None,
        }
    }

    #[test]
    fn a_range_runs_forward_and_stays_short() {
        let valid = DutyQuery {
            from: date("2026-11-01"),
            to: date("2026-11-30"),
        };
        assert!(valid.validate().is_ok());

        let reversed = DutyQuery {
            from: date("2026-12-01"),
            to: date("2026-11-30"),
        };
        assert!(reversed.validate().is_err());

        let too_long = DutyQuery {
            from: date("2026-01-01"),
            to: date("2027-12-31"),
        };
        assert!(too_long.validate().is_err());
    }

    #[test]
    fn a_month_runs_from_its_first_to_its_last_day() {
        let (from, to) = month_range(11, 2026).unwrap();
        assert_eq!(from, date("2026-11-01"));
        assert_eq!(to, date("2026-11-30"));

        let (from, to) = month_range(2, 2028).unwrap();
        assert_eq!(from, date("2028-02-01"));
        assert_eq!(to, date("2028-02-29"));

        let (from, to) = month_range(12, 2026).unwrap();
        assert_eq!(from, date("2026-12-01"));
        assert_eq!(to, date("2026-12-31"));

        assert!(month_range(0, 2026).is_err());
        assert!(month_range(13, 2026).is_err());
        assert!(month_range(11, 1899).is_err());
    }

    #[test]
    fn a_shift_needs_its_id() {
        let clean = draft(Some(KIND_SHIFT), Some(3)).clean().unwrap().unwrap();
        assert_eq!(clean.kind, KIND_SHIFT);
        assert_eq!(clean.shift_id, Some(3));
        assert!(draft(Some(KIND_SHIFT), None).clean().is_err());
    }

    #[test]
    fn absences_do_not_carry_a_shift() {
        let sick = draft(Some(KIND_SICK), Some(3)).clean().unwrap().unwrap();
        assert_eq!(sick.kind, KIND_SICK);
        assert_eq!(sick.shift_id, None);
        let vacation = draft(Some(KIND_VACATION), None).clean().unwrap().unwrap();
        assert_eq!(vacation.kind, KIND_VACATION);
        assert_eq!(vacation.shift_id, None);
    }

    #[test]
    fn a_note_belongs_to_the_cell_and_stays_short() {
        let mut cell = draft(Some(KIND_SHIFT), Some(3));
        cell.note = Some("  Tausch mit Kollegen  ".to_string());
        assert_eq!(
            cell.clean().unwrap().unwrap().note.as_deref(),
            Some("Tausch mit Kollegen")
        );

        let mut empty = draft(Some(KIND_SHIFT), Some(3));
        empty.note = Some("   ".to_string());
        assert_eq!(empty.clean().unwrap().unwrap().note, None);

        let mut too_long = draft(Some(KIND_SHIFT), Some(3));
        too_long.note = Some("x".repeat(MAX_NOTE_CHARS + 1));
        assert!(too_long.clean().is_err());
    }

    #[test]
    fn a_free_time_carries_its_own_clocks() {
        let frei = |start: Option<&str>, end: Option<&str>, pause: Option<i16>| DutyInput {
            employee_id: 7,
            day: date("2026-11-23"),
            kind: Some(KIND_FREE.to_string()),
            shift_id: None,
            starts_at: start.map(str::to_string),
            ends_at: end.map(str::to_string),
            break_minutes: pause,
            note: None,
        };
        let clean = frei(Some("7:30"), Some("13:00"), Some(30))
            .clean()
            .unwrap()
            .unwrap();
        assert_eq!(clean.kind, KIND_FREE);
        assert_eq!(clean.shift_id, None);
        assert_eq!(clean.starts_at.as_deref(), Some("07:30"));
        assert_eq!(clean.ends_at.as_deref(), Some("13:00"));
        assert_eq!(clean.break_minutes, Some(30));
        let shift = clean.plan_shift().unwrap();
        assert_eq!(shift.net, 300_i64);

        // Without the break the time counts whole.
        let ohne = frei(Some("22:00"), Some("06:00"), None)
            .clean()
            .unwrap()
            .unwrap();
        assert_eq!(ohne.break_minutes, Some(0));
        assert_eq!(ohne.plan_shift().unwrap().net, 480_i64);

        assert!(frei(Some("07:30"), None, None).clean().is_err());
        assert!(frei(Some("07:30"), Some("07:30"), None).clean().is_err());
        assert!(frei(Some("25:00"), Some("13:00"), None).clean().is_err());
        assert!(
            frei(Some("08:00"), Some("12:00"), Some(300))
                .clean()
                .is_err()
        );
        assert!(
            frei(Some("08:00"), Some("12:00"), Some(MAX_FREE_BREAK + 1))
                .clean()
                .is_err()
        );
    }

    #[test]
    fn a_free_series_needs_its_clocks_too() {
        let mut frei = serie(3, Some(KIND_FREE), None);
        assert!(frei.clean().is_err());
        frei.starts_at = Some("09:00".to_string());
        frei.ends_at = Some("17:30".to_string());
        frei.break_minutes = Some(45);
        let clean = frei.clean().unwrap();
        assert_eq!(clean.kind, KIND_FREE);
        assert_eq!(clean.plan_shift().unwrap().net, 465_i64);
    }

    #[test]
    fn without_a_kind_the_cell_is_cleared() {
        assert!(draft(None, None).clean().unwrap().is_none());
        assert!(draft(Some("  "), None).clean().unwrap().is_none());
        assert!(draft(Some("evening"), None).clean().is_err());
    }

    fn serie(days: i64, kind: Option<&str>, shift_id: Option<i64>) -> DutySeriesInput {
        DutySeriesInput {
            employee_id: 7,
            day: date("2026-12-01"),
            days,
            kind: kind.map(str::to_string),
            shift_id,
            starts_at: None,
            ends_at: None,
            break_minutes: None,
        }
    }

    #[test]
    fn a_series_counts_the_first_day_and_stays_within_its_bounds() {
        let clean = serie(5, Some(KIND_SHIFT), Some(34)).clean().unwrap();
        assert_eq!(clean.days, 5);
        assert_eq!(clean.day, date("2026-12-01"));
        assert_eq!(clean.shift_id, Some(34));

        assert!(serie(1, Some(KIND_SICK), None).clean().is_ok());
        assert!(serie(0, Some(KIND_SICK), None).clean().is_err());
        assert!(serie(-3, Some(KIND_SICK), None).clean().is_err());
        assert!(
            serie(MAX_SERIES_DAYS + 1, Some(KIND_SICK), None)
                .clean()
                .is_err()
        );
    }

    #[test]
    fn a_series_always_needs_a_kind_and_a_shift_id() {
        assert!(serie(5, None, None).clean().is_err());
        assert!(serie(5, Some("  "), Some(34)).clean().is_err());
        assert!(serie(5, Some(KIND_SHIFT), None).clean().is_err());
        assert!(serie(5, Some("evening"), Some(34)).clean().is_err());

        let vacation = serie(3, Some(KIND_VACATION), Some(34)).clean().unwrap();
        assert_eq!(vacation.kind, KIND_VACATION);
        assert_eq!(vacation.shift_id, None);
    }
}
