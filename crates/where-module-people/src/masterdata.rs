//! The master data of the module as exchange files.
//!
//! Unlike the exchange of a month ([`crate::exchange`]) these files carry records, not
//! days: they are written on one installation and read on the next without a hand on
//! them. Three areas exist, one per page of the master data:
//!
//! * `lists` — `Liste; Name; Abteilung; Vorarbeiter`, one row per entry of the
//!   departments, the positions and the locations;
//! * `employees` — one row per person with the columns of the mask;
//! * `shifts` — `Kürzel; Name; Beginn; Ende; Pause; Band; Nur Azubis; Farbe; Aktiv;
//!   Ausgeschlossene Positionen`, the shifts of the schedule planning.
//!
//! The pages write CSV (`;`, UTF-8 with BOM) — see
//! `modules/where-are-my-people/web/stammdaten-tausch.ts` — and the reader takes CSV
//! and XLSX like the exchange of the plan does. An import never deletes: it creates
//! what the file carries and, with `replace`, overwrites the records it finds again.

use std::str::FromStr;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use utoipa::ToSchema;
use where_domain::DomainError;

use crate::employee::{Employee, EmployeeInput};
use crate::employee_repo::EmployeeRepository;
use crate::exchange::{cell, head_of, is_yes, read_table};
use crate::list_repo::PeopleListRepository;
use crate::lists::{ListEntry, ListEntryInput, ListKind, needs_foreman_default};
use crate::shift_repo::ShiftRepository;
use crate::shifts::{Shift, ShiftBand, ShiftInput};

/// Which master data a file carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterArea {
    /// Departments, positions and locations.
    Lists,
    /// The employee master.
    Employees,
    /// The shifts of the schedule planning.
    Shifts,
}

impl MasterArea {
    /// From the path `/people/masterdata/{area}`.
    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_lowercase().as_str() {
            "lists" => Some(Self::Lists),
            "employees" => Some(Self::Employees),
            "shifts" => Some(Self::Shifts),
            _ => None,
        }
    }

    /// The key of the path and of the file name.
    pub fn key(self) -> &'static str {
        match self {
            Self::Lists => "lists",
            Self::Employees => "employees",
            Self::Shifts => "shifts",
        }
    }
}

/// Head of the list sheet: which list, then the name.
pub const LIST_HEAD: [&str; 2] = ["Liste", "Name"];
const LIST_DEPARTMENT: &str = "Abteilung";
const LIST_FOREMAN: &str = "Vorarbeiter";

/// Head of the employee sheet — the two columns without which a row is no person.
pub const EMPLOYEE_HEAD: [&str; 2] = ["Vorname", "Nachname"];
const EMPLOYEE_NUMBER: &str = "Personalnummer";
const EMPLOYEE_SALUTATION: &str = "Anrede";
const EMPLOYEE_TITLE: &str = "Titel";
const EMPLOYEE_FIRST: &str = "Vorname";
const EMPLOYEE_LAST: &str = "Nachname";
const EMPLOYEE_BIRTH: &str = "Geburtsdatum";
const EMPLOYEE_BIRTH_PLACE: &str = "Geburtsort";
const EMPLOYEE_NATIONALITY: &str = "Nationalität";
const EMPLOYEE_MARITAL: &str = "Familienstand";
const EMPLOYEE_STREET: &str = "Straße";
const EMPLOYEE_POSTAL: &str = "PLZ";
const EMPLOYEE_CITY: &str = "Ort";
const EMPLOYEE_COUNTRY: &str = "Land";
const EMPLOYEE_EMAIL: &str = "E-Mail";
const EMPLOYEE_PHONE: &str = "Telefon";
const EMPLOYEE_MOBILE: &str = "Mobil";
const EMPLOYEE_ENTRY: &str = "Eintritt";
const EMPLOYEE_EXIT: &str = "Austritt";
const EMPLOYEE_EMPLOYMENT: &str = "Beschäftigung";
const EMPLOYEE_HOURS: &str = "Wochenstunden";
const EMPLOYEE_WORK_DAYS: &str = "Arbeitstage";
const EMPLOYEE_VACATION: &str = "Urlaubstage";
const EMPLOYEE_POSITION: &str = "Position";
const EMPLOYEE_DEPARTMENT: &str = "Abteilung";
const EMPLOYEE_SUBDEPARTMENT: &str = "Unterabteilung";
const EMPLOYEE_LOCATION: &str = "Standort";
const EMPLOYEE_COST_CENTER: &str = "Kostenstelle";
const EMPLOYEE_MANAGER: &str = "Vorgesetzter";
const EMPLOYEE_SOCIAL: &str = "Sozialversicherungsnummer";
const EMPLOYEE_TAX: &str = "Steuer-ID";
const EMPLOYEE_NOTES: &str = "Notiz";
const EMPLOYEE_ACTIVE: &str = "Aktiv";
const EMPLOYEE_APPRENTICE: &str = "Azubi";
const EMPLOYEE_AUTO_PLAN: &str = "Automatisch disponierbar";
const EMPLOYEE_QUALIFICATIONS: &str = "Qualifikationen";

