//! Calendar (in epirent "TeamAgenda"): appointments and their recurrence.
//!
//! The legacy application draws the calendar with the hmCal plugin; its data lives
//! in `"Scheduler"` plus `"Scheduler_RRules"` (recurrence) and `"Scheduler_RRExDate"`
//! (single occurrences that were removed). This module rebuilds the data side of
//! that: one appointment per row, a series expanded for the requested range and a
//! removal list per occurrence.

use chrono::{DateTime, Datelike, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::DomainError;

/// How a series repeats. The legacy codes are a plugin detail; the numbers below
/// are our own mapping and are translated in the persistence layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum RecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl RecurrenceFrequency {
    /// Steps `date` forward by `interval` units of this frequency.
    pub fn advance(self, date: NaiveDate, interval: u32) -> Option<NaiveDate> {
        let months = match self {
            Self::Daily => {
                return date.checked_add_signed(chrono::TimeDelta::days(i64::from(interval)));
            }
            Self::Weekly => {
                return date.checked_add_signed(chrono::TimeDelta::days(7 * i64::from(interval)));
            }
            Self::Monthly => interval,
            Self::Yearly => 12 * interval,
        };
        date.checked_add_months(chrono::Months::new(months))
    }
}

/// Repetition of an appointment (`"Scheduler_RRules"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AgendaRecurrence {
    pub frequency: RecurrenceFrequency,
    /// Every n-th day/week/month/year; `1` = every one.
    pub interval: u16,
    /// Last day the series may occur.
    pub until: Option<NaiveDate>,
    /// Number of occurrences, counted from the first one.
    pub count: Option<i32>,
}

impl AgendaRecurrence {
    fn validate(&self) -> Result<(), DomainError> {
        if self.interval == 0 {
            return Err(DomainError::Validation(
                "Die Wiederholung braucht ein Intervall von mindestens 1.".into(),
            ));
        }
        if self.count.is_some_and(|count| count < 1) {
            return Err(DomainError::Validation(
                "Die Anzahl der Wiederholungen muss mindestens 1 sein.".into(),
            ));
        }
        Ok(())
    }

    /// Date of the n-th occurrence of a series that starts at `first`.
    fn nth(&self, first: NaiveDate, n: u32) -> Option<NaiveDate> {
        if n == 0 {
            return Some(first);
        }
        self.frequency.advance(first, u32::from(self.interval) * n)
    }
}

/// One appointment as stored (`"Scheduler"`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AgendaEvent {
    pub id: i32,
    pub title: String,
    /// Second line of the tile in the calendar (`"Event_HeadText"`).
    pub head_text: Option<String>,
    pub notes: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub all_day: bool,
    /// Calendar the appointment belongs to (`"Type"` → `"Storage_TACategory"`).
    pub category: Option<i32>,
    pub category_name: Option<String>,
    pub venue: Option<String>,
    pub address_id: Option<i32>,
    pub address_name: Option<String>,
    /// Tile colour; `custom` marks a colour the user picked instead of a default.
    pub color_red: Option<i32>,
    pub color_green: Option<i32>,
    pub color_blue: Option<i32>,
    pub custom_color: bool,
    /// Set when the appointment is visible to one user only (`"PrivateUser_Primkey"`).
    pub private_user: Option<i32>,
    pub remind: bool,
    /// Minutes before the start that the reminder fires.
    pub remind_minutes: Option<i32>,
    pub recurrence: Option<AgendaRecurrence>,
    pub changed_at: Option<DateTime<Utc>>,
    pub changed_by: Option<String>,
}

