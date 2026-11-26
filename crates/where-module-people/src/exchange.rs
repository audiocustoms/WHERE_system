//! The exchange files of the schedule plan: one month out of the board and back in.
//!
//! Two sheets leave the module and come back unchanged, so a file written on one
//! machine can be read on the other without editing it:
//!
//! * the disposition — `Standort; Datum; Beginn; Titel; Kennzeichen`, one row per
//!   event, in the order the board shows them;
//! * the plan — `Personalnummer; Name; Datum; Art; Schicht; Beginn; Ende; Pause;
//!   Notiz`, one row per cell.
//!
//! Both are written as CSV (`;`, UTF-8 with BOM) and as XLSX with one sheet; the
//! reader accepts either — the file is recognised by its first bytes.

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use where_domain::DomainError;

use crate::dispo::{self, DispoEvent, DispoLocation, DispoScan};

/// Head of the disposition sheet.
pub const DISPO_HEAD: [&str; 5] = ["Standort", "Datum", "Beginn", "Titel", "Kennzeichen"];

/// The sixth column of the disposition sheet, written only when the event was set
/// aside. It is optional: a sheet without it imports every event as planned.
const DISPO_IGNORED: &str = "Ignoriert";

/// Head of the plan sheet.
pub const PLAN_HEAD: [&str; 9] = [
    "Personalnummer",
    "Name",
    "Datum",
    "Art",
    "Schicht",
    "Beginn",
    "Ende",
    "Pause",
    "Notiz",
];

/// What an exchange import did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ExchangeReport {
    /// Rows the file carried, without its head.
    pub rows: usize,
    /// Rows that were written.
    pub written: usize,
    /// Everything that was skipped, with the line of the file.
    pub warnings: Vec<String>,
}

/// The month an exchange file is written into, with the switches of the window.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, IntoParams)]
pub struct ExchangeQuery {
    /// Month the file is written into (1–12).
    pub month: u32,
    pub year: i32,
    /// Empty the month before writing; without it the rows are added.
    #[serde(default)]
    pub replace: bool,
    /// Drop disposition events that carry no start time.
    #[serde(default)]
    pub only_with_time: bool,
}

impl ExchangeQuery {
    /// The first and the last day of the month.
    pub fn range(&self) -> Result<(NaiveDate, NaiveDate), DomainError> {
        crate::schedule::month_range(self.month, self.year)
    }
}

/// What a preview of an exchange file shows before anything is written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ExchangeScan {
    /// Rows of the file that can be written.
    pub rows: usize,
    /// Distinct days those rows cover.
    pub days: usize,
    /// What the rows belong to: the locations of a disposition, the people of a plan.
    pub groups: Vec<ExchangeGroup>,
    /// Everything that will be skipped, with the line of the file.
    pub warnings: Vec<String>,
    /// Entries the month already holds.
    pub existing_entries: usize,
    /// `true` when this very file was already written into the month.
    pub already_imported: bool,
}

/// One name of a preview with the rows behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ExchangeGroup {
    pub name: String,
    pub rows: usize,
}

/// One cell of the plan as the sheet carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    /// Personnel number of the employee; the name is only carried along.
    pub personnel_no: String,
    pub name: String,
    pub day: NaiveDate,
    /// `Schicht`, `Zeitfenster`, `Krank` or `Urlaub`.
    pub art: PlanArt,
    /// Short name of the shift, for `Schicht`.
    pub shift: Option<String>,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub break_minutes: Option<i16>,
    pub note: Option<String>,
    /// Line of the file, counted from the first row after the head.
    pub line: usize,
}

/// What stands in a cell of the plan sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanArt {
    Shift,
    Free,
    Sick,
    Vacation,
}

impl PlanArt {
    /// Reads the word of the `Art` column.
    fn parse(text: &str) -> Option<Self> {
        match text.trim().to_lowercase().as_str() {
            "schicht" | "shift" | "dienst" => Some(Self::Shift),
            "zeitfenster" | "frei" | "free" => Some(Self::Free),
            "krank" | "sick" => Some(Self::Sick),
            "urlaub" | "vacation" => Some(Self::Vacation),
            _ => None,
        }
    }