/// Head of the shift sheet — name and times, as the mask asks for them.
pub const SHIFT_HEAD: [&str; 3] = ["Name", "Beginn", "Ende"];
const SHIFT_SHORT: &str = "Kürzel";
const SHIFT_BREAK: &str = "Pause";
const SHIFT_BAND: &str = "Band";
const SHIFT_APPRENTICE: &str = "Nur Azubis";
const SHIFT_COLOR: &str = "Farbe";
const SHIFT_DEPARTMENT: &str = "Abteilung";
const SHIFT_ACTIVE: &str = "Aktiv";
const SHIFT_EXCLUSIONS: &str = "Ausgeschlossene Positionen";

/// What an import did — or, as a scan, would do.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MasterReport {
    /// Rows the file carried, without its head.
    pub rows: usize,
    /// Records that did not exist before.
    pub created: usize,
    /// Records the file changes; without `replace` they stay as they are.
    pub changed: usize,
    /// Records the file agrees with.
    pub unchanged: usize,
    /// Everything that was skipped, with the line of the file.
    pub warnings: Vec<String>,
}

/// Reads a master-data file and writes it — or only says what it would do.
///
/// `write` is `false` for the preview of the window: the file is read and compared
/// with the records in the database, nothing is changed.
pub async fn import(
    pool: &PgPool,
    area: MasterArea,
    bytes: &[u8],
    replace: bool,
    write: bool,
) -> Result<MasterReport, DomainError> {
    match area {
        MasterArea::Lists => {
            let rows = read_lists(bytes)?;
            apply_lists(pool, rows, replace, write).await
        }
        MasterArea::Employees => {
            let rows = read_employees(bytes)?;
            apply_employees(pool, rows, replace, write).await
        }
        MasterArea::Shifts => {
            let rows = read_shifts(bytes)?;
            apply_shifts(pool, rows, replace, write).await
        }
    }
}

/// The columns of a sheet by the name of their head; a sheet may leave any of them out.
struct Columns {
    names: Vec<String>,
}

impl Columns {
    fn of(rows: &[Vec<String>], start: usize) -> Self {
        let head = start
            .checked_sub(1)
            .and_then(|index| rows.get(index))
            .cloned()
            .unwrap_or_default();
        Self {
            names: head.iter().map(|text| text.trim().to_lowercase()).collect(),
        }
    }

    fn position(&self, name: &str) -> Option<usize> {
        let wanted = name.trim().to_lowercase();
        self.names.iter().position(|known| *known == wanted)
    }

    fn has(&self, name: &str) -> bool {
        self.position(name).is_some()
    }

    /// The trimmed text of one column, empty when the sheet does not carry it.
    fn text(&self, row: &[String], name: &str) -> String {
        self.position(name)
            .map(|index| cell(row, index).trim().to_string())
            .unwrap_or_default()
    }

    /// The column as `Option`, empty cells included.
    fn opt(&self, row: &[String], name: &str) -> Option<String> {
        let text = self.text(row, name);
        if text.is_empty() { None } else { Some(text) }
    }

    /// A switch: empty stays at `default`, anything else is read as yes or no.
    fn flag(&self, row: &[String], name: &str, default: bool) -> bool {
        let text = self.text(row, name);
        if text.is_empty() {
            default
        } else {
            is_yes(&text)
        }
    }

    /// A day as `2026-11-16` or `16.11.2026`; anything else is named and dropped.
    fn day(
        &self,
        row: &[String],
        name: &str,
        line: usize,
        warnings: &mut Vec<String>,
    ) -> Option<NaiveDate> {
        let text = self.text(row, name);
        if text.is_empty() {
            return None;
        }
        match parse_day(&text) {
            Some(day) => Some(day),
            None => {
                warnings.push(format!(
                    "Zeile {line}: „{text}“ in „{name}“ ist kein Datum und bleibt leer."
                ));
                None
            }
        }
    }