impl AgendaEvent {
    /// Checks the minimum the mask guarantees: a title, an ordered period and a
    /// recurrence that makes sense.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.title.trim().is_empty() {
            return Err(DomainError::Validation(
                "Der Termin braucht einen Titel.".into(),
            ));
        }
        if self.end_date < self.start_date {
            return Err(DomainError::Validation(
                "Das Ende darf nicht vor dem Beginn liegen.".into(),
            ));
        }
        if !self.all_day
            && self.end_date == self.start_date
            && let (Some(start), Some(end)) = (self.start_time, self.end_time)
            && end < start
        {
            return Err(DomainError::Validation(
                "Die Endzeit liegt vor der Startzeit.".into(),
            ));
        }
        for (name, channel) in [
            ("Rot", self.color_red),
            ("Grün", self.color_green),
            ("Blau", self.color_blue),
        ] {
            if channel.is_some_and(|value| !(0..=255).contains(&value)) {
                return Err(DomainError::Validation(format!(
                    "Der Farbanteil {name} muss zwischen 0 und 255 liegen."
                )));
            }
        }
        if self.remind_minutes.is_some_and(|minutes| minutes < 0) {
            return Err(DomainError::Validation(
                "Die Erinnerung kann nicht vor der Vergangenheit liegen.".into(),
            ));
        }
        if let Some(recurrence) = &self.recurrence {
            recurrence.validate()?;
        }
        Ok(())
    }

    /// Number of days the appointment covers, `0` for a single day.
    fn duration_days(&self) -> i64 {
        (self.end_date - self.start_date).num_days()
    }

    /// First days of the occurrences that touch `from`..`to` (inclusive), including
    /// the occurrences of a series.
    ///
    /// A multi-day appointment contributes its first day only; its length travels
    /// in `end_date`, so the tile can show the period instead of covering several
    /// cells. `exceptions` are the dates a single occurrence was removed. The list
    /// is capped, so a daily series over decades cannot flood the calendar.
    pub fn occurrences(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        exceptions: &[NaiveDate],
    ) -> Vec<NaiveDate> {
        const LIMIT: usize = 400;

        let mut result = Vec::new();
        let Some(recurrence) = &self.recurrence else {
            if self.start_date <= to && self.end_date >= from {
                result.push(self.start_date);
            }
            return result;
        };

        let duration = self.duration_days();
        let mut index: u32 = 0;
        loop {
            if result.len() >= LIMIT {
                break;
            }
            if recurrence
                .count
                .is_some_and(|count| i64::from(index) >= i64::from(count))
            {
                break;
            }
            let Some(start) = recurrence.nth(self.start_date, index) else {
                break;
            };
            if recurrence.until.is_some_and(|until| start > until) {
                break;
            }
            if start > to {
                break;
            }
            let end = start
                .checked_add_signed(chrono::TimeDelta::days(duration))
                .unwrap_or(start);
            if end >= from && !exceptions.contains(&start) {
                result.push(start);
            }
            index += 1;
        }
        result
    }
}

/// What the mask sends for a new or changed appointment.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AgendaEventInput {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub head_text: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub start_date: Option<NaiveDate>,
    #[serde(default)]
    pub end_date: Option<NaiveDate>,
    #[serde(default)]
    pub start_time: Option<NaiveTime>,
    #[serde(default)]
    pub end_time: Option<NaiveTime>,
    #[serde(default)]
    pub all_day: bool,
    #[serde(default)]
    pub category: Option<i32>,
    #[serde(default)]
    pub venue: Option<String>,
    #[serde(default)]
    pub address_id: Option<i32>,
    #[serde(default)]
    pub color_red: Option<i32>,
    #[serde(default)]
    pub color_green: Option<i32>,
    #[serde(default)]
    pub color_blue: Option<i32>,
    #[serde(default)]
    pub custom_color: bool,
    #[serde(default)]
    pub private_user: Option<i32>,
    #[serde(default)]
    pub remind: bool,
    #[serde(default)]
    pub remind_minutes: Option<i32>,
    #[serde(default)]
    pub recurrence: Option<AgendaRecurrence>,
}

impl AgendaEventInput {
    /// Fills the defaults the mask leaves open and validates the result.
    pub fn into_event(self) -> Result<AgendaEvent, DomainError> {
        let start_date = self
            .start_date
            .ok_or_else(|| DomainError::Validation("Der Termin braucht ein Startdatum.".into()))?;
        let end_date = self.end_date.unwrap_or(start_date);
        let event = AgendaEvent {
            id: 0,
            title: self.title,
            head_text: self.head_text,
            notes: self.notes,
            start_date,
            end_date,
            start_time: if self.all_day { None } else { self.start_time },
            end_time: if self.all_day { None } else { self.end_time },
            all_day: self.all_day,
            category: self.category,
            category_name: None,
            venue: self.venue,
            address_id: self.address_id,
            address_name: None,
            color_red: self.color_red,
            color_green: self.color_green,
            color_blue: self.color_blue,
            custom_color: self.custom_color,
            private_user: self.private_user,
            remind: self.remind,
            remind_minutes: self.remind_minutes,
            recurrence: self.recurrence,
            changed_at: None,
            changed_by: None,
        };
        event.validate()?;
        Ok(event)
    }
}

/// One date of an appointment in the requested range — a series contributes several.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgendaOccurrence {
    /// Appointment the occurrence belongs to.
    pub event_id: i32,
    /// First day of this occurrence (differs from `start_date` for a series).
    pub date: NaiveDate,
    pub title: String,
    pub head_text: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub all_day: bool,
    pub category: Option<i32>,
    pub category_name: Option<String>,
    pub venue: Option<String>,
    pub address_id: Option<i32>,
    pub address_name: Option<String>,
    pub color_red: Option<i32>,
    pub color_green: Option<i32>,
    pub color_blue: Option<i32>,
    pub custom_color: bool,
    pub private_user: Option<i32>,
    pub recurring: bool,
}

