//! The PeM disposition export (KOKOS.event PeM).
//!
//! PeM writes the disposition as a print layout, not as a table: above it the
//! date header, below it one block per location (`Großes Haus`, `Podium`, …) and
//! under that the staff plan, which this module ignores. The height of a block
//! varies with the number of events, so it is not read from the content: PeM
//! merges the location cell over exactly the rows of its block (`A3:D8`), and
//! that merge is what this parser follows. A block therefore needs no guessing
//! and survives any number of event rows, empty rows included.
//!
//! Every day of the sheet uses two columns (`E:F`, `G:H`, …); the text stands in
//! the left one. The day of a column comes from the date header (`01.12`), the
//! year from the caller — the export itself carries none.
//!
//! Every filled cell becomes its own entry: the time is split off, so both stay
//! editable in the board.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use chrono::NaiveDate;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

use crate::flags;
use crate::shifts;

/// The longest title an event takes.
pub const MAX_TITLE_LEN: usize = 200;

/// How many entries one location may have on one day.
pub const MAX_ENTRIES: usize = 50;

/// A cell of the sheet: `(row, column)`, both counted from 1.
type Cell = (u32, u32);

/// A merged range, counted from 1 and including both ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Merge {
    row_from: u32,
    col_from: u32,
    row_to: u32,
    col_to: u32,
}

/// One event of a location on one day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DispoEvent {
    /// The day the event belongs to (`YYYY-MM-DD`).
    pub day: NaiveDate,
    /// Start as `HH:MM`; empty when the cell carried no time.
    pub starts_at: Option<String>,
    /// The text without its time, for example `Max und Moritz Vst`.
    pub title: String,
    /// The flag recognised at the end of the text, for example `Vst`.
    pub flag: Option<String>,
    /// `true` when the planner set the event aside: it asks for nobody.
    pub ignored: bool,
}

/// A location block of the disposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DispoLocation {
    /// The name from the first cell of the block, without the tail PeM repeats
    /// on every location, for example `Großes Haus`.
    pub name: String,
    /// First row of the block.
    pub first_row: u32,
    /// Last row of the block.
    pub last_row: u32,
    /// Every filled day cell of the block, in reading order.
    pub events: Vec<DispoEvent>,
}

impl DispoLocation {
    /// Number of rows the block occupies.
    pub fn rows(&self) -> u32 {
        self.last_row - self.first_row + 1
    }
}

/// What the sheet contained, without writing anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DispoScan {
    /// The locations in the order of the sheet.
    pub locations: Vec<DispoLocation>,
    /// Number of days found in the date header.
    pub days: usize,
    /// Everything that was skipped.
    pub warnings: Vec<String>,
    /// `true` when a disposition with the same content was already imported for
    /// this month; writing it again without `replace` is refused.
    #[serde(default)]
    pub already_imported: bool,
    /// Entries the month already holds, written by an earlier import.
    #[serde(default)]
    pub existing_entries: usize,
}

/// What the import did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DispoImportReport {
    /// Locations found in the sheet.
    pub locations: usize,
    /// Locations that did not exist yet and were added to the list.
    pub locations_created: usize,
    /// Flags that did not exist yet and were created from the legend because the
    /// disposition uses them.
    #[serde(default)]
    pub flags_created: usize,
    /// Days of the date header.
    pub days: usize,
    /// Entries that were written.
    pub entries_created: usize,
    /// Everything that was skipped.
    pub warnings: Vec<String>,
}

/// One entry of the disposition: an event of a location on a day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DispoEntry {
    pub id: i64,
    pub location_id: i64,
    /// The day the event takes place (`YYYY-MM-DD`).
    pub day: NaiveDate,
    /// Start as `HH:MM`; empty when the event has no time.
    pub starts_at: Option<String>,
    /// The event without its time.
    pub title: String,
    /// Code of the flag, for example `Vst`; empty when the text carries none.
    pub flag: Option<String>,
    /// Order within its day.
    pub sort_order: i32,
    /// Set aside by the planner: the event asks for nobody and marks no day as
    /// unclear.
    pub ignored: bool,
}

/// One entry as the window sends it back; without `id` it is new.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DispoEntryInput {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub starts_at: String,
    pub title: String,
    /// Code of the flag (`Vst`); empty when the entry carries none.
    #[serde(default)]
    pub flag: Option<String>,
    /// Set aside: kept, but it asks for nobody and marks no day as unclear.
    #[serde(default)]
    pub ignored: bool,
}

/// The entries of one location on one day; the list replaces that day.
#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct DispoDayInput {
    #[serde(default)]
    pub entries: Vec<DispoEntryInput>,
}

/// Whether an event of the disposition is set aside.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct DispoIgnoredInput {
    /// `true` keeps the event but stops it from asking for anybody.
    pub ignored: bool,
}

/// A checked entry for the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanDispoEntry {
    pub id: Option<i64>,
    /// `HH:MM`, empty when the event has no time.
    pub starts_at: Option<String>,
    pub title: String,
    /// Code of the flag; `None` when the entry carries none.
    pub flag: Option<String>,
    /// Set aside by the planner.
    pub ignored: bool,
}