    /// A whole number, e.g. the work days of the week.
    fn small(
        &self,
        row: &[String],
        name: &str,
        line: usize,
        warnings: &mut Vec<String>,
    ) -> Option<i16> {
        let text = self.text(row, name);
        if text.is_empty() {
            return None;
        }
        match text.replace(',', ".").parse::<i16>() {
            Ok(value) => Some(value),
            Err(_) => {
                warnings.push(format!(
                    "Zeile {line}: „{text}“ in „{name}“ ist keine ganze Zahl und bleibt leer."
                ));
                None
            }
        }
    }

    /// Hours per week, with a comma or a full stop as the decimal mark.
    fn hours(&self, row: &[String], line: usize, warnings: &mut Vec<String>) -> Option<Decimal> {
        let text = self.text(row, EMPLOYEE_HOURS);
        if text.is_empty() {
            return None;
        }
        match Decimal::from_str(&text.replace(',', ".")) {
            Ok(value) => Some(value),
            Err(_) => {
                warnings.push(format!(
                    "Zeile {line}: „{text}“ in „{EMPLOYEE_HOURS}“ ist keine Stundenzahl und bleibt leer."
                ));
                None
            }
        }
    }
}

/// `2026-11-16` or `16.11.2026`.
fn parse_day(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(text, "%d.%m.%Y"))
        .ok()
}

/// Names that a sheet lists in one cell, separated by `,` or `;`.
fn split_names(text: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for part in text.split([',', ';']) {
        let name = part.trim();
        if name.is_empty() {
            continue;
        }
        if !names.iter().any(|known| known.eq_ignore_ascii_case(name)) {
            names.push(name.to_string());
        }
    }
    names
}

/// Which list a row of the list sheet means.
fn list_kind(text: &str) -> Option<ListKind> {
    match text.trim().to_lowercase().as_str() {
        "abteilungen" | "abteilung" | "departments" | "department" => Some(ListKind::Departments),
        "unterabteilungen" | "unterabteilung" | "subdepartments" | "subdepartment" => {
            Some(ListKind::Subdepartments)
        }
        "positionen" | "position" | "positions" => Some(ListKind::Positions),
        "standorte" | "standort" | "locations" | "location" => Some(ListKind::Locations),
        _ => None,
    }
}

/// One row of the list sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListRow {
    pub kind: ListKind,
    pub name: String,
    /// Only a location carries one.
    pub department: Option<String>,
    /// Only a location carries one; empty when the sheet says nothing.
    pub foreman: Option<bool>,
}

/// Reads the list sheet.
pub fn read_lists(bytes: &[u8]) -> Result<(Vec<ListRow>, Vec<String>), DomainError> {
    let rows = read_table(bytes)?;
    let Some((_, start)) = head_of(&rows, &LIST_HEAD) else {
        return Err(DomainError::Validation(format!(
            "Die Kopfzeile „{}“ fehlt.",
            LIST_HEAD.join("; ")
        )));
    };
    let columns = Columns::of(&rows, start);
    let mut read: Vec<ListRow> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(start) {
        let line = index + 1;
        let name = columns.text(row, "Name");
        if name.is_empty() {
            warnings.push(format!("Zeile {line}: ohne Namen übersprungen."));
            continue;
        }
        let Some(kind) = list_kind(&columns.text(row, "Liste")) else {
            warnings.push(format!(
                "Zeile {line}: „{}“ ist keine Liste (Abteilungen, Unterabteilungen, Positionen, Standorte).",
                columns.text(row, "Liste")
            ));
            continue;
        };
        let foreman = if columns.has(LIST_FOREMAN) {
            let text = columns.text(row, LIST_FOREMAN);
            if text.is_empty() {
                None
            } else {
                Some(is_yes(&text))
            }
        } else {
            None
        };
        read.push(ListRow {
            kind,
            name,
            department: columns.opt(row, LIST_DEPARTMENT),
            foreman,
        });
    }
    Ok((read, warnings))
}

/// One row of the employee sheet.
#[derive(Debug, Clone)]
pub struct EmployeeRow {
    /// Key of the file: the personnel number, otherwise last and first name.
    pub key: String,
    pub input: EmployeeInput,
}

/// The name a row is found again by when it carries no personnel number.
fn person_key(last: &str, first: &str) -> String {
    format!(
        "{}|{}",
        last.trim().to_lowercase(),
        first.trim().to_lowercase()
    )
}