impl AgendaOccurrence {
    /// Builds the calendar entry of `date` from the appointment.
    pub fn of(event: &AgendaEvent, date: NaiveDate) -> Self {
        let duration = event.duration_days();
        Self {
            event_id: event.id,
            date,
            title: event.title.clone(),
            head_text: event.head_text.clone(),
            start_date: date,
            end_date: date
                .checked_add_signed(chrono::TimeDelta::days(duration))
                .unwrap_or(date),
            start_time: event.start_time,
            end_time: event.end_time,
            all_day: event.all_day,
            category: event.category,
            category_name: event.category_name.clone(),
            venue: event.venue.clone(),
            address_id: event.address_id,
            address_name: event.address_name.clone(),
            color_red: event.color_red,
            color_green: event.color_green,
            color_blue: event.color_blue,
            custom_color: event.custom_color,
            private_user: event.private_user,
            recurring: event.recurrence.is_some(),
        }
    }
}

/// Calendar the appointments are filed under (`"Storage_TACategory"`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgendaCategory {
    pub id: i32,
    pub name: String,
    /// Number of appointments in this calendar.
    pub appointments: i64,
}

/// Name of a calendar to create or rename (`"Storage_TACategory"`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AgendaCategoryInput {
    #[serde(default)]
    pub name: String,
}

impl AgendaCategoryInput {
    pub fn validated(&self) -> Result<String, DomainError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Der Kalender braucht einen Namen.".into(),
            ));
        }
        if name.chars().count() > 255 {
            return Err(DomainError::Validation(
                "Der Name des Kalenders ist zu lang (maximal 255 Zeichen).".into(),
            ));
        }
        Ok(name.to_string())
    }
}

/// A birthday from the address master. The calendar shows it as a whole-day line
/// in the year of the visible range — the address mask owns the date.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AgendaBirthday {
    pub address_id: i32,
    pub name: String,
    /// Day inside the requested range.
    pub date: NaiveDate,
    /// Year of birth, so the tile can say how old the person turns.
    pub birth_year: i32,
}

impl AgendaBirthday {
    /// Birthday of `birthday` as a day of the range, `None` when it does not fall
    /// inside it. A `29.02.` lands on the `28.02.` in a common year.
    pub fn in_range(
        address_id: i32,
        name: String,
        birthday: NaiveDate,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Option<Self> {
        if name.trim().is_empty() {
            return None;
        }
        for year in from.year()..=to.year() {
            let Some(day) = NaiveDate::from_ymd_opt(year, birthday.month(), birthday.day())
                .or_else(|| NaiveDate::from_ymd_opt(year, birthday.month(), birthday.day() - 1))
            else {
                continue;
            };
            if day >= from && day <= to {
                return Some(Self {
                    address_id,
                    name: name.trim().to_string(),
                    date: day,
                    birth_year: birthday.year(),
                });
            }
        }
        None
    }
}

/// Everything the calendar view needs for one range.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgendaRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// One entry per appointment — its first day — and one per occurrence of a series.
    pub occurrences: Vec<AgendaOccurrence>,
    /// Birthdays of the range, from the address master.
    pub birthdays: Vec<AgendaBirthday>,
    /// Filter list of the left column.
    pub categories: Vec<AgendaCategory>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn event(start: &str, end: &str) -> AgendaEvent {
        AgendaEvent {
            id: 1,
            title: "Aufbau".into(),
            start_date: date(start),
            end_date: date(end),
            ..AgendaEvent::default()
        }
    }

    #[test]
    fn single_appointment_is_found_inside_the_range() {
        let e = event("2026-11-10", "2026-11-10");
        assert_eq!(
            e.occurrences(date("2026-11-01"), date("2026-11-30"), &[])
                .len(),
            1
        );
        assert!(
            e.occurrences(date("2026-12-01"), date("2026-12-31"), &[])
                .is_empty()
        );
    }

    #[test]
    fn appointment_overlapping_the_range_start_is_found() {
        // A three-day appointment that began before the shown month.
        let e = event("2026-10-30", "2026-11-02");
        assert_eq!(
            e.occurrences(date("2026-11-01"), date("2026-11-30"), &[]),
            vec![date("2026-10-30")]
        );
    }

