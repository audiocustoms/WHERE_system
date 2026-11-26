//! The month of one employee as an iCalendar file (`text/calendar`).
//!
//! The board hands every employee their own file: the days they work with the shift
//! they hold and its times — nothing of the colleagues, nothing of the disposition.
//! `DTSTART`/`DTEND` carry the real times, so a calendar client shows the shift where it
//! belongs; the summary is the name of the shift, the description repeats the line the
//! mail body shows (`02.11. - Spätschicht - 15:00 - 23:00`).

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::employee::Employee;
use crate::schedule::KIND_SHIFT;
use crate::shifts::Shift;

/// What the board sends: the period and the employees whose calendar goes out.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct IcalSendInput {
    pub from: NaiveDate,
    pub to: NaiveDate,
    #[serde(default)]
    pub employee_ids: Vec<i64>,
}

/// What one send brought.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct IcalSendReport {
    /// Messages the server accepted.
    pub sent: usize,
    /// Who got none, with the reason.
    #[serde(default)]
    pub skipped: Vec<IcalSkip>,
}

/// One employee without a message.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IcalSkip {
    pub name: String,
    pub reason: String,
}

/// One entry of the exported month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcalDay {
    pub day: NaiveDate,
    /// Name of the shift, e.g. `Spätschicht`.
    pub shift: String,
    /// `HH:MM`, empty when the day counts as a whole (a free day).
    pub starts_at: String,
    pub ends_at: String,
}

/// Everything one employee's calendar carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcalMonth {
    /// Name as the entry shows it, e.g. `Mauris Apolloni`.
    pub employee: String,
    /// Personnel number, empty when none is maintained.
    pub personnel_no: String,
    pub days: Vec<IcalDay>,
}

/// Builds the month of an employee from the cells of the board.
///
/// `cells` is the stored plan (`employee_id`, `day`, `kind`, `shift_id`, `starts_at`,
/// `ends_at`); only shifts (`kind = shift`) become entries — vacation and sickness are no
/// calendar event. A stored shift cell carries no times of its own, they come from the
/// shift; a free day keeps none and counts as the whole day.
pub fn month_of(
    employee: &Employee,
    cells: &[crate::schedule::ScheduleEntry],
    shifts: &[Shift],
) -> IcalMonth {
    let id = employee.id.unwrap_or_default();
    let mut days: Vec<IcalDay> = cells
        .iter()
        .filter(|cell| cell.employee_id == id && cell.kind == KIND_SHIFT)
        .filter_map(|cell| {
            let shift = cell
                .shift_id
                .and_then(|shift_id| shifts.iter().find(|entry| entry.id == shift_id));
            let name = shift
                .map(|entry| entry.name.clone())
                .unwrap_or_else(|| cell.note.clone().unwrap_or_default());
            if name.trim().is_empty() {
                return None;
            }
            let (starts_at, ends_at) = clock_of(cell, shift);
            Some(IcalDay {
                day: cell.day,
                shift: name,
                starts_at,
                ends_at,
            })
        })
        .collect();
    days.sort_by_key(|entry| entry.day);
    IcalMonth {
        employee: format!("{} {}", employee.first_name, employee.last_name)
            .trim()
            .to_string(),
        personnel_no: employee.personnel_no.clone().unwrap_or_default(),
        days,
    }
}

/// The times of one day: the cell, else the shift. A free day carries none.
fn clock_of(cell: &crate::schedule::ScheduleEntry, shift: Option<&Shift>) -> (String, String) {
    if shift.is_some_and(|entry| entry.band.is_free()) {
        return (String::new(), String::new());
    }
    let starts_at = given(&cell.starts_at).or_else(|| shift.map(|entry| entry.starts_at.clone()));
    let ends_at = given(&cell.ends_at).or_else(|| shift.map(|entry| entry.ends_at.clone()));
    (starts_at.unwrap_or_default(), ends_at.unwrap_or_default())
}