/// Reads the employee sheet — one row per person, the columns of the mask.
pub fn read_employees(bytes: &[u8]) -> Result<(Vec<EmployeeRow>, Vec<String>), DomainError> {
    let rows = read_table(bytes)?;
    let Some((_, start)) = head_of(&rows, &EMPLOYEE_HEAD) else {
        return Err(DomainError::Validation(format!(
            "Die Kopfzeile „{}“ fehlt.",
            EMPLOYEE_HEAD.join("; ")
        )));
    };
    let columns = Columns::of(&rows, start);
    let mut read: Vec<EmployeeRow> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(start) {
        let line = index + 1;
        let first = columns.text(row, EMPLOYEE_FIRST);
        let last = columns.text(row, EMPLOYEE_LAST);
        if first.is_empty() || last.is_empty() {
            warnings.push(format!(
                "Zeile {line}: Vor- und Nachname fehlen, übersprungen."
            ));
            continue;
        }
        let number = columns.opt(row, EMPLOYEE_NUMBER);
        let key = match &number {
            Some(number) => number.to_lowercase(),
            None => person_key(&last, &first),
        };
        if read.iter().any(|known| known.key == key) {
            warnings.push(format!(
                "Zeile {line}: {first} {last} steht in der Datei mehrfach, übersprungen."
            ));
            continue;
        }
        let input = EmployeeInput {
            personnel_no: number,
            salutation: columns.opt(row, EMPLOYEE_SALUTATION),
            title: columns.opt(row, EMPLOYEE_TITLE),
            first_name: first,
            last_name: last,
            birth_date: columns.day(row, EMPLOYEE_BIRTH, line, &mut warnings),
            birth_place: columns.opt(row, EMPLOYEE_BIRTH_PLACE),
            nationality: columns.opt(row, EMPLOYEE_NATIONALITY),
            marital_status: columns.opt(row, EMPLOYEE_MARITAL),
            street: columns.opt(row, EMPLOYEE_STREET),
            postal_code: columns.opt(row, EMPLOYEE_POSTAL),
            city: columns.opt(row, EMPLOYEE_CITY),
            country: columns.opt(row, EMPLOYEE_COUNTRY),
            email: columns.opt(row, EMPLOYEE_EMAIL),
            phone: columns.opt(row, EMPLOYEE_PHONE),
            mobile: columns.opt(row, EMPLOYEE_MOBILE),
            entry_date: columns.day(row, EMPLOYEE_ENTRY, line, &mut warnings),
            exit_date: columns.day(row, EMPLOYEE_EXIT, line, &mut warnings),
            employment_type: columns.opt(row, EMPLOYEE_EMPLOYMENT),
            weekly_hours: columns.hours(row, line, &mut warnings),
            work_days: columns.small(row, EMPLOYEE_WORK_DAYS, line, &mut warnings),
            vacation_days: columns.small(row, EMPLOYEE_VACATION, line, &mut warnings),
            position: columns.opt(row, EMPLOYEE_POSITION),
            department: columns.opt(row, EMPLOYEE_DEPARTMENT),
            subdepartment: columns.opt(row, EMPLOYEE_SUBDEPARTMENT),
            location: columns.opt(row, EMPLOYEE_LOCATION),
            cost_center: columns.opt(row, EMPLOYEE_COST_CENTER),
            manager: columns.opt(row, EMPLOYEE_MANAGER),
            social_security_no: columns.opt(row, EMPLOYEE_SOCIAL),
            tax_id: columns.opt(row, EMPLOYEE_TAX),
            notes: columns.opt(row, EMPLOYEE_NOTES),
            active: columns.flag(row, EMPLOYEE_ACTIVE, true),
            is_apprentice: columns.flag(row, EMPLOYEE_APPRENTICE, false),
            auto_plan: columns.flag(row, EMPLOYEE_AUTO_PLAN, true),
            qualifications: split_names(&columns.text(row, EMPLOYEE_QUALIFICATIONS)),
        };
        if let Err(error) = input.validate() {
            warnings.push(format!(
                "Zeile {line}: {} {} — {error}",
                input.first_name, input.last_name
            ));
            continue;
        }
        read.push(EmployeeRow { key, input });
    }
    Ok((read, warnings))
}

/// One row of the shift sheet.
#[derive(Debug, Clone)]
pub struct ShiftRow {
    /// Key of the file: the short name, otherwise the name.
    pub key: String,
    pub input: ShiftInput,
    /// `None` when the sheet has no such column — then the exclusions stay untouched.
    pub exclusions: Option<Vec<String>>,
}

/// The band as a sheet names it: `früh`, `spät`, `zwischen` or `frei` — the sheet
/// of the theatre is German, the stored value is not.
fn shift_band(text: &str) -> Option<ShiftBand> {
    match text.trim().to_lowercase().as_str() {
        "" => Some(ShiftBand::Early),
        "früh" | "frueh" | "fruehschicht" | "early" => Some(ShiftBand::Early),
        "spät" | "spaet" | "spaetschicht" | "late" => Some(ShiftBand::Late),
        "zwischen" | "zwischenschicht" | "between" | "intermediate" => {
            Some(ShiftBand::Intermediate)
        }
        "frei" | "free" => Some(ShiftBand::Free),
        _ => None,
    }
}