    /// The word the sheet carries.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shift => "Schicht",
            Self::Free => "Zeitfenster",
            Self::Sick => "Krank",
            Self::Vacation => "Urlaub",
        }
    }
}

/// Reads the grid of the first sheet — XLSX when the file starts like a zip, CSV
/// otherwise. Blank rows are dropped.
pub fn read_table(bytes: &[u8]) -> Result<Vec<Vec<String>>, DomainError> {
    let rows = if bytes.starts_with(b"PK\x03\x04") {
        let sheet = dispo::read_sheet(bytes)?;
        grid(&sheet)
    } else {
        csv(&String::from_utf8_lossy(bytes))?
    };
    Ok(rows
        .into_iter()
        .filter(|row| row.iter().any(|cell| !cell.trim().is_empty()))
        .collect())
}

/// German month names, as the notes of the exchange write them.
const MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

/// A month and its year, as a note names it: Dezember 2026.
pub fn month_and_year(day: NaiveDate) -> String {
    format!("{} {}", MONTHS[day.month0() as usize], day.year())
}

/// The day a row of a file lands on in the chosen month: a row of that month keeps its
/// day, a row of another month moves to the same day of the chosen month — so the
/// disposition of December becomes the disposition of January. `None` when the chosen
/// month has no such day (the 31st of February).
pub fn day_in_month(day: NaiveDate, from: NaiveDate, to: NaiveDate) -> Option<NaiveDate> {
    if day >= from && day <= to {
        return Some(day);
    }
    NaiveDate::from_ymd_opt(from.year(), from.month(), day.day())
        .filter(|ziel| *ziel >= from && *ziel <= to)
}

/// One line for the window when the days of a file had to move onto the chosen month.
pub fn moved_note(source: NaiveDate, from: NaiveDate, moved: usize) -> Option<String> {
    if moved == 0 || source.year() == from.year() && source.month() == from.month() {
        return None;
    }
    Some(format!(
        "Die Tage der Datei lagen im {}; geschrieben wurde der {} ({moved} Zeilen).",
        month_and_year(source),
        month_and_year(from)
    ))
}

/// The disposition sheet as the import of the module knows it. Days of another month
/// are laid onto the chosen one, see day_in_month; with `only_with_time` an event
/// whose text carries no time is dropped.
pub fn read_dispo(
    bytes: &[u8],
    from: NaiveDate,
    to: NaiveDate,
    only_with_time: bool,
) -> Result<DispoScan, DomainError> {
    let rows = read_table(bytes)?;
    let Some((head, start)) = head_of(&rows, &DISPO_HEAD) else {
        return Err(DomainError::Validation(format!(
            "Die Kopfzeile „{}“ fehlt.",
            DISPO_HEAD.join("; ")
        )));
    };
    let mut locations: Vec<DispoLocation> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut moved = 0usize;
    let mut first_foreign: Option<NaiveDate> = None;
    let ignored_column = optional_column(&rows, start, DISPO_IGNORED);
    for (index, row) in rows.iter().enumerate().skip(start) {
        let line = index + 1;
        let location = cell(row, head[0]).trim();
        let day_text = cell(row, head[1]).trim();
        if location.is_empty() && day_text.is_empty() {
            continue;
        }
        let Some(gelesen) = parse_day(day_text) else {
            warnings.push(format!("Zeile {line}: „{day_text}“ ist kein Datum."));
            continue;
        };
        let Some(day) = day_in_month(gelesen, from, to) else {
            warnings.push(format!(
                "Zeile {line}: den {}. {} gibt es nicht.",
                gelesen.day(),
                month_and_year(from)
            ));
            continue;
        };
        if day != gelesen {
            moved += 1;
            first_foreign.get_or_insert(gelesen);
        }
        let title = cell(row, head[3]).trim();
        if title.is_empty() {
            warnings.push(format!("Zeile {line}: der Titel fehlt."));
            continue;
        }
        if location.is_empty() {
            warnings.push(format!("Zeile {line}: der Standort fehlt."));
            continue;
        }
        let starts_at = clock(cell(row, head[2]));
        let flag = Some(cell(row, head[4]).trim().to_string()).filter(|code| !code.is_empty());
        let ignored = ignored_column
            .map(|column| is_yes(cell(row, column)))
            .unwrap_or(false);
        let event = DispoEvent {
            day,
            starts_at,
            title: title.to_string(),
            flag,
            ignored,
        };
        match locations.iter_mut().find(|entry| entry.name == location) {
            Some(entry) => {
                entry.last_row = line as u32;
                entry.events.push(event);
            }
            None => locations.push(DispoLocation {
                name: location.to_string(),
                first_row: line as u32,
                last_row: line as u32,
                events: vec![event],
            }),
        }
    }
    if let Some(source) = first_foreign {
        if let Some(note) = moved_note(source, from, moved) {
            warnings.push(note);
        }
    }
    let mut dropped = 0usize;
    if only_with_time {
        for location in &mut locations {
            let before = location.events.len();
            location.events.retain(|event| event.starts_at.is_some());
            dropped += before - location.events.len();
        }
        locations.retain(|location| !location.events.is_empty());
        if dropped > 0 {
            warnings.push(format!("{dropped} Termine ohne Uhrzeit übersprungen."));
        }
    }
    let mut days: Vec<NaiveDate> = Vec::new();
    for location in &locations {
        for event in &location.events {
            if !days.contains(&event.day) {
                days.push(event.day);
            }
        }
    }
    Ok(DispoScan {
        locations,
        days: days.len(),
        warnings,
        already_imported: false,
        existing_entries: 0,
    })
}