fn given(time: &Option<String>) -> Option<String> {
    time.as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// The line the mail body and the description repeat: `02.11. - Spätschicht - 15:00 - 23:00`.
pub fn line(day: &IcalDay) -> String {
    let date = day.day.format("%d.%m.").to_string();
    let time = match (day.starts_at.is_empty(), day.ends_at.is_empty()) {
        (false, false) => format!("{} - {}", day.starts_at, day.ends_at),
        (false, true) => day.starts_at.clone(),
        _ => String::new(),
    };
    if time.is_empty() {
        format!("{date} - {}", day.shift)
    } else {
        format!("{date} - {} - {time}", day.shift)
    }
}

/// The whole calendar as `text/calendar`.
pub fn ics(month: &IcalMonth, stamp: NaiveDateTime) -> String {
    let mut text = String::new();
    text.push_str("BEGIN:VCALENDAR\r\n");
    text.push_str("VERSION:2.0\r\n");
    text.push_str("PRODID:-//WHERE//Dienstplan//DE\r\n");
    text.push_str("CALSCALE:GREGORIAN\r\n");
    text.push_str("METHOD:PUBLISH\r\n");
    text.push_str(&format!(
        "X-WR-CALNAME:{}\r\n",
        escape(&calendar_name(month))
    ));
    for day in &month.days {
        text.push_str("BEGIN:VEVENT\r\n");
        text.push_str(&format!("UID:{}\r\n", uid(month, day)));
        text.push_str(&format!("DTSTAMP:{}\r\n", form(stamp)));
        match times(day) {
            Some((start, end)) => {
                text.push_str(&format!("DTSTART:{}\r\n", form(start)));
                text.push_str(&format!("DTEND:{}\r\n", form(end)));
            }
            None => {
                text.push_str(&format!("DTSTART;VALUE=DATE:{}\r\n", date(day.day)));
                let next = day.day.succ_opt().unwrap_or(day.day);
                text.push_str(&format!("DTEND;VALUE=DATE:{}\r\n", date(next)));
            }
        }
        text.push_str(&format!("SUMMARY:{}\r\n", escape(&day.shift)));
        text.push_str(&format!("DESCRIPTION:{}\r\n", escape(&line(day))));
        text.push_str("END:VEVENT\r\n");
    }
    text.push_str("END:VCALENDAR\r\n");
    text
}

/// Calendar name: `Dienstplan – Mauris Apolloni (1006)`.
pub fn calendar_name(month: &IcalMonth) -> String {
    if month.personnel_no.is_empty() {
        format!("Dienstplan – {}", month.employee)
    } else {
        format!("Dienstplan – {} ({})", month.employee, month.personnel_no)
    }
}

/// The subject of the mail: the calendar name with a plain hyphen, so the header needs no
/// encoded word.
pub fn subject(month: &IcalMonth) -> String {
    if month.personnel_no.is_empty() {
        format!("Dienstplan - {}", month.employee)
    } else {
        format!("Dienstplan - {} ({})", month.employee, month.personnel_no)
    }
}

/// Start and end of one entry; `None` for a day that counts as a whole.
fn times(day: &IcalDay) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let start = day.day.and_time(clock(&day.starts_at)?);
    let mut end = day.day.and_time(clock(&day.ends_at)?);
    if end <= start {
        // An end before the start runs past midnight.
        end += chrono::TimeDelta::days(1);
    }
    Some((start, end))
}

fn clock(text: &str) -> Option<NaiveTime> {
    let (hour, minute) = text.trim().split_once(':')?;
    NaiveTime::from_hms_opt(hour.trim().parse().ok()?, minute.trim().parse().ok()?, 0)
}

/// `20261102` — a whole day, as a calendar wants it.
fn date(day: NaiveDate) -> String {
    day.format("%Y%m%d").to_string()
}

/// `20261102T150000` — local time, as a calendar wants it without a zone.
fn form(moment: NaiveDateTime) -> String {
    moment.format("%Y%m%dT%H%M%S").to_string()
}

fn uid(month: &IcalMonth, day: &IcalDay) -> String {
    let wer = format!(
        "{}|{}|{}",
        month.employee,
        day.day.format("%Y-%m-%d"),
        day.shift
    );
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in wer.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}@where.dienstplan")
}