/// Reads the shift sheet.
pub fn read_shifts(bytes: &[u8]) -> Result<(Vec<ShiftRow>, Vec<String>), DomainError> {
    let rows = read_table(bytes)?;
    let Some((_, start)) = head_of(&rows, &SHIFT_HEAD) else {
        return Err(DomainError::Validation(format!(
            "Die Kopfzeile „{}“ fehlt.",
            SHIFT_HEAD.join("; ")
        )));
    };
    let columns = Columns::of(&rows, start);
    let mut read: Vec<ShiftRow> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for (index, row) in rows.iter().enumerate().skip(start) {
        let line = index + 1;
        let name = columns.text(row, "Name");
        if name.is_empty() {
            warnings.push(format!("Zeile {line}: ohne Namen übersprungen."));
            continue;
        }
        let Some(band) = shift_band(&columns.text(row, SHIFT_BAND)) else {
            warnings.push(format!(
                "Zeile {line}: „{}“ ist kein Band (früh, spät, zwischen, frei).",
                columns.text(row, SHIFT_BAND)
            ));
            continue;
        };
        let short_name = columns.opt(row, SHIFT_SHORT);
        let key = match &short_name {
            Some(short) => short.to_lowercase(),
            None => name.to_lowercase(),
        };
        if read.iter().any(|known| known.key == key) {
            warnings.push(format!(
                "Zeile {line}: „{name}“ steht in der Datei mehrfach, übersprungen."
            ));
            continue;
        }
        let input = ShiftInput {
            name,
            short_name,
            starts_at: columns.text(row, "Beginn"),
            ends_at: columns.text(row, "Ende"),
            break_minutes: columns
                .small(row, SHIFT_BREAK, line, &mut warnings)
                .unwrap_or_default(),
            band: Some(band),
            apprentice_only: columns.flag(row, SHIFT_APPRENTICE, false),
            color: columns.opt(row, SHIFT_COLOR),
            department: columns.opt(row, SHIFT_DEPARTMENT),
            active: columns.flag(row, SHIFT_ACTIVE, true),
        };
        if let Err(error) = input.clean() {
            warnings.push(format!("Zeile {line}: „{}“ — {error}", input.name));
            continue;
        }
        let exclusions = if columns.has(SHIFT_EXCLUSIONS) {
            Some(split_names(&columns.text(row, SHIFT_EXCLUSIONS)))
        } else {
            None
        };
        read.push(ShiftRow {
            key,
            input,
            exclusions,
        });
    }
    Ok((read, warnings))
}

/// Writes (or plans) the rows of the list sheet.
async fn apply_lists(
    pool: &PgPool,
    rows: (Vec<ListRow>, Vec<String>),
    replace: bool,
    write: bool,
) -> Result<MasterReport, DomainError> {
    let (rows, warnings) = rows;
    let mut report = MasterReport {
        rows: rows.len(),
        warnings,
        ..MasterReport::default()
    };
    let departments = PeopleListRepository::new(pool.clone(), ListKind::Departments);
    let subdepartments = PeopleListRepository::new(pool.clone(), ListKind::Subdepartments);
    let positions = PeopleListRepository::new(pool.clone(), ListKind::Positions);
    let locations = PeopleListRepository::new(pool.clone(), ListKind::Locations);
    let mut known: Vec<(ListKind, ListEntry)> = Vec::new();
    for kind in [
        ListKind::Departments,
        ListKind::Subdepartments,
        ListKind::Positions,
        ListKind::Locations,
    ] {
        for entry in repository_for(&departments, &subdepartments, &positions, &locations, kind)
            .list()
            .await?
        {
            known.push((kind, entry));
        }
    }
    for row in rows {
        let found = known.iter().position(|(kind, entry)| {
            *kind == row.kind && entry.name.eq_ignore_ascii_case(&row.name)
        });
        // A sub-department and a location name a parent: the department they sit under.
        let carries_parent =
            row.kind == ListKind::Locations || row.kind == ListKind::Subdepartments;
        match found.map(|index| known[index].1.clone()) {
            Some(entry) => {
                let foreman = row.foreman.unwrap_or(needs_foreman_default());
                let differs = (carries_parent && entry.department != row.department)
                    || (row.kind == ListKind::Locations
                        && row.foreman.is_some()
                        && entry.needs_foreman != foreman);
                if !replace || !differs {
                    report.unchanged += 1;
                    continue;
                }
                if write {
                    let repository = repository_for(
                        &departments,
                        &subdepartments,
                        &positions,
                        &locations,
                        row.kind,
                    );
                    if carries_parent {
                        repository
                            .set_department(entry.id, row.department.as_deref())
                            .await?;
                    }
                    if row.kind == ListKind::Locations {
                        repository.set_needs_foreman(entry.id, foreman).await?;
                    }
                }
                report.changed += 1;
            }
            None => {
                if write {
                    let repository = repository_for(
                        &departments,
                        &subdepartments,
                        &positions,
                        &locations,
                        row.kind,
                    );
                    let created = repository
                        .create(&ListEntryInput {
                            name: row.name.clone(),
                        })
                        .await?;
                    if carries_parent {
                        if let Some(department) = &row.department {
                            repository
                                .set_department(created.id, Some(department))
                                .await?;
                        }
                    }
                    if row.kind == ListKind::Locations {
                        if let Some(foreman) = row.foreman
                            && foreman != needs_foreman_default()
                        {
                            repository.set_needs_foreman(created.id, foreman).await?;
                        }
                    }
                }
                report.created += 1;
            }
        }
    }
    Ok(report)
}