/// The plan sheet as it was read: its rows and everything that stayed out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanSheet {
    pub rows: Vec<PlanRow>,
    pub warnings: Vec<String>,
}

/// The plan sheet, row by row.
pub fn read_plan(bytes: &[u8]) -> Result<PlanSheet, DomainError> {
    let rows = read_table(bytes)?;
    let Some((head, start)) = head_of(&rows, &PLAN_HEAD) else {
        return Err(DomainError::Validation(format!(
            "Die Kopfzeile „{}“ fehlt.",
            PLAN_HEAD.join("; ")
        )));
    };
    let mut sheet = PlanSheet::default();
    for (index, row) in rows.iter().enumerate().skip(start) {
        let line = index + 1;
        let personnel_no = cell(row, head[0]).trim();
        let day_text = cell(row, head[2]).trim();
        if personnel_no.is_empty() && day_text.is_empty() {
            continue;
        }
        let Some(day) = parse_day(day_text) else {
            sheet
                .warnings
                .push(format!("Zeile {line}: „{day_text}“ ist kein Datum."));
            continue;
        };
        let art_text = cell(row, head[3]);
        let Some(art) = PlanArt::parse(art_text) else {
            sheet.warnings.push(format!(
                "Zeile {line}: die Art „{}“ ist unbekannt.",
                art_text.trim()
            ));
            continue;
        };
        sheet.rows.push(PlanRow {
            personnel_no: personnel_no.to_string(),
            name: cell(row, head[1]).trim().to_string(),
            day,
            art,
            shift: Some(cell(row, head[4]).trim().to_string()).filter(|name| !name.is_empty()),
            starts_at: clock(cell(row, head[5])),
            ends_at: clock(cell(row, head[6])),
            break_minutes: parse_minutes(cell(row, head[7])),
            note: Some(cell(row, head[8]).trim().to_string()).filter(|text| !text.is_empty()),
            line,
        });
    }
    Ok(sheet)
}

/// The preview of a disposition file: rows, days and locations, and what stays out.
pub fn preview_dispo(scan: &DispoScan) -> ExchangeScan {
    ExchangeScan {
        rows: scan
            .locations
            .iter()
            .map(|location| location.events.len())
            .sum(),
        days: scan.days,
        groups: scan
            .locations
            .iter()
            .map(|location| ExchangeGroup {
                name: location.name.clone(),
                rows: location.events.len(),
            })
            .collect(),
        warnings: scan.warnings.clone(),
        ..ExchangeScan::default()
    }
}