    #[test]
    fn weekly_series_produces_one_entry_per_week() {
        let mut e = event("2026-11-02", "2026-11-02"); // a Monday
        e.recurrence = Some(AgendaRecurrence {
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            until: None,
            count: None,
        });
        let dates = e.occurrences(date("2026-11-01"), date("2026-11-30"), &[]);
        assert_eq!(
            dates,
            vec![
                date("2026-11-02"),
                date("2026-11-09"),
                date("2026-11-16"),
                date("2026-11-23"),
                date("2026-11-30")
            ]
        );
    }

    #[test]
    fn series_respects_interval_count_and_until() {
        let mut e = event("2026-11-02", "2026-11-02");
        e.recurrence = Some(AgendaRecurrence {
            frequency: RecurrenceFrequency::Daily,
            interval: 2,
            until: Some(date("2026-11-06")),
            count: Some(3),
        });
        // Day 0, 2, 4 — the third occurrence is the last one because of `count`.
        assert_eq!(
            e.occurrences(date("2026-11-01"), date("2026-11-30"), &[]),
            vec![date("2026-11-02"), date("2026-11-04"), date("2026-11-06")]
        );
    }

    #[test]
    fn removed_occurrence_is_skipped() {
        let mut e = event("2026-11-02", "2026-11-02");
        e.recurrence = Some(AgendaRecurrence {
            frequency: RecurrenceFrequency::Weekly,
            interval: 1,
            until: None,
            count: None,
        });
        let dates = e.occurrences(
            date("2026-11-01"),
            date("2026-11-16"),
            &[date("2026-11-09")],
        );
        assert_eq!(dates, vec![date("2026-11-02"), date("2026-11-16")]);
    }

    #[test]
    fn monthly_series_clamps_to_the_last_day_of_short_months() {
        let mut e = event("2026-01-31", "2026-01-31");
        e.recurrence = Some(AgendaRecurrence {
            frequency: RecurrenceFrequency::Monthly,
            interval: 1,
            until: None,
            count: None,
        });
        // February has no 31st; the series falls back to its last day and returns to
        // the 31st in March, because every occurrence is computed from the first one.
        let dates = e.occurrences(date("2026-02-01"), date("2026-03-31"), &[]);
        assert_eq!(dates, vec![date("2026-02-28"), date("2026-03-31")]);
    }

    #[test]
    fn validation_rejects_an_inverted_period() {
        let mut input = AgendaEventInput {
            title: "Aufbau".into(),
            start_date: Some(date("2026-11-10")),
            end_date: Some(date("2026-11-01")),
            ..AgendaEventInput::default()
        };
        assert!(input.clone().into_event().is_err());
        input.end_date = Some(date("2026-11-12"));
        assert!(input.into_event().is_ok());
    }

    #[test]
    fn validation_rejects_an_empty_title() {
        let input = AgendaEventInput {
            start_date: Some(date("2026-11-10")),
            ..AgendaEventInput::default()
        };
        assert!(matches!(
            input.into_event(),
            Err(DomainError::Validation(_))
        ));
    }

    #[test]
    fn all_day_drops_the_times() {
        let input = AgendaEventInput {
            title: "Feiertag".into(),
            start_date: Some(date("2026-11-10")),
            start_time: Some(NaiveTime::from_hms_opt(9, 0, 0).unwrap()),
            all_day: true,
            ..AgendaEventInput::default()
        };
        let event = input.into_event().unwrap();
        assert!(event.start_time.is_none() && event.all_day);
    }

    #[test]
    fn birthday_lands_in_the_year_of_the_range() {
        let birthday = AgendaBirthday::in_range(
            7,
            "Bühnenbau Meister".into(),
            date("1975-11-13"),
            date("2026-11-01"),
            date("2026-11-30"),
        )
        .unwrap();
        assert_eq!(birthday.date, date("2026-11-13"));
        assert_eq!(birthday.birth_year, 1975);
    }

    #[test]
    fn birthday_outside_the_range_is_dropped() {
        assert!(
            AgendaBirthday::in_range(
                7,
                "Bühnenbau Meister".into(),
                date("1975-03-13"),
                date("2026-11-01"),
                date("2026-11-30"),
            )
            .is_none()
        );
    }

    #[test]
    fn birthday_on_the_29th_of_february_falls_back_to_the_28th() {
        let birthday = AgendaBirthday::in_range(
            8,
            "Schauspielhaus".into(),
            date("1980-02-29"),
            date("2026-02-01"),
            date("2026-02-28"),
        )
        .unwrap();
        assert_eq!(birthday.date, date("2026-02-28"));
        let leap = AgendaBirthday::in_range(
            8,
            "Schauspielhaus".into(),
            date("1980-02-29"),
            date("2028-02-01"),
            date("2028-02-29"),
        )
        .unwrap();
        assert_eq!(leap.date, date("2028-02-29"));
    }
}