fn repository_for<'a>(
    departments: &'a PeopleListRepository,
    subdepartments: &'a PeopleListRepository,
    positions: &'a PeopleListRepository,
    locations: &'a PeopleListRepository,
    kind: ListKind,
) -> &'a PeopleListRepository {
    match kind {
        ListKind::Departments => departments,
        ListKind::Subdepartments => subdepartments,
        ListKind::Positions => positions,
        ListKind::Locations => locations,
    }
}

/// Writes (or plans) the rows of the employee sheet.
async fn apply_employees(
    pool: &PgPool,
    rows: (Vec<EmployeeRow>, Vec<String>),
    replace: bool,
    write: bool,
) -> Result<MasterReport, DomainError> {
    let (rows, warnings) = rows;
    let mut report = MasterReport {
        rows: rows.len(),
        warnings,
        ..MasterReport::default()
    };
    let repository = EmployeeRepository::new(pool.clone());
    let known = repository.list(None).await?;
    for row in rows {
        let found = known.iter().find(|employee| {
            employee.id.is_some()
                && match (&employee.personnel_no, &row.input.personnel_no) {
                    (Some(number), Some(wanted)) => number.trim().eq_ignore_ascii_case(wanted),
                    (None, None) => {
                        person_key(&employee.last_name, &employee.first_name)
                            == person_key(&row.input.last_name, &row.input.first_name)
                    }
                    _ => false,
                }
        });
        match found {
            Some(employee) => {
                if !replace {
                    report.unchanged += 1;
                    continue;
                }
                let id = employee.id.unwrap_or_default();
                // The list of the master data carries no qualifications, so the record
                // has to be read for the comparison.
                let current = repository.get(id).await?;
                if same_employee(&current, &row.input) {
                    report.unchanged += 1;
                    continue;
                }
                if write {
                    repository.update(id, &row.input).await?;
                }
                report.changed += 1;
            }
            None => {
                if write {
                    repository.create(&row.input).await?;
                }
                report.created += 1;
            }
        }
    }
    Ok(report)
}

/// Whether the record already is what the row says.
fn same_employee(employee: &Employee, input: &EmployeeInput) -> bool {
    employee_columns(
        &employee.personnel_no,
        &employee.salutation,
        &employee.title,
        &employee.first_name,
        &employee.last_name,
    ) == employee_columns(
        &input.personnel_no,
        &input.salutation,
        &input.title,
        &input.first_name,
        &input.last_name,
    ) && employee.birth_date == input.birth_date
        && text(&employee.birth_place) == text(&input.birth_place)
        && text(&employee.nationality) == text(&input.nationality)
        && text(&employee.marital_status) == text(&input.marital_status)
        && text(&employee.street) == text(&input.street)
        && text(&employee.postal_code) == text(&input.postal_code)
        && text(&employee.city) == text(&input.city)
        && text(&employee.country) == text(&input.country)
        && text(&employee.email) == text(&input.email)
        && text(&employee.phone) == text(&input.phone)
        && text(&employee.mobile) == text(&input.mobile)
        && employee.entry_date == input.entry_date
        && employee.exit_date == input.exit_date
        && text(&employee.employment_type) == text(&input.employment_type)
        && employee.weekly_hours == input.weekly_hours
        && employee.work_days == input.work_days
        && employee.vacation_days == input.vacation_days
        && text(&employee.position) == text(&input.position)
        && text(&employee.department) == text(&input.department)
        && text(&employee.subdepartment) == text(&input.subdepartment)
        && text(&employee.location) == text(&input.location)
        && text(&employee.cost_center) == text(&input.cost_center)
        && text(&employee.manager) == text(&input.manager)
        && text(&employee.social_security_no) == text(&input.social_security_no)
        && text(&employee.tax_id) == text(&input.tax_id)
        && text(&employee.notes) == text(&input.notes)
        && employee.active == input.active
        && employee.is_apprentice == input.is_apprentice
        && employee.auto_plan == input.auto_plan
        && split_names(&employee.qualifications.join(","))
            == split_names(&input.qualifications.join(","))
}