/// The preview of a plan file: rows, days and people, and what stays out.
pub fn preview_plan(sheet: &PlanSheet, from: NaiveDate, to: NaiveDate) -> ExchangeScan {
    let mut groups: Vec<ExchangeGroup> = Vec::new();
    let mut days: Vec<NaiveDate> = Vec::new();
    let mut rows = 0usize;
    for row in &sheet.rows {
        let Some(day) = day_in_month(row.day, from, to) else {
            continue;
        };
        rows += 1;
        if !days.contains(&day) {
            days.push(day);
        }
        match groups.iter_mut().find(|group| group.name == row.name) {
            Some(group) => group.rows += 1,
            None => groups.push(ExchangeGroup {
                name: row.name.clone(),
                rows: 1,
            }),
        }
    }
    ExchangeScan {
        rows,
        days: days.len(),
        groups,
        warnings: sheet.warnings.clone(),
        ..ExchangeScan::default()
    }
}

// ------------------------------------------------------------------ reading

/// The XLSX of the module as rows of text.
fn grid(sheet: &dispo::Sheet) -> Vec<Vec<String>> {
    let rows = sheet.cells.keys().map(|(row, _)| *row).max().unwrap_or(0);
    let columns = sheet
        .cells
        .keys()
        .map(|(_, column)| *column)
        .max()
        .unwrap_or(0);
    (1..=rows)
        .map(|row| {
            (1..=columns)
                .map(|column| sheet.cells.get(&(row, column)).cloned().unwrap_or_default())
                .collect()
        })
        .collect()
}

/// The lines of a German CSV: `;` (or `,` when the head uses it), quotes where the
/// text carries the separator.
fn csv(text: &str) -> Result<Vec<Vec<String>>, DomainError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let separator = if text.lines().next().is_some_and(|line| line.contains(';')) {
        ';'
    } else {
        ','
    };
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '"' if quoted => {
                if characters.peek() == Some(&'"') {
                    characters.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            }
            '"' => quoted = true,
            '\r' if !quoted => {}
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ if character == separator && !quoted => row.push(std::mem::take(&mut field)),
            _ => field.push(character),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

/// The row with the head and the index of the first data row.
pub(crate) fn head_of(rows: &[Vec<String>], head: &[&str]) -> Option<(Vec<usize>, usize)> {
    for (index, row) in rows.iter().enumerate() {
        let texts: Vec<String> = row.iter().map(|cell| cell.trim().to_lowercase()).collect();
        let columns: Option<Vec<usize>> = head
            .iter()
            .map(|name| texts.iter().position(|text| text == &name.to_lowercase()))
            .collect();
        if let Some(columns) = columns {
            return Some((columns, index + 1));
        }
    }
    None
}

pub(crate) fn cell(row: &[String], index: usize) -> &str {
    row.get(index).map(String::as_str).unwrap_or("")
}

/// The column of an optional head name, `None` when the sheet does not carry it.
fn optional_column(rows: &[Vec<String>], start: usize, name: &str) -> Option<usize> {
    let head = rows.get(start.checked_sub(1)?)?;
    head.iter()
        .position(|text| text.trim().to_lowercase() == name.to_lowercase())
}

/// `ja`, `yes`, `x` or `1` say yes; everything else, an empty cell included, no.
pub(crate) fn is_yes(text: &str) -> bool {
    matches!(
        text.trim().to_lowercase().as_str(),
        "ja" | "yes" | "x" | "1" | "true" | "wahr"
    )
}

/// `2026-11-16` or `16.11.2026`.
fn parse_day(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(text.trim(), "%d.%m.%Y"))
        .ok()
}

/// A clock as `HH:MM`; anything else is no time.
fn clock(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let cleaned = text.replace('.', ":");
    let (hours, minutes) = cleaned.split_once(':')?;
    let hours: u32 = hours.trim().parse().ok()?;
    let minutes: u32 = minutes.trim().parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(format!("{hours:02}:{minutes:02}"))
}