impl DispoEntryInput {
    /// Trims the title, reads the time in any usual form and checks both.
    pub fn clean(&self) -> Result<CleanDispoEntry, DomainError> {
        let title = cap(self.title.trim(), MAX_TITLE_LEN);
        if title.is_empty() {
            return Err(DomainError::Validation(
                "An entry needs a text.".to_string(),
            ));
        }
        let starts_at = self.starts_at.trim();
        let starts_at =
            if starts_at.is_empty() {
                None
            } else {
                let normalized = starts_at.replace('.', ":");
                Some(shifts::clock(&normalized).ok_or_else(|| {
                    DomainError::Validation(format!("Unknown time “{starts_at}”."))
                })?)
            };
        Ok(CleanDispoEntry {
            id: self.id,
            starts_at,
            title,
            flag: self
                .flag
                .as_deref()
                .map(str::trim)
                .filter(|flag| !flag.is_empty())
                .map(str::to_string),
            ignored: self.ignored,
        })
    }
}

impl DispoDayInput {
    /// Checks every entry; a day takes at most [`MAX_ENTRIES`] of them.
    pub fn clean(&self) -> Result<Vec<CleanDispoEntry>, DomainError> {
        if self.entries.len() > MAX_ENTRIES {
            return Err(DomainError::Validation(format!(
                "A day takes at most {MAX_ENTRIES} entries."
            )));
        }
        self.entries.iter().map(DispoEntryInput::clean).collect()
    }
}

/// The month and the year a file is booked on.
#[derive(Debug, Clone, Copy, Deserialize, utoipa::IntoParams)]
pub struct DispoQuery {
    /// Month the file belongs to (1–12).
    pub month: u32,
    /// Year the file belongs to; the export itself carries none.
    pub year: i32,
    /// Empty the entries of that month before writing.
    #[serde(default)]
    pub replace: bool,
    /// Drop cell texts that do not start with a time.
    #[serde(default)]
    pub only_with_time: bool,
}

impl DispoQuery {
    /// The first and the last day of the month.
    pub fn month_range(&self) -> Result<(NaiveDate, NaiveDate), DomainError> {
        crate::schedule::month_range(self.month, self.year)
    }
}

/// Reads the disposition of an xlsx file and books it on the given month.
///
/// `codes` are the flag codes of the master data; a text that ends with one of
/// them carries that flag. With `only_with_time` a cell whose text does not
/// start with a time is dropped — `i.A. an die Vorstellung` carries no hour and
/// would only add noise.
pub fn scan(
    bytes: &[u8],
    month: u32,
    year: i32,
    only_with_time: bool,
    codes: &[String],
) -> Result<DispoScan, DomainError> {
    if !(1..=12).contains(&month) {
        return Err(DomainError::Validation(
            "The month has to lie between 1 and 12.".to_string(),
        ));
    }
    let sheet = read_sheet(bytes)?;
    let mut scan = detect(&sheet, month, year, codes);
    if only_with_time {
        let mut skipped = 0;
        for location in &mut scan.locations {
            let before = location.events.len();
            location.events.retain(|event| event.starts_at.is_some());
            skipped += before - location.events.len();
        }
        if skipped > 0 {
            scan.warnings
                .push(format!("{skipped} Termine ohne Uhrzeit übersprungen."));
        }
    }
    Ok(scan)
}

/// `true` when the text starts with a time (`09:00 …`, `9.00 …`).
pub fn has_time(text: &str) -> bool {
    split_time(text).0.is_some()
}

/// Splits `09:00 Max und Moritz Vst` into its time and its text.
///
/// A cell that only carries a time keeps it as text — an entry without a text
/// would be lost otherwise.
fn split_time(text: &str) -> (Option<String>, String) {
    let text = text.trim();
    let bytes = text.as_bytes();
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits == 0 || digits > 2 || bytes.len() < digits + 3 {
        return (None, text.to_string());
    }
    let separator = bytes[digits];
    if (separator != b':' && separator != b'.')
        || !bytes[digits + 1].is_ascii_digit()
        || !bytes[digits + 2].is_ascii_digit()
    {
        return (None, text.to_string());
    }
    let clock = text[..digits + 3].replace('.', ":");
    let Some(starts_at) = shifts::clock(&clock) else {
        return (None, text.to_string());
    };
    let rest = text[digits + 3..].trim();
    if rest.is_empty() {
        return (None, text.to_string());
    }
    (Some(starts_at), cap(rest, MAX_TITLE_LEN))
}