/// Two texts as the comparison reads them: trimmed, empty as empty.
fn text(value: &Option<String>) -> String {
    value
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

/// The identifying texts of an employee, trimmed.
fn employee_columns(
    number: &Option<String>,
    salutation: &Option<String>,
    title: &Option<String>,
    first: &str,
    last: &str,
) -> Vec<String> {
    vec![
        text(number),
        text(salutation),
        text(title),
        first.trim().to_string(),
        last.trim().to_string(),
    ]
}

/// Writes (or plans) the rows of the shift sheet.
async fn apply_shifts(
    pool: &PgPool,
    rows: (Vec<ShiftRow>, Vec<String>),
    replace: bool,
    write: bool,
) -> Result<MasterReport, DomainError> {
    let (rows, warnings) = rows;
    let mut report = MasterReport {
        rows: rows.len(),
        warnings,
        ..MasterReport::default()
    };
    let repository = ShiftRepository::new(pool.clone());
    let known = repository.list().await?;
    for row in rows {
        let found = known
            .iter()
            .find(|shift| match (&shift.short_name, &row.input.short_name) {
                (Some(short), Some(wanted)) => short.trim().eq_ignore_ascii_case(wanted),
                _ => shift.name.trim().eq_ignore_ascii_case(&row.input.name),
            });
        match found {
            Some(shift) => {
                let exclusions = match &row.exclusions {
                    Some(_) => row.exclusions.clone().unwrap_or_default(),
                    None => repository.excluded_positions(shift.id).await?,
                };
                if !replace || same_shift(shift, &row.input, &exclusions) {
                    report.unchanged += 1;
                    continue;
                }
                if write {
                    let clean = row.input.clean()?;
                    repository.update(shift.id, &clean).await?;
                    if row.exclusions.is_some() {
                        repository.set_exclusions(shift.id, &exclusions).await?;
                    }
                }
                report.changed += 1;
            }
            None => {
                if write {
                    let clean = row.input.clean()?;
                    let created = repository.create(&clean).await?;
                    if let Some(exclusions) = &row.exclusions
                        && !exclusions.is_empty()
                    {
                        repository.set_exclusions(created.id, exclusions).await?;
                    }
                }
                report.created += 1;
            }
        }
    }
    Ok(report)
}

/// Whether the shift already is what the row says.
fn same_shift(shift: &Shift, input: &ShiftInput, exclusions: &[String]) -> bool {
    let Ok(clean) = input.clean() else {
        return false;
    };
    let color = |value: &Option<String>| {
        value
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_ascii_uppercase)
    };
    clean.name == shift.name.trim()
        && clean.short_name
            == shift
                .short_name
                .as_deref()
                .map(str::trim)
                .map(str::to_string)
        && clean.starts_at == shift.starts_at
        && clean.ends_at == shift.ends_at
        && clean.break_minutes == shift.break_minutes
        && clean.band == shift.band
        && clean.apprentice_only == shift.apprentice_only
        && color(&clean.color) == color(&shift.color)
        && clean.active == shift.active
        && {
            let mut left: Vec<String> = exclusions
                .iter()
                .map(|name| name.trim().to_lowercase())
                .collect();
            let mut right: Vec<String> = shift
                .excluded_positions
                .iter()
                .map(|name| name.trim().to_lowercase())
                .collect();
            left.sort();
            right.sort();
            left == right
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(lines: &[&str]) -> Vec<u8> {
        lines.join("\r\n").into_bytes()
    }

    #[test]
    fn the_list_sheet_names_all_three_lists() {
        let (rows, warnings) = read_lists(&sheet(&[
            "Liste; Name; Abteilung; Vorarbeiter",
            "Abteilungen; Beleuchtung;;",
            "Positionen; Tontechnik;;",
            "Standorte; Großes Haus; Technik; ja",
            "Standorte; Podium; Technik; nein",
            "; Ohne Liste;;",
        ]))
        .unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].kind, ListKind::Departments);
        assert_eq!(rows[1].kind, ListKind::Positions);
        assert_eq!(rows[2].department.as_deref(), Some("Technik"));
        assert_eq!(rows[2].foreman, Some(true));
        assert_eq!(rows[3].foreman, Some(false));
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("steht keine Liste") || warnings[0].contains("keine Liste"));
    }

    #[test]
    fn the_employee_sheet_reads_the_columns_of_the_mask() {
        let (rows, warnings) = read_employees(&sheet(&[
            "Personalnummer; Vorname; Nachname; Geburtsdatum; Wochenstunden; Arbeitstage; Aktiv; Azubi; Qualifikationen",
            "1001; Ada; Lovelace; 10.12.1985; 39,5; 5; ja; nein; Ton, Licht; Ton",
            "; Grace; Hopper; 1906-12-09; 20; 3; nein; ja;",
            "1003; Ohne; Nummer3; kein Datum; viel; 4; ;;",
        ]))
        .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].key, "1001");
        assert_eq!(
            rows[0].input.birth_date,
            NaiveDate::from_ymd_opt(1985, 12, 10)
        );
        assert_eq!(rows[0].input.weekly_hours, Decimal::from_str("39.5").ok());
        assert_eq!(rows[0].input.work_days, Some(5));
        assert!(rows[0].input.active);
        assert!(!rows[0].input.is_apprentice);
        assert_eq!(rows[0].input.qualifications, vec!["Ton", "Licht"]);
        // Without a personnel number the name is the key.
        assert_eq!(rows[1].key, "hopper|grace");
        assert!(!rows[1].input.active);
        assert!(rows[1].input.is_apprentice);
        // The third row names a date and a number that cannot be read; both stay empty.
        assert!(warnings.iter().any(|text| text.contains("kein Datum")));
        assert!(
            warnings
                .iter()
                .any(|text| text.contains("keine Stundenzahl"))
        );
    }

    #[test]
    fn the_shift_sheet_reads_the_times_and_the_band() {
        let (rows, warnings) = read_shifts(&sheet(&[
            "Kürzel; Name; Beginn; Ende; Pause; Band; Abteilung; Nur Azubis; Farbe; Aktiv; Ausgeschlossene Positionen",
            "F; Frühschicht; 06:00; 14:30; 30; früh; Beleuchtung; nein; #2E7D32; ja; Ton, Licht",
            "S-WE; Spätschicht Wochenende; 14:30; 23:00; 30; spät;; nein;; ja;",
            "FT; Frei; ; ; 0; frei;; nein; #7C3AED; ja;",
            "X; Kaputt; 06:00; 06:00; 0; früh;; nein;; ja;",
        ]))
        .unwrap();
        assert_eq!(rows.len(), 3, "{warnings:?}");
        assert_eq!(rows[0].input.band, Some(ShiftBand::Early));
        assert_eq!(rows[0].input.break_minutes, 30);
        assert_eq!(rows[0].input.department.as_deref(), Some("Beleuchtung"));
        assert_eq!(
            rows[0].exclusions.as_deref(),
            Some(["Ton".to_string(), "Licht".to_string()].as_slice())
        );
        assert_eq!(rows[1].input.band, Some(ShiftBand::Late));
        assert_eq!(rows[1].input.department, None);
        assert_eq!(rows[1].exclusions.as_deref(), Some([].as_slice()));
        assert_eq!(rows[2].input.band, Some(ShiftBand::Free));
        // The last row starts and ends at the same time and is refused.
        assert!(
            warnings.iter().any(|text| text.starts_with("Zeile 5")),
            "{warnings:?}"
        );
    }

    #[test]
    fn a_short_sheet_keeps_its_qualifications() {
        let (rows, warnings) = read_employees(
            b"Personalnummer;Vorname;Nachname;Qualifikationen\r\n2001;Test;Person;A, B",
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].input.qualifications, vec!["A", "B"]);
    }

    #[test]
    fn a_file_without_its_head_is_refused() {
        assert!(read_lists(&sheet(&["Name; Abteilung", "A; B"])).is_err());
        assert!(read_employees(&sheet(&["Nachname; Ort", "A; B"])).is_err());
        assert!(read_shifts(&sheet(&["Kürzel; Name", "F; Früh"])).is_err());
    }

    #[test]
    fn the_area_keys_are_the_paths() {
        assert_eq!(MasterArea::from_key("lists"), Some(MasterArea::Lists));
        assert_eq!(
            MasterArea::from_key("Employees"),
            Some(MasterArea::Employees)
        );
        assert_eq!(MasterArea::from_key("shifts"), Some(MasterArea::Shifts));
        assert_eq!(MasterArea::from_key("people"), None);
        assert_eq!(MasterArea::Shifts.key(), "shifts");
    }
}