fn parse_minutes(text: &str) -> Option<i16> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    text.parse::<i16>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(rows: &[&str]) -> Vec<u8> {
        rows.join("\r\n").into_bytes()
    }

    /// The first and the last day of a month.
    fn month(year: i32, month: u32) -> (NaiveDate, NaiveDate) {
        let from = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let next = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
        };
        (from, next.pred_opt().unwrap())
    }

    #[test]
    fn the_disposition_sheet_comes_back_as_it_left() {
        let bytes = text(&[
            "Standort;Datum;Beginn;Titel;Kennzeichen",
            "Großes Haus;2026-11-02;12:00;Streetcar BP;",
            "Großes Haus;2026-11-02;;Einweisung;",
            "Podium;2026-11-03;18:00;Schönste Versi…;Vst",
        ]);
        let (from, to) = month(2026, 11);
        let scan = read_dispo(&bytes, from, to, false).unwrap();
        assert_eq!(scan.locations.len(), 2);
        assert_eq!(scan.locations[0].name, "Großes Haus");
        assert_eq!(scan.locations[0].events.len(), 2);
        assert_eq!(
            scan.locations[0].events[0].starts_at.as_deref(),
            Some("12:00")
        );
        assert!(scan.locations[0].events[1].starts_at.is_none());
        assert_eq!(scan.locations[1].events[0].flag.as_deref(), Some("Vst"));
        assert_eq!(scan.days, 2);
        assert!(scan.warnings.is_empty(), "{:?}", scan.warnings);
    }

    #[test]
    fn a_broken_disposition_row_is_named() {
        let bytes = text(&[
            "Standort;Datum;Beginn;Titel;Kennzeichen",
            "Podium;32.13.2026;18:00;Test;",
            "Podium;2026-11-03;18:00;;",
        ]);
        let (from, to) = month(2026, 11);
        let scan = read_dispo(&bytes, from, to, false).unwrap();
        assert!(scan.locations.is_empty());
        assert_eq!(scan.warnings.len(), 2);
        assert!(scan.warnings[0].starts_with("Zeile 2"));
        assert!(scan.warnings[1].starts_with("Zeile 3"));
    }

    #[test]
    fn the_plan_sheet_reads_every_art() {
        let bytes = text(&[
            "Personalnummer;Name;Datum;Art;Schicht;Beginn;Ende;Pause;Notiz",
            "1022;Volk, Emma;2026-11-16;Schicht;S-AZ;;;;Tausch",
            "67;Denk, M.;2026-11-23;Zeitfenster;;15:00;19:00;30;",
            "68;Pfüger;2026-11-05;Krank;;;;;",
            "69;Apolloni;2026-11-06;Urlaub;;;;;",
        ]);
        let sheet = read_plan(&bytes).unwrap();
        let plan = &sheet.rows;
        assert_eq!(plan.len(), 4);
        assert_eq!(plan[0].art, PlanArt::Shift);
        assert_eq!(plan[0].shift.as_deref(), Some("S-AZ"));
        assert_eq!(plan[0].note.as_deref(), Some("Tausch"));
        assert_eq!(plan[1].art, PlanArt::Free);
        assert_eq!(plan[1].starts_at.as_deref(), Some("15:00"));
        assert_eq!(plan[1].break_minutes, Some(30));
        assert_eq!(plan[2].art, PlanArt::Sick);
        assert_eq!(plan[3].art, PlanArt::Vacation);
        assert!(sheet.warnings.is_empty(), "{:?}", sheet.warnings);
    }

    #[test]
    fn a_comma_separated_file_and_german_dates_are_read_too() {
        let bytes = text(&[
            "Standort,Datum,Beginn,Titel,Kennzeichen",
            "Podium,03.11.2026,18:00,\"Versi, groß\",Vst",
        ]);
        let (from, to) = month(2026, 11);
        let scan = read_dispo(&bytes, from, to, false).unwrap();
        assert_eq!(scan.locations[0].events[0].title, "Versi, groß");
        assert_eq!(scan.locations[0].events[0].day.to_string(), "2026-11-03");
    }

    #[test]
    fn a_set_aside_event_stays_set_aside() {
        let bytes = text(&[
            "Standort;Datum;Beginn;Titel;Kennzeichen;Ignoriert",
            "Podium;2026-11-03;18:00;Streetcar;Vst;ja",
            "Podium;2026-11-03;20:00;Zusatz;;",
        ]);
        let (from, to) = month(2026, 11);
        let scan = read_dispo(&bytes, from, to, false).unwrap();
        assert!(scan.locations[0].events[0].ignored);
        assert!(!scan.locations[0].events[1].ignored);
    }

    #[test]
    fn a_disposition_of_another_month_lands_on_the_chosen_one() {
        let bytes = text(&[
            "Standort;Datum;Beginn;Titel;Kennzeichen",
            "Großes Haus;01.12.2026;19:30;Teststück;Vst",
            "Podium;31.12.2026;;Einweisung;",
        ]);
        let (from, to) = month(2027, 1);
        let scan = read_dispo(&bytes, from, to, false).unwrap();
        assert_eq!(scan.locations[0].events[0].day.to_string(), "2027-01-01");
        assert_eq!(scan.locations[1].events[0].day.to_string(), "2027-01-31");
        assert_eq!(scan.warnings.len(), 1);
        assert!(
            scan.warnings[0].contains("Dezember 2026"),
            "{:?}",
            scan.warnings
        );
        assert!(scan.warnings[0].contains("Januar 2027"));

        // The 31st is missing in February — the row stays out and is named.
        let (from, to) = month(2027, 2);
        let scan = read_dispo(&bytes, from, to, false).unwrap();
        assert_eq!(scan.locations.len(), 1);
        assert!(scan.warnings[0].starts_with("Zeile 3"));
    }

    #[test]
    fn a_preview_names_the_locations_and_what_stays_out() {
        let bytes = text(&[
            "Standort;Datum;Beginn;Titel;Kennzeichen",
            "Großes Haus;01.11.2026;12:00;Streetcar BP;",
            "Großes Haus;02.11.2026;12:00;Streetcar BP;",
            "Podium;32.13.2026;18:00;Test;",
        ]);
        let (from, to) = month(2026, 11);
        let scan = read_dispo(&bytes, from, to, false).unwrap();
        let preview = preview_dispo(&scan);
        assert_eq!(preview.rows, 2);
        assert_eq!(preview.days, 2);
        assert_eq!(preview.groups.len(), 1);
        assert_eq!(preview.groups[0].name, "Großes Haus");
        assert_eq!(preview.groups[0].rows, 2);
        assert_eq!(preview.warnings.len(), 1);
    }

    #[test]
    fn rows_without_a_time_can_stay_out() {
        let bytes = text(&[
            "Standort;Datum;Beginn;Titel;Kennzeichen",
            "Podium;03.11.2026;18:00;Streetcar;Vst",
            "Podium;03.11.2026;;i.A. an die Vorstellung;",
        ]);
        let (from, to) = month(2026, 11);
        let scan = read_dispo(&bytes, from, to, true).unwrap();
        assert_eq!(scan.locations[0].events.len(), 1);
        assert_eq!(scan.days, 1);
        assert!(
            scan.warnings[0].contains("ohne Uhrzeit"),
            "{:?}",
            scan.warnings
        );

        let scan = read_dispo(&bytes, from, to, false).unwrap();
        assert_eq!(scan.locations[0].events.len(), 2);
    }

    #[test]
    fn the_preview_of_a_plan_counts_people_and_days() {
        let bytes = text(&[
            "Personalnummer;Name;Datum;Art;Schicht;Beginn;Ende;Pause;Notiz",
            "1001;Denk, Marcus;02.11.2026;Schicht;F;;;;",
            "1001;Denk, Marcus;03.11.2026;Schicht;F;;;;",
            "1006;Apolloni, Mauris;02.11.2026;Krank;;;;;",
            "1007;Unbekannt;02.11.2026;Feiertag;;;;;",
        ]);
        let sheet = read_plan(&bytes).unwrap();
        assert_eq!(sheet.rows.len(), 3);
        assert_eq!(sheet.warnings.len(), 1);
        assert!(sheet.warnings[0].starts_with("Zeile 5"));
        let (from, to) = month(2026, 11);
        let preview = preview_plan(&sheet, from, to);
        assert_eq!(preview.rows, 3);
        assert_eq!(preview.days, 2);
        assert_eq!(preview.groups.len(), 2);
        assert_eq!(preview.groups[0].rows, 2);
    }

    #[test]
    fn a_file_without_a_head_is_refused() {
        let bytes = text(&["Irgendwas;2026-11-02"]);
        let (from, to) = month(2026, 11);
        assert!(read_dispo(&bytes, from, to, false).is_err());
        assert!(read_plan(&bytes).is_err());
    }
}