/// Shortens a text to the length a title takes.
fn cap(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// The blocks, their days and their events.
fn detect(sheet: &Sheet, month: u32, year: i32, codes: &[String]) -> DispoScan {
    let mut warnings: Vec<String> = Vec::new();

    let mut days: Vec<(u32, NaiveDate)> = Vec::new();
    let mut header_row = 0;
    for ((row, col), text) in &sheet.cells {
        if *row > 2 {
            continue;
        }
        let Some(day) = day_of_month(text) else {
            continue;
        };
        header_row = header_row.max(*row);
        match NaiveDate::from_ymd_opt(year, month, day) {
            Some(date) => days.push((*col, date)),
            None => warnings.push(format!(
                "Der {day}. gibt es im gewählten Monat nicht — der Tag bleibt leer."
            )),
        }
    }
    days.sort_by_key(|(col, _)| *col);

    if days.is_empty() {
        warnings.push(
            "In der Kopfzeile steht kein Datum — ist das wirklich der PeM-Export?".to_string(),
        );
    }

    let band_end = band_end(sheet, header_row);
    let mut blocks: Vec<Merge> = sheet
        .merges
        .iter()
        .copied()
        .filter(|merge| merge.col_from == 1 && merge.col_to >= 2 && merge.row_to >= merge.row_from)
        .filter(|merge| merge.row_from > header_row && merge.row_from < band_end)
        .collect();
    blocks.sort_by_key(|merge| (merge.row_from, merge.col_from));

    let mut locations: Vec<DispoLocation> = Vec::new();
    for block in blocks {
        let name = sheet
            .cells
            .get(&(block.row_from, block.col_from))
            .map(|text| text.trim().to_string())
            .unwrap_or_default();
        if name.is_empty() {
            warnings.push(format!(
                "Zeile {} beginnt einen Block ohne Standortnamen — übersprungen.",
                block.row_from
            ));
            continue;
        }
        if let Some(previous) = locations
            .iter()
            .find(|known| block.row_from <= known.last_row)
        {
            warnings.push(format!(
                "Der Block „{name}“ ab Zeile {} überschneidet „{}“ — übersprungen.",
                block.row_from, previous.name
            ));
            continue;
        }
        let mut events: Vec<DispoEvent> = Vec::new();
        for (col, day) in &days {
            for row in block.row_from..=block.row_to {
                let Some(text) = sheet.cells.get(&(row, *col)) else {
                    continue;
                };
                let (starts_at, title) = split_time(text);
                if title.is_empty() {
                    continue;
                }
                let flag = flags::recognise(&title, codes);
                events.push(DispoEvent {
                    day: *day,
                    starts_at,
                    title,
                    flag,
                    ignored: false,
                });
            }
        }
        locations.push(DispoLocation {
            name,
            first_row: block.row_from,
            last_row: block.row_to,
            events,
        });
    }

    if locations.is_empty() && !days.is_empty() {
        warnings.push(
            "Kein Standort erkannt — die verbundenen Zellen der Standortspalte fehlen.".to_string(),
        );
    }

    trim_shared_suffix(&mut locations);

    DispoScan {
        locations,
        days: days.len(),
        warnings,
        already_imported: false,
        existing_entries: 0,
    }
}

/// Fingerprint of a scanned disposition: the same planning always gives the same
/// value, whatever PeM wrote around it in the xlsx container.
///
/// It is no security hash; it only has to tell one disposition of a month from
/// another, so that importing the same file twice can be refused.
pub fn fingerprint(scan: &DispoScan) -> String {
    fn feed(hash: &mut u64, text: &str) {
        for byte in text.as_bytes() {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // A separator keeps `ab` + `c` from hashing like `a` + `bc`.
        *hash ^= 0x1f;
        *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for location in &scan.locations {
        feed(&mut hash, &location.name);
        for event in &location.events {
            feed(&mut hash, &event.day.to_string());
            feed(&mut hash, event.starts_at.as_deref().unwrap_or(""));
            feed(&mut hash, &event.title);
            feed(&mut hash, event.flag.as_deref().unwrap_or(""));
            feed(&mut hash, if event.ignored { "1" } else { "0" });
        }
    }
    format!("{hash:016x}")
}

/// The flag codes a disposition uses, each one once — the flags its import has to
/// bring into the master data before it can assign them.
pub fn used_flags(scan: &DispoScan) -> Vec<String> {
    let mut codes: Vec<String> = Vec::new();
    for event in scan
        .locations
        .iter()
        .flat_map(|location| location.events.iter())
    {
        let Some(code) = &event.flag else {
            continue;
        };
        if !codes.iter().any(|known| known.eq_ignore_ascii_case(code)) {
            codes.push(code.clone());
        }
    }
    codes
}

/// One disposition entry as the duplicate check compares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispoKey {
    pub location: String,
    pub day: NaiveDate,
    /// `HH:MM`, empty when the event carries no time.
    pub starts_at: Option<String>,
    pub title: String,
    pub flag: Option<String>,
}

impl DispoKey {
    /// The location ignoring case, then day, time, title and flag.
    fn tuple(&self) -> (String, NaiveDate, String, String, String) {
        (
            self.location.trim().to_lowercase(),
            self.day,
            self.starts_at.clone().unwrap_or_default(),
            self.title.trim().to_string(),
            self.flag.as_deref().unwrap_or("").trim().to_lowercase(),
        )
    }
}

/// `true` when a sheet adds nothing: every event of the scan stands in the month
/// already. Appending it would only double the disposition.
pub fn adds_nothing(scan: &DispoScan, existing: &[DispoKey]) -> bool {
    let existing: std::collections::HashSet<_> = existing.iter().map(DispoKey::tuple).collect();
    for location in &scan.locations {
        for event in &location.events {
            let key = DispoKey {
                location: location.name.clone(),
                day: event.day,
                starts_at: event.starts_at.clone(),
                title: event.title.clone(),
                flag: event.flag.clone(),
            };
            if !existing.contains(&key.tuple()) {
                return false;
            }
        }
    }
    true
}

/// PeM appends the house and the city to every location
/// (`Großes Haus, Theater Ulm, Ulm`). The export repeats that tail on every
/// block, so the longest end all names share is dropped — cut back to a comma,
/// so the names become `Großes Haus`, `Podium`, `Podium.bar`, `Oberes Foyer`.
///
/// Nothing is dropped when only one location was found (its tail would be its
/// whole name) or when the names share no end behind a comma.
fn trim_shared_suffix(locations: &mut [DispoLocation]) {
    if locations.len() < 2 || locations.iter().any(|location| location.name.is_empty()) {
        return;
    }
    let mut sorted: Vec<&str> = locations
        .iter()
        .map(|location| location.name.as_str())
        .collect();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return;
    }
    let mut shared: Vec<char> = locations[0].name.chars().collect();
    for location in &locations[1..] {
        let other: Vec<char> = location.name.chars().collect();
        let keep = shared
            .iter()
            .rev()
            .zip(other.iter().rev())
            .take_while(|(left, right)| left == right)
            .count();
        shared.drain(..shared.len() - keep);
        if shared.is_empty() {
            return;
        }
    }
    let Some(start) = shared.iter().position(|character| *character == ',') else {
        return;
    };
    let tail = shared.len() - start;
    let mut names: Vec<String> = Vec::with_capacity(locations.len());
    for location in locations.iter() {
        let count = location.name.chars().count();
        if count <= tail {
            return;
        }
        let name: String = location.name.chars().take(count - tail).collect();
        let name = name.trim_end().to_string();
        if name.is_empty() {
            return;
        }
        names.push(name);
    }
    for (location, name) in locations.iter_mut().zip(names) {
        location.name = name;
    }
}

/// The first row that no longer belongs to the disposition.
///
/// PeM writes the staff plan under the disposition. Its header (`Soll`,
/// `Arbzeit`, `+/-` in B, C and D) marks the end, and so does the first row
/// without any text after the disposition started — whichever comes first.
fn band_end(sheet: &Sheet, header_row: u32) -> u32 {
    let mut rows: Vec<u32> = sheet.cells.keys().map(|(row, _)| *row).collect();
    rows.sort_unstable();
    rows.dedup();
    let last = rows.last().copied().unwrap_or(header_row);
    let mut end = last + 1;

    for row in &rows {
        let is_target = sheet
            .cells
            .get(&(*row, 2))
            .is_some_and(|text| text.trim() == "Soll");
        let is_hours = sheet
            .cells
            .get(&(*row, 3))
            .is_some_and(|text| text.trim() == "Arbzeit");
        if is_target && is_hours {
            end = end.min(*row);
            break;
        }
    }

    let first_block = sheet
        .merges
        .iter()
        .filter(|merge| merge.col_from == 1 && merge.col_to >= 2)
        .map(|merge| merge.row_from)
        .min();
    if let Some(start) = first_block {
        if let Some(blank) = (start + 1..=last).find(|row| !rows.contains(row)) {
            end = end.min(blank);
        }
    }
    end
}

/// The day of a date header cell (`01.12`, `1.12`, `01.12.`).
fn day_of_month(text: &str) -> Option<u32> {
    let text = text.trim().trim_end_matches('.');
    let mut parts = text.split('.');
    let day: u32 = parts.next()?.trim().parse().ok()?;
    let month: u32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(day)
}

// --------------------------------------------------------------------- xlsx

/// The text cells and the merged ranges of the first worksheet.
pub(crate) struct Sheet {
    pub(crate) cells: BTreeMap<Cell, String>,
    merges: Vec<Merge>,
}

/// Opens the xlsx and reads the first worksheet.
pub(crate) fn read_sheet(bytes: &[u8]) -> Result<Sheet, DomainError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| DomainError::Validation(format!("Keine xlsx-Datei: {error}")))?;

    let shared = match archive.by_name("xl/sharedStrings.xml") {
        Ok(mut file) => {
            let mut xml = String::new();
            file.read_to_string(&mut xml)
                .map_err(|error| DomainError::Validation(format!("Unlesbare Datei: {error}")))?;
            shared_strings(&xml)?
        }
        Err(_) => Vec::new(),
    };

    let mut sheet_name: Option<String> = None;
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|error| DomainError::Validation(format!("Unlesbare Datei: {error}")))?;
        let name = match file.name() {
            Ok(name) => name.to_string(),
            Err(_) => continue,
        };
        if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
            let replace = match &sheet_name {
                Some(known) => {
                    name.len() < known.len() || (name.len() == known.len() && name < *known)
                }
                None => true,
            };
            if replace {
                sheet_name = Some(name);
            }
        }
    }
    let Some(sheet_name) = sheet_name else {
        return Err(DomainError::Validation(
            "Die Datei enthält kein Tabellenblatt.".to_string(),
        ));
    };
    let mut xml = String::new();
    archive
        .by_name(&sheet_name)
        .map_err(|error| DomainError::Validation(format!("Unlesbares Blatt: {error}")))?
        .read_to_string(&mut xml)
        .map_err(|error| DomainError::Validation(format!("Unlesbares Blatt: {error}")))?;
    worksheet(&xml, &shared)
}