/// Escapes a text for a calendar property (RFC 5545, 3.3.11).
fn escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::ScheduleEntry;
    use crate::shifts::{ShiftBand, ShiftInput};

    fn employee() -> Employee {
        let mut person = Employee::default();
        person.id = Some(7);
        person.first_name = "Mauris".into();
        person.last_name = "Apolloni".into();
        person.personnel_no = Some("1006".into());
        person
    }

    fn shift(name: &str) -> Shift {
        let input = ShiftInput {
            name: name.into(),
            short_name: Some("S".into()),
            starts_at: "15:00".into(),
            ends_at: "23:00".into(),
            band: Some(ShiftBand::Late),
            break_minutes: 30,
            color: None,
            department: None,
            apprentice_only: false,
            active: true,
        };
        let clean = input.clean().unwrap();
        Shift {
            id: 4,
            name: clean.name,
            short_name: clean.short_name,
            starts_at: clean.starts_at,
            ends_at: clean.ends_at,
            break_minutes: clean.break_minutes,
            band: clean.band,
            apprentice_only: clean.apprentice_only,
            color: clean.color,
            department: clean.department,
            active: clean.active,
            sort_order: 0,
            excluded_positions: Vec::new(),
        }
    }

    fn cell(day: NaiveDate, kind: &str) -> ScheduleEntry {
        let mut entry = ScheduleEntry::default();
        entry.employee_id = 7;
        entry.day = day;
        entry.kind = kind.to_string();
        entry.shift_id = Some(4);
        entry
    }

    fn free_shift(name: &str) -> Shift {
        let mut entry = shift(name);
        entry.band = ShiftBand::Free;
        entry.starts_at = "00:00".into();
        entry.ends_at = "23:59".into();
        entry
    }

    #[test]
    fn only_shifts_become_entries() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let cells = [
            cell(day, KIND_SHIFT),
            cell(day.succ_opt().unwrap(), "vacation"),
            cell(day.succ_opt().unwrap().succ_opt().unwrap(), KIND_SHIFT),
        ];
        let month = month_of(&employee(), &cells, &[shift("Spätschicht")]);
        assert_eq!(month.days.len(), 2);
        assert_eq!(line(&month.days[0]), "02.11. - Spätschicht - 15:00 - 23:00");
        assert_eq!(month.employee, "Mauris Apolloni");
    }

    #[test]
    fn a_stored_cell_takes_its_times_from_the_shift() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let month = month_of(
            &employee(),
            &[cell(day, KIND_SHIFT)],
            &[shift("Spätschicht")],
        );
        assert_eq!(line(&month.days[0]), "02.11. - Spätschicht - 15:00 - 23:00");
        let text = ics(&month, day.and_hms_opt(9, 0, 0).unwrap());
        assert!(text.contains("DTSTART:20261102T150000\r\n"), "{text}");
        assert!(text.contains("DTEND:20261102T230000\r\n"), "{text}");
    }

    #[test]
    fn a_free_day_counts_as_the_whole_day() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let month = month_of(
            &employee(),
            &[cell(day, KIND_SHIFT)],
            &[free_shift("Freier Tag")],
        );
        assert_eq!(line(&month.days[0]), "01.11. - Freier Tag");
        let text = ics(&month, day.and_hms_opt(9, 0, 0).unwrap());
        assert!(text.contains("DTSTART;VALUE=DATE:20261101\r\n"), "{text}");
        assert!(text.contains("DTEND;VALUE=DATE:20261102\r\n"), "{text}");
    }

    #[test]
    fn the_calendar_carries_the_times_and_escapes_commas() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let mut cells = vec![cell(day, KIND_SHIFT)];
        cells[0].starts_at = Some("15:00".into());
        let month = month_of(&employee(), &cells, &[shift("Spät, Schicht; 2")]);
        let text = ics(&month, day.and_hms_opt(9, 0, 0).unwrap());
        assert!(text.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(text.ends_with("END:VCALENDAR\r\n"));
        assert!(text.contains("DTSTART:20261102T150000\r\n"), "{text}");
        assert!(text.contains("DTEND:20261102T230000\r\n"), "{text}");
        assert!(text.contains("SUMMARY:Spät\\, Schicht\\; 2\r\n"), "{text}");
        assert!(text.contains("X-WR-CALNAME:Dienstplan – Mauris Apolloni (1006)\r\n"));
        assert!(text.contains("DESCRIPTION:02.11. - Spät\\, Schicht\\; 2 - 15:00 - 23:00\r\n"));
    }

    #[test]
    fn an_end_before_the_start_runs_past_midnight() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let mut cells = vec![cell(day, KIND_SHIFT)];
        cells[0].starts_at = Some("23:00".into());
        cells[0].ends_at = Some("02:00".into());
        let month = month_of(&employee(), &cells, &[shift("Nacht")]);
        let text = ics(&month, day.and_hms_opt(9, 0, 0).unwrap());
        assert!(text.contains("DTSTART:20261102T230000\r\n"), "{text}");
        assert!(text.contains("DTEND:20261103T020000\r\n"), "{text}");
    }

    #[test]
    fn the_uid_stays_the_same_for_the_same_entry() {
        let day = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
        let month = IcalMonth {
            employee: "Mauris Apolloni".into(),
            personnel_no: String::new(),
            days: vec![IcalDay {
                day,
                shift: "Spätschicht".into(),
                starts_at: "15:00".into(),
                ends_at: "23:00".into(),
            }],
        };
        let first = ics(&month, day.and_hms_opt(9, 0, 0).unwrap());
        let second = ics(&month, day.and_hms_opt(10, 30, 0).unwrap());
        let uid_of = |text: &str| {
            text.lines()
                .find(|row| row.starts_with("UID:"))
                .unwrap()
                .to_string()
        };
        assert_eq!(uid_of(&first), uid_of(&second));
        assert_eq!(calendar_name(&month), "Dienstplan – Mauris Apolloni");
        assert_eq!(subject(&month), "Dienstplan - Mauris Apolloni");
    }
}