/// The strings of `xl/sharedStrings.xml`, in their index order.
fn shared_strings(xml: &str) -> Result<Vec<String>, DomainError> {
    let mut reader = Reader::from_str(xml);
    let mut list: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let mut in_text = false;
    loop {
        match reader.read_event().map_err(invalid)? {
            Event::Start(start) => match start.local_name().as_ref() {
                b"si" => current = Some(String::new()),
                b"t" => in_text = true,
                _ => {}
            },
            Event::End(end) => match end.local_name().as_ref() {
                b"t" => in_text = false,
                b"si" => list.push(current.take().unwrap_or_default()),
                _ => {}
            },
            Event::Text(text) => {
                if in_text {
                    let value = text.decode().map_err(invalid)?;
                    if let Some(buffer) = current.as_mut() {
                        buffer.push_str(&value);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(list)
}

/// The cells and merges of a worksheet.
fn worksheet(xml: &str, shared: &[String]) -> Result<Sheet, DomainError> {
    let mut reader = Reader::from_str(xml);
    let mut cells: BTreeMap<Cell, String> = BTreeMap::new();
    let mut merges: Vec<Merge> = Vec::new();

    let mut cell: Option<(Cell, Option<String>, String)> = None;
    let mut collecting = false;

    loop {
        match reader.read_event().map_err(invalid)? {
            Event::Start(start) => match start.local_name().as_ref() {
                b"c" => {
                    let mut position = None;
                    let mut kind = None;
                    for attribute in start.attributes().flatten() {
                        let value = attribute.unescape_value().map_err(invalid)?;
                        match attribute.key.as_ref() {
                            b"r" => position = parse_cell(&value),
                            b"t" => kind = Some(value.to_string()),
                            _ => {}
                        }
                    }
                    cell = position.map(|at| (at, kind, String::new()));
                }
                b"v" | b"t" => collecting = cell.is_some(),
                _ => {}
            },
            Event::End(end) => match end.local_name().as_ref() {
                b"v" | b"t" => collecting = false,
                b"c" => {
                    if let Some((at, kind, value)) = cell.take() {
                        let text = match kind.as_deref() {
                            Some("s") => value
                                .trim()
                                .parse::<usize>()
                                .ok()
                                .and_then(|index| shared.get(index))
                                .cloned()
                                .unwrap_or_default(),
                            _ => value,
                        };
                        if !text.trim().is_empty() {
                            cells.insert(at, text);
                        }
                    }
                }
                _ => {}
            },
            Event::Empty(empty) => {
                if empty.local_name().as_ref() == b"mergeCell" {
                    for attribute in empty.attributes().flatten() {
                        if attribute.key.as_ref() != b"ref" {
                            continue;
                        }
                        let value = attribute.unescape_value().map_err(invalid)?;
                        if let Some(merge) = parse_merge(&value) {
                            merges.push(merge);
                        }
                    }
                }
            }
            Event::Text(text) => {
                if collecting {
                    let value = text.decode().map_err(invalid)?;
                    if let Some((_, _, buffer)) = cell.as_mut() {
                        buffer.push_str(&value);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(Sheet { cells, merges })
}

fn invalid(error: impl std::fmt::Display) -> DomainError {
    DomainError::Validation(format!("Unlesbares Blatt: {error}"))
}

/// `AB12` → row 12, column 28.
fn parse_cell(text: &str) -> Option<Cell> {
    let mut column = 0u32;
    let mut row = String::new();
    for character in text.chars() {
        if character.is_ascii_alphabetic() {
            if !row.is_empty() {
                return None;
            }
            column = column * 26 + (character.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
        } else if character.is_ascii_digit() {
            row.push(character);
        } else {
            return None;
        }
    }
    Some((row.parse().ok()?, column))
}

/// `A3:D8` → the range.
fn parse_merge(text: &str) -> Option<Merge> {
    let (from, to) = text.split_once(':')?;
    let (row_from, col_from) = parse_cell(from)?;
    let (row_to, col_to) = parse_cell(to)?;
    Some(Merge {
        row_from,
        col_from,
        row_to,
        col_to,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    /// A copy of the PeM export the user works with.
    const BEISPIEL: &[u8] = include_bytes!("../tests/fixtures/peM-dispo.xlsx");

    /// The flag codes of the legend, as the master data would carry them.
    fn codes() -> Vec<String> {
        crate::flags::PRESETS
            .iter()
            .map(|preset| preset.code.to_string())
            .collect()
    }

    fn scan_december() -> DispoScan {
        scan(BEISPIEL, 12, 2026, false, &codes()).unwrap()
    }

    fn day(month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, month, day).unwrap()
    }

    #[test]
    fn the_fingerprint_follows_the_content_of_the_sheet() {
        let scan = scan_december();
        assert_eq!(fingerprint(&scan), fingerprint(&scan.clone()));

        let mut changed = scan.clone();
        changed.locations[0].events[0].starts_at = Some("23:59".to_string());
        assert_ne!(fingerprint(&scan), fingerprint(&changed));

        // Where the block sat on the sheet and what was skipped while reading it
        // say nothing about the planning itself.
        let mut moved = scan.clone();
        moved.locations[0].first_row += 3;
        moved.locations[0].last_row += 3;
        moved.warnings.push("Skipped a row.".to_string());
        moved.days += 1;
        assert_eq!(fingerprint(&scan), fingerprint(&moved));
    }

    #[test]
    fn every_location_is_one_block() {
        let scan = scan_december();
        let names: Vec<&str> = scan.locations.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Großes Haus", "Podium", "Podium.bar", "Oberes Foyer"]
        );
    }

    /// A location as the parser sees it, without events.
    fn ort(name: &str) -> DispoLocation {
        DispoLocation {
            name: name.to_string(),
            first_row: 3,
            last_row: 8,
            events: Vec::new(),
        }
    }

    #[test]
    fn the_tail_peem_repeats_is_cut_at_a_comma() {
        let mut orte = vec![
            ort("Großes Haus, Theater Ulm, Ulm"),
            ort("Podium, Theater Ulm, Ulm"),
            ort("Oberes Foyer, Theater Ulm, Ulm"),
        ];
        trim_shared_suffix(&mut orte);
        let names: Vec<&str> = orte.iter().map(|ort| ort.name.as_str()).collect();
        assert_eq!(names, vec!["Großes Haus", "Podium", "Oberes Foyer"]);
    }

    #[test]
    fn a_single_location_keeps_its_whole_name() {
        let mut orte = vec![ort("Großes Haus, Theater Ulm, Ulm")];
        trim_shared_suffix(&mut orte);
        assert_eq!(orte[0].name, "Großes Haus, Theater Ulm, Ulm");
    }

    #[test]
    fn names_without_a_shared_comma_keep_their_name() {
        let mut orte = vec![ort("Großes Haus"), ort("Podium.bar")];
        trim_shared_suffix(&mut orte);
        let names: Vec<&str> = orte.iter().map(|ort| ort.name.as_str()).collect();
        assert_eq!(names, vec!["Großes Haus", "Podium.bar"]);
    }

    #[test]
    fn two_identical_names_are_left_alone() {
        let mut orte = vec![ort("Foyer, Haus A"), ort("Foyer, Haus A")];
        trim_shared_suffix(&mut orte);
        let names: Vec<&str> = orte.iter().map(|ort| ort.name.as_str()).collect();
        assert_eq!(names, vec!["Foyer, Haus A", "Foyer, Haus A"]);
    }

    #[test]
    fn the_block_height_comes_from_the_merge() {
        let scan = scan_december();
        let heights: Vec<(u32, u32, u32)> = scan
            .locations
            .iter()
            .map(|l| (l.first_row, l.last_row, l.rows()))
            .collect();
        assert_eq!(
            heights,
            vec![(3, 8, 6), (9, 13, 5), (14, 14, 1), (15, 15, 1)]
        );
    }

    #[test]
    fn a_sheet_that_stands_in_the_month_already_adds_nothing() {
        let bytes = "Standort;Datum;Beginn;Titel;Kennzeichen\r\n\
            Großes Haus;01.12.2026;19:30;Teststück;Vst\r\n\
            Podium;02.12.2026;;Einweisung;\r\n"
            .as_bytes();
        let from = NaiveDate::from_ymd_opt(2026, 12, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let scan = crate::exchange::read_dispo(bytes, from, to, false).unwrap();
        let vorhanden = vec![
            DispoKey {
                location: "großes haus".to_string(),
                day: NaiveDate::from_ymd_opt(2026, 12, 1).unwrap(),
                starts_at: Some("19:30".to_string()),
                title: "Teststück".to_string(),
                flag: Some("Vst".to_string()),
            },
            DispoKey {
                location: "Podium".to_string(),
                day: NaiveDate::from_ymd_opt(2026, 12, 2).unwrap(),
                starts_at: None,
                title: "Einweisung".to_string(),
                flag: None,
            },
        ];
        assert!(adds_nothing(&scan, &vorhanden));
        // One missing event is enough for the file to add something.
        assert!(!adds_nothing(&scan, &vorhanden[..1]));
    }

    #[test]
    fn the_staff_plan_below_is_not_read() {
        let scan = scan_december();
        assert_eq!(scan.locations.len(), 4);
        assert!(
            scan.locations
                .iter()
                .all(|l| !l.name.starts_with("Apolloni") && l.last_row < 18)
        );
    }

    #[test]
    fn the_days_come_from_the_date_header() {
        let scan = scan_december();
        assert_eq!(scan.days, 31);
        assert!(scan.warnings.is_empty(), "{:?}", scan.warnings);
    }

    #[test]
    fn a_row_without_an_event_stays_in_its_block() {
        let scan = scan_december();
        let grosse = &scan.locations[0];
        assert_eq!(grosse.rows(), 6);
        assert_eq!(grosse.last_row, 8);
    }

    #[test]
    fn every_cell_becomes_an_entry_of_its_own() {
        let scan = scan_december();
        let grosse = &scan.locations[0];
        let first: Vec<(&str, &str)> = grosse
            .events
            .iter()
            .filter(|event| event.day == day(12, 1))
            .map(|event| {
                (
                    event.starts_at.as_deref().unwrap_or(""),
                    event.title.as_str(),
                )
            })
            .collect();
        assert_eq!(
            first,
            vec![
                ("09:00", "Max und Moritz Vst"),
                ("11:00", "Max und Moritz Vst"),
                ("17:00", "Così fan tutte Probe"),
            ]
        );
        let podium = &scan.locations[1];
        let first: Vec<(&str, &str)> = podium
            .events
            .iter()
            .filter(|event| event.day == day(12, 1))
            .map(|event| {
                (
                    event.starts_at.as_deref().unwrap_or(""),
                    event.title.as_str(),
                )
            })
            .collect();
        assert_eq!(
            first,
            vec![
                ("11:25", "Together in Motion 26/27 Probe"),
                ("18:00", "Ehebrecherinnen Probe"),
            ]
        );
        assert!(
            scan.locations
                .iter()
                .flat_map(|location| location.events.iter())
                .all(|event| event.day.month() == 12 && event.day.year() == 2026)
        );
    }

    #[test]
    fn the_flag_of_an_event_is_recognised() {
        let scan = scan_december();
        let first = &scan.locations[0].events[0];
        assert_eq!(first.title, "Max und Moritz Vst");
        assert_eq!(first.flag.as_deref(), Some("Vst"));
        let mut counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for event in scan
            .locations
            .iter()
            .flat_map(|location| location.events.iter())
        {
            if let Some(flag) = &event.flag {
                *counts.entry(flag.clone()).or_default() += 1;
            }
        }
        assert!(counts.get("Vst").copied().unwrap_or(0) > 20, "{counts:?}");
        assert!(counts.contains_key("Probe"), "{counts:?}");
        assert!(counts.contains_key("BO"), "{counts:?}");
        assert!(counts.contains_key("Belkorr."), "{counts:?}");
        assert!(counts.contains_key("HP1"), "{counts:?}");
    }

    #[test]
    fn the_import_knows_which_flags_to_bring_in() {
        let scan = scan_december();
        let used = used_flags(&scan);
        assert!(used.contains(&"Vst".to_string()), "{used:?}");
        assert!(used.contains(&"Belkorr.".to_string()), "{used:?}");
        // Each code once, whatever the number of events.
        let mut sorted = used.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), used.len());
        assert!(used.len() <= crate::flags::PRESETS.len(), "{used:?}");
    }

    #[test]
    fn a_cell_without_a_time_keeps_its_text() {
        let scan = scan_december();
        let ohne: Vec<&DispoEvent> = scan
            .locations
            .iter()
            .flat_map(|location| location.events.iter())
            .filter(|event| event.starts_at.is_none())
            .collect();
        assert!(!ohne.is_empty(), "der Beispielplan hat Zellen ohne Uhrzeit");
        assert!(ohne.iter().any(|event| event.title.starts_with("i.A.")));
    }

    #[test]
    fn a_time_is_recognised_at_the_start() {
        assert!(has_time("09:00 Max und Moritz Vst"));
        assert!(has_time("9.00 Probe"));
        assert!(has_time(" 20:00 Kipppunkte Vst"));
        assert!(!has_time("i.A. an die Vorstellung Spaceman"));
        assert!(!has_time("Achtung"));
        assert!(!has_time("25:00 zu spät"));
    }

    #[test]
    fn the_time_is_split_off_the_text() {
        assert_eq!(
            split_time("09:00 Max und Moritz Vst"),
            (Some("09:00".to_string()), "Max und Moritz Vst".to_string())
        );
        assert_eq!(
            split_time("9.00 Probe"),
            (Some("09:00".to_string()), "Probe".to_string())
        );
        assert_eq!(
            split_time("i.A. an die Vorstellung"),
            (None, "i.A. an die Vorstellung".to_string())
        );
        // A cell that is only a time keeps it as its text.
        assert_eq!(split_time("18:00"), (None, "18:00".to_string()));
    }

    #[test]
    fn a_text_without_a_time_can_be_dropped() {
        let alle = scan_december();
        let mit_zeit = scan(BEISPIEL, 12, 2026, true, &codes()).unwrap();
        let events = |scan: &DispoScan| {
            scan.locations
                .iter()
                .flat_map(|location| location.events.iter())
                .count()
        };
        let ohne = alle
            .locations
            .iter()
            .flat_map(|location| location.events.iter())
            .filter(|event| event.starts_at.is_none())
            .count();
        assert!(ohne > 0);
        assert_eq!(events(&alle) - events(&mit_zeit), ohne);
        assert!(
            mit_zeit
                .warnings
                .iter()
                .any(|warning| warning.contains("ohne Uhrzeit"))
        );
    }

    #[test]
    fn a_day_the_month_does_not_have_is_reported() {
        let scan = scan(BEISPIEL, 11, 2026, false, &codes()).unwrap();
        assert_eq!(scan.days, 30);
        assert!(scan.warnings.iter().any(|warning| warning.contains("31.")));
        assert!(
            scan.locations
                .iter()
                .flat_map(|location| location.events.iter())
                .all(|event| event.day.month() == 11)
        );
    }

    #[test]
    fn something_that_is_not_a_workbook_is_refused() {
        assert!(scan(b"not a zip", 12, 2026, false, &codes()).is_err());
    }

    #[test]
    fn a_month_outside_the_year_is_refused() {
        assert!(scan(BEISPIEL, 13, 2026, false, &codes()).is_err());
        assert!(scan(BEISPIEL, 0, 2026, false, &codes()).is_err());
    }

    #[test]
    fn a_cell_text_stops_at_the_column_width() {
        assert_eq!(cap("short", 10), "short");
        let lang = cap(&"x".repeat(30), 10);
        assert_eq!(lang.chars().count(), 10);
        assert!(lang.ends_with('…'));
    }

    #[test]
    fn a_header_day_is_read_in_every_usual_form() {
        assert_eq!(day_of_month("01.12"), Some(1));
        assert_eq!(day_of_month("1.12"), Some(1));
        assert_eq!(day_of_month(" 24.12. "), Some(24));
        assert_eq!(day_of_month("Di"), None);
        assert_eq!(day_of_month("1. Weihnachtstag"), None);
        assert_eq!(day_of_month("00.12"), None);
        assert_eq!(day_of_month("01.13"), None);
    }

    #[test]
    fn a_cell_reference_is_read_column_and_row() {
        assert_eq!(parse_cell("A3"), Some((3, 1)));
        assert_eq!(parse_cell("E1"), Some((1, 5)));
        assert_eq!(parse_cell("BN60"), Some((60, 66)));
        assert_eq!(parse_cell("3A"), None);
    }

    #[test]
    fn an_entry_is_checked_before_it_is_written() {
        let input = DispoEntryInput {
            id: None,
            starts_at: " 20.00 ".to_string(),
            title: "  Probe  ".to_string(),
            flag: None,
            ignored: false,
        };
        let clean = input.clean().unwrap();
        assert_eq!(clean.starts_at.as_deref(), Some("20:00"));
        assert_eq!(clean.title, "Probe");
        assert_eq!(clean.flag, None);
        let ohne = DispoEntryInput {
            id: Some(7),
            starts_at: String::new(),
            title: "Sitzprobe".to_string(),
            flag: Some("  ".to_string()),
            ignored: false,
        };
        let sauber = ohne.clean().unwrap();
        assert_eq!(sauber.starts_at, None);
        assert_eq!(sauber.flag, None);
        let mit_flag = DispoEntryInput {
            id: None,
            starts_at: String::new(),
            title: "Max und Moritz Vst".to_string(),
            flag: Some(" Vst ".to_string()),
            ignored: false,
        };
        assert_eq!(mit_flag.clean().unwrap().flag.as_deref(), Some("Vst"));
        let leer = DispoEntryInput {
            id: None,
            starts_at: String::new(),
            title: "   ".to_string(),
            flag: None,
            ignored: false,
        };
        assert!(leer.clean().is_err());
        let kaputt = DispoEntryInput {
            id: None,
            starts_at: "25:00".to_string(),
            title: "Probe".to_string(),
            flag: None,
            ignored: false,
        };
        assert!(kaputt.clean().is_err());
    }

    #[test]
    fn a_day_takes_a_limited_number_of_entries() {
        let entry = || DispoEntryInput {
            id: None,
            starts_at: String::new(),
            title: "Probe".to_string(),
            flag: None,
            ignored: false,
        };
        let voll = DispoDayInput {
            entries: (0..MAX_ENTRIES).map(|_| entry()).collect(),
        };
        assert_eq!(voll.clean().unwrap().len(), MAX_ENTRIES);
        let too_many = DispoDayInput {
            entries: (0..=MAX_ENTRIES).map(|_| entry()).collect(),
        };
        assert!(too_many.clean().is_err());
    }

    #[test]
    fn the_month_range_covers_the_last_day() {
        let (from, to) = DispoQuery {
            month: 12,
            year: 2026,
            replace: false,
            only_with_time: false,
        }
        .month_range()
        .unwrap();
        assert_eq!(from, day(12, 1));
        assert_eq!(to, day(12, 31));
        let (_, februar) = DispoQuery {
            month: 2,
            year: 2028,
            replace: false,
            only_with_time: false,
        }
        .month_range()
        .unwrap();
        assert_eq!(februar, NaiveDate::from_ymd_opt(2028, 2, 29).unwrap());
    }
}
