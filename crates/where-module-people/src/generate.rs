//! Building a schedule plan from the stored data.
//!
//! The generator fills the cells of a period: for every day it takes the demand
//! of that day (per shift and position) and gives the free cells to the employees
//! that fit, following the rules of the planning (`PlanningRules`) and the
//! catalogue (`Shift::band`, `Shift::apprentice_only`,
//! `Shift::excluded_positions`).
//!
//! Order inside a day: shifts in the order of the catalogue, inside a shift the
//! named positions first, then the open rows (`*`). Regular employees are placed
//! before apprentices, and an apprentice only joins a day that already has a
//! colleague — apprentices assist, they are never alone. The one with the fewest
//! booked hours wins, on equal hours the lower id decides. Cells that are already
//! planned cover their part of the demand. What cannot be filled is reported as a
//! gap.
//!
//! `shift_rotation_days` is a rule, not a wish: the first worked day of a band
//! starts a block that holds the employee in that band until the window has
//! passed. Inside a block the other band is not a choice — the row stays open
//! instead — and the blocks of the employees fill the two shifts of a day. A
//! `Zwischenschicht`, a weekend shift and a free day are no band: they neither
//! end a block nor start one.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

use crate::demand::{ANY_POSITION, DemandEntry, is_foreman};
use crate::planning::PlanningRules;
use crate::schedule::KIND_SHIFT;
use crate::shifts::{Shift, ShiftBand, weekend_marker};

/// Longest period one call generates.
pub const MAX_GENERATE_DAYS: i64 = 92;
/// Days the compensation of a worked Sunday may be moved — an `Ersatzruhetag`
/// has to fall within two weeks.
const COMPENSATION_DAYS: u64 = 14;
/// Days of history the caller reads back for rest time, the running week and the
/// running shift block (`shift_rotation_days` when that is longer).
pub const HISTORY_DAYS: i64 = 8;

/// How many days before the period the generator needs to see.
pub fn history_days(rotation_days: i16) -> i64 {
    HISTORY_DAYS.max(i64::from(rotation_days))
}

/// Why a demand row stayed open.
pub const GAP_NO_EMPLOYEE: &str = "no_employee";
/// The demand of that day falls on a holiday that may not be worked.
pub const GAP_HOLIDAY: &str = "holiday";
/// The location is bound to a sub-department that has nobody on the staff.
pub const GAP_SUBDEPARTMENT: &str = "subdepartment";

/// What the mask sends to build a plan.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateInput {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Replace shift cells that are already planned. Sick, vacation and free
    /// time stay untouched.
    #[serde(default)]
    pub overwrite: bool,
}

/// A demand row the generator could not fill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ScheduleGap {
    pub day: NaiveDate,
    pub shift_id: i64,
    pub position: String,
    /// How many employees are missing for that row.
    pub missing: i32,
    /// [`GAP_NO_EMPLOYEE`], [`GAP_SUBDEPARTMENT`] or [`GAP_HOLIDAY`].
    pub reason: String,
}

/// What the generator did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct GenerateReport {
    /// Days of the period.
    pub days: usize,
    /// Cells written.
    pub assigned: usize,
    /// Cells that stayed as they were.
    pub kept: usize,
    /// Shift cells removed because of `overwrite`.
    pub replaced: usize,
    /// Cells the hours balance added beyond the demand.
    pub extra: usize,
    /// Free days written (`FT`, and the `SoFT` after a worked Sunday).
    pub free_days: usize,
    /// Rows that stayed open.
    pub gaps: Vec<ScheduleGap>,
    /// Days whose disposition could not be read into shifts (`?` on the board).
    pub unclear_days: Vec<NaiveDate>,
}

impl GenerateInput {
    /// Checks the period and returns the days it covers.
    pub fn days(&self) -> Result<Vec<NaiveDate>, DomainError> {
        if self.to < self.from {
            return Err(DomainError::Validation(
                "The end lies before the start.".to_string(),
            ));
        }
        let span = (self.to - self.from).num_days() + 1;
        if span > MAX_GENERATE_DAYS {
            return Err(DomainError::Validation(format!(
                "At most {MAX_GENERATE_DAYS} days can be planned at once."
            )));
        }
        Ok((0..span)
            .map(|offset| self.from + chrono::Days::new(offset as u64))
            .collect())
    }
}

/// An employee as the generator sees them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: i64,
    pub name: String,
    /// Position of the personnel master data; empty when none is kept.
    pub position: String,
    /// Department of the personnel master data; empty when none is kept. The
    /// generator never reads it — a house is only opened by the sub-department.
    pub department: String,
    /// Sub-department under that department; empty when none is kept.
    pub subdepartment: String,
    pub apprentice: bool,
    /// `false` = the employee is only planned by hand; the generator skips them.
    pub auto_plan: bool,
    /// Agreed working days per week; `None` = not set.
    pub work_days: Option<i16>,
    /// Agreed minutes per week; `None` = no hours balance for this employee.
    pub weekly_minutes: Option<i32>,
}

/// `true` when the contract counts that weekday as a working day.
pub fn is_workday(day: NaiveDate, work_days: i16) -> bool {
    match day.weekday() {
        chrono::Weekday::Sat => work_days >= 6,
        chrono::Weekday::Sun => work_days >= 7,
        _ => true,
    }
}

/// Minutes the contract owes on that day; `0` without a contract or on a day the
/// employee does not work.
pub fn target_minutes(person: &Person, day: NaiveDate) -> i32 {
    let (Some(weekly), Some(work_days)) = (person.weekly_minutes, person.work_days) else {
        return 0;
    };
    let work_days = work_days.clamp(1, 7);
    if weekly <= 0 || !is_workday(day, work_days) {
        return 0;
    }
    weekly / i32::from(work_days)
}

/// A cell of the schedule plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub employee_id: i64,
    pub day: NaiveDate,
    /// `shift`, `sick`, `vacation` or `free`.
    pub kind: String,
    pub shift_id: Option<i64>,
    /// Start as `HH:MM`; empty when the row only names a shift.
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
    pub break_minutes: Option<i16>,
}

impl Cell {
    fn is_shift(&self) -> bool {
        self.kind == KIND_SHIFT
    }
}

/// `false` for the foreman row of a house that is run without a foreman.
fn wanted(context: &Context<'_>, row: &DemandEntry) -> bool {
    !(is_foreman(&row.position) && context.foremanless.contains(&key(&row.location)))
}

/// Whether the cell is a worked shift. A free day is stored as a shift too, but
/// nobody works it.
fn works(context: &Context<'_>, cell: &Cell) -> bool {
    cell.is_shift()
        && !shift_of(context.shifts, cell.shift_id)
            .map(|shift| shift.band.is_free())
            .unwrap_or(false)
}

/// A cell the generator wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedCell {
    pub employee_id: i64,
    pub day: NaiveDate,
    pub shift_id: i64,
}

/// The plan of the period.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Generated {
    pub cells: Vec<PlannedCell>,
    /// Free days that close the calendar: `FT`, and `SoFT` as the compensation
    /// after a worked Sunday.
    pub free: Vec<PlannedCell>,
    pub gaps: Vec<ScheduleGap>,
    pub kept: usize,
    pub replaced: usize,
    /// Cells the balance topped up beyond the demand.
    pub extra: usize,
}

/// What the hours balance needs.
#[derive(Debug, Clone, Default)]
pub struct Balance {
    /// Minutes every employee is ahead (+) or behind (−) when the period starts.
    pub before: HashMap<i64, i32>,
    /// Minutes one vacation day counts.
    pub vacation_minutes: i32,
    /// Minutes one sick day counts.
    pub sick_minutes: i32,
}

impl Balance {
    /// Minutes a cell counts for the balance: work as it was planned, an absence
    /// with the hours of the settings, a free day not at all.
    pub fn minutes_of(&self, cell: &Cell, shifts: &[Shift]) -> i32 {
        match cell.kind.as_str() {
            crate::schedule::KIND_SICK => self.sick_minutes,
            crate::schedule::KIND_VACATION => self.vacation_minutes,
            _ => {
                let shift = shift_of(shifts, cell.shift_id);
                if shift.map(|shift| shift.band.is_free()).unwrap_or(false) {
                    return 0;
                }
                let start = cell
                    .starts_at
                    .as_deref()
                    .and_then(minutes)
                    .or_else(|| shift.map(starts));
                let end = cell
                    .ends_at
                    .as_deref()
                    .and_then(minutes)
                    .or_else(|| shift.map(ends));
                let (Some(start), Some(end)) = (start, end) else {
                    return 0;
                };
                let pause = cell
                    .break_minutes
                    .map(i32::from)
                    .or_else(|| shift.map(|shift| i32::from(shift.break_minutes)))
                    .unwrap_or(0);
                ((end - start).rem_euclid(24 * 60) - pause).max(0)
            }
        }
    }
}

/// Everything the generator needs for one run.
pub struct Context<'a> {
    pub rules: &'a PlanningRules,
    pub shifts: &'a [Shift],
    pub people: &'a [Person],
    /// Cells before the period — they carry rest time and the running week.
    pub history: &'a [Cell],
    /// Cells inside the period.
    pub existing: &'a [Cell],
    /// Holidays of the period.
    pub holidays: &'a [NaiveDate],
    /// Demand per day, `day` → shift, position and location.
    pub demand: &'a BTreeMap<NaiveDate, Vec<DemandEntry>>,
    /// Sub-department per location: who may work where. A location without an
    /// entry takes anybody.
    pub subdepartments: &'a HashMap<String, String>,
    /// Locations (lower case) that are run without a foreman; their demand for
    /// the position `Meister` is not planned.
    pub foremanless: &'a BTreeSet<String>,
    /// Hours balance; without it the generator only serves the demand.
    pub balance: Option<Balance>,
    /// The free day that closes every empty day (`FT`).
    pub free_shift: Option<i64>,
    /// The free day that compensates a worked Sunday (`SoFT`).
    pub compensation_shift: Option<i64>,
}

/// `HH:MM` as minutes since midnight.
fn minutes(text: &str) -> Option<i32> {
    let mut parts = text.trim().split(':');
    let hours: i32 = parts.next()?.trim().parse().ok()?;
    let mins: i32 = parts.next()?.trim().parse().ok()?;
    if !(0..=23).contains(&hours) || !(0..=59).contains(&mins) {
        return None;
    }
    Some(hours * 60 + mins)
}

/// Absolute minutes of a moment, for rest periods across days.
fn absolute(day: NaiveDate, minutes_since_midnight: i32) -> i64 {
    day.num_days_from_ce() as i64 * 1440 + i64::from(minutes_since_midnight)
}

/// Start of the shift in minutes since midnight.
fn starts(shift: &Shift) -> i32 {
    minutes(&shift.starts_at).unwrap_or(0)
}

/// End of the shift in minutes since midnight.
fn ends(shift: &Shift) -> i32 {
    minutes(&shift.ends_at).unwrap_or(0)
}

/// Net working minutes of a shift (window minus break).
fn shift_minutes(shift: &Shift) -> i32 {
    let (start, end) = (starts(shift), ends(shift));
    let span = if end > start {
        end - start
    } else {
        end + 24 * 60 - start
    };
    (span - i32::from(shift.break_minutes)).max(0)
}

/// ISO week of a day.
fn week_of(day: NaiveDate) -> (i32, u32) {
    let week = day.iso_week();
    (week.year(), week.week())
}

/// The band a cell writes into the rotation block, when it writes one.
///
/// The weekend shift is exempt: on a Saturday or Sunday it is the only shift the
/// houses have, so it may be worked inside a running block. Hours, days in a row
/// and the rest period still count, so the next working day stays guarded.
fn rotation_band(shift: &Shift) -> Option<ShiftBand> {
    if weekend_marker(shift.short_name.as_deref(), &shift.name) {
        return None;
    }
    Some(shift.band)
}

/// What one employee carries into the next day.
#[derive(Default, Clone)]
struct Record {
    /// End of the last cell as absolute minutes.
    last_end: Option<i64>,
    /// Last day the employee worked.
    last_day: Option<NaiveDate>,
    /// Working days in a row up to `last_day`.
    streak: i32,
    /// Worked minutes per ISO week (`year`, `week`).
    week_minutes: HashMap<(i32, u32), i32>,
    /// Working days per ISO week.
    week_days: HashMap<(i32, u32), i32>,
    /// Minutes booked in this run — the fairness key.
    booked_minutes: i32,
    /// Band and day of the last worked cell.
    last_band: Option<(ShiftBand, NaiveDate)>,
    /// First day of the band block the employee is in.
    band_from: Option<NaiveDate>,
    /// Absolute end of a worked Sunday whose compensation day is still owed.
    open_compensation: Option<i64>,
    /// Absolute end of the schedule a compensation day followed — the schedule after that day
    /// owes the 24 hours of the law plus the ordinary rest.
    compensation_end: Option<i64>,
}

/// Days a band block may be left early when nothing else fits.
///
/// The rotation keeps an employee in one band, but it is a rhythm and not a law:
/// shortly before a block ends — the last [`RELAX_DAYS`] days — the plan may move
/// them to the other band if the row cannot be staffed otherwise. The rest period
/// has to hold as always (`fits` checks it).
pub const RELAX_DAYS: i64 = 3;

impl Record {
    /// The band the employee is bound to on that day, if the block still runs.
    ///
    /// A block starts with the first worked day of a band and keeps the employee
    /// there for `shift_rotation_days` days; only after that the plan may move
    /// them to the other band, which starts the next block.
    fn bound(&self, day: NaiveDate, rotation_days: i16) -> Option<ShiftBand> {
        let (band, _) = self.last_band?;
        let from = self.band_from?;
        if (day - from).num_days() < i64::from(rotation_days) {
            Some(band)
        } else {
            None
        }
    }

    /// The running block has reached its last days, so a switch is allowed when
    /// the row cannot be filled inside the band.
    fn block_ends_soon(&self, day: NaiveDate, rotation_days: i16) -> bool {
        let Some(from) = self.band_from else {
            return false;
        };
        (day - from).num_days() >= i64::from(rotation_days) - RELAX_DAYS
    }

    /// Books a working day into the record.
    fn book(
        &mut self,
        day: NaiveDate,
        minutes: i32,
        end: i32,
        band: Option<ShiftBand>,
        rotation_days: i16,
    ) {
        self.settle_compensation(day, false);
        let (year, week) = week_of(day);
        *self.week_minutes.entry((year, week)).or_insert(0) += minutes;
        *self.week_days.entry((year, week)).or_insert(0) += 1;
        // A day off breaks the row, so only the day before continues it.
        self.streak = if self.last_day == day.pred_opt() {
            self.streak + 1
        } else {
            1
        };
        self.last_day = Some(day);
        self.last_end = Some(absolute(day, end));
        self.booked_minutes += minutes;
        // This schedule pays the compensation day, and a Sunday of its own opens a new one.
        self.compensation_end = None;
        if day.weekday() == chrono::Weekday::Sun {
            self.open_compensation = Some(absolute(day, end));
        }
        // A free day and a Zwischenschicht are no band: they neither end a block
        // nor start one.
        if let Some(band) = band
            && !band.is_free()
            && band != ShiftBand::Intermediate
        {
            let same = self
                .last_band
                .map(|(last, _)| last == band)
                .unwrap_or(false);
            let running = self
                .band_from
                .map(|from| (day - from).num_days() < i64::from(rotation_days))
                .unwrap_or(false);
            if !same || !running {
                self.band_from = Some(day);
            }
            self.last_band = Some((band, day));
        }
    }

    /// Books a day without work — a free day of the plan, sick or vacation.
    fn rest(&mut self, day: NaiveDate) {
        self.settle_compensation(day, true);
    }

    /// A day without work after a worked Sunday is the compensation day of the law:
    /// the 24 hours are measured from the schedule before it. A schedule only settles a
    /// compensation that lies a whole day back.
    fn settle_compensation(&mut self, day: NaiveDate, rest_day: bool) {
        let Some(last_day) = self.last_day else {
            return;
        };
        if self.open_compensation.is_none() {
            return;
        }
        let between = if rest_day {
            last_day < day
        } else {
            last_day < day.pred_opt().unwrap_or(day)
        };
        if between {
            self.compensation_end = self.last_end;
            self.open_compensation = None;
        }
    }
}

/// Fills the plan of `days`.
pub fn generate(context: &Context<'_>, days: &[NaiveDate]) -> Generated {
    let mut records: HashMap<i64, Record> = HashMap::new();
    // Cells before the period carry rest time and the running week.
    for cell in context.history.iter().filter(|cell| works(context, cell)) {
        book_cell(
            &mut records,
            context.shifts,
            cell,
            context.rules.shift_rotation_days,
        );
    }

    let mut taken: BTreeSet<(i64, NaiveDate)> = BTreeSet::new();
    let mut company: BTreeSet<NaiveDate> = BTreeSet::new();
    let mut bands: HashMap<(NaiveDate, ShiftBand, String), i32> = HashMap::new();
    let mut kept = 0usize;
    for cell in context.existing {
        taken.insert((cell.employee_id, cell.day));
        kept += 1;
        if works(context, cell)
            && context
                .people
                .iter()
                .any(|person| person.id == cell.employee_id && !person.apprentice)
        {
            company.insert(cell.day);
        }
        if works(context, cell)
            && let Some(shift) = shift_of(context.shifts, cell.shift_id)
            && let Some(person) = context
                .people
                .iter()
                .find(|person| person.id == cell.employee_id)
        {
            *bands
                .entry((cell.day, shift.band, key(&person.position)))
                .or_insert(0) += 1;
        }
    }

    let mut generated = Generated {
        kept,
        ..Generated::default()
    };

    // What the plan gives every employee in this period, for the hours balance.
    let mut period_minutes: HashMap<i64, i32> = HashMap::new();
    if let Some(balance) = context.balance.as_ref() {
        for cell in context
            .existing
            .iter()
            .filter(|cell| days.contains(&cell.day))
        {
            *period_minutes.entry(cell.employee_id).or_insert(0) +=
                balance.minutes_of(cell, context.shifts);
        }
    }

    for day in days {
        // A house that is run without a foreman never asks for one, whatever the
        // stored demand says: the row is not planned and not reported as a gap.
        let rows: Vec<&DemandEntry> = context
            .demand
            .get(day)
            .map(|rows| rows.iter().filter(|row| wanted(context, row)).collect())
            .unwrap_or_default();
        let demand: &[&DemandEntry] = &rows;
        // The cells that stay carry rest time and hours like a planned one; a
        // free day books nothing of that.
        for cell in context
            .existing
            .iter()
            .filter(|cell| cell.day == *day && works(context, cell))
        {
            book_cell(
                &mut records,
                context.shifts,
                cell,
                context.rules.shift_rotation_days,
            );
        }
        if demand.is_empty() {
            continue;
        }
        if context.holidays.contains(day) && !context.rules.holiday_work_allowed {
            for row in demand {
                generated.gaps.push(ScheduleGap {
                    day: *day,
                    shift_id: row.shift_id,
                    position: row.position.clone(),
                    missing: row.needed,
                    reason: GAP_HOLIDAY.to_string(),
                });
            }
            continue;
        }
        // Every row of the day, with what is still missing.
        let mut jobs: Vec<Job<'_>> = Vec::new();
        for shift in context.shifts.iter().filter(|shift| shift.active) {
            let mut rows: Vec<&DemandEntry> = demand
                .iter()
                .copied()
                .filter(|row| row.shift_id == shift.id)
                .collect();
            // Location, then the named positions, the open row last.
            rows.sort_by(|left, right| {
                (
                    key(&left.location),
                    left.position == ANY_POSITION,
                    key(&left.position),
                )
                    .cmp(&(
                        key(&right.location),
                        right.position == ANY_POSITION,
                        key(&right.position),
                    ))
            });
            let planned = planned_cells(context, *day, shift.id);
            let covered = cover(&rows, &planned, context);
            for (index, row) in rows.into_iter().enumerate() {
                let missing = (row.needed - covered[index]).max(0);
                if missing > 0 {
                    jobs.push(Job {
                        shift,
                        row,
                        missing,
                    });
                }
            }
        }

        // One seat per row and round, so the early and the late shift of a
        // position are staffed before one of them gets a second person: the
        // counter shift. Regular employees first, then the apprentices, and
        // inside that: whoever is already in the band of the row, so the free
        // ones are left for the other band of the day. Without that order the
        // rotation would leave rows open that somebody else could have filled.
        let bands_of_the_day = {
            let mut bands: Vec<ShiftBand> = jobs.iter().map(|job| job.shift.band).collect();
            bands.sort_by_key(|band| *band as u8);
            bands.dedup();
            bands.len() > 1
        };
        for with_apprentices in [false, true] {
            if with_apprentices && context.rules.apprentice_needs_company && !company.contains(day)
            {
                continue;
            }
            for (only_stays, relaxed) in [(bands_of_the_day, false), (false, false), (false, true)]
            {
                loop {
                    let mut progress = false;
                    for job in jobs.iter_mut() {
                        if job.missing <= 0 {
                            continue;
                        }
                        let Some(person) = pick(
                            context,
                            job.shift,
                            job.row,
                            *day,
                            &records,
                            &taken,
                            &bands,
                            with_apprentices,
                            only_stays,
                            relaxed,
                        ) else {
                            continue;
                        };
                        assign(
                            &mut records,
                            &mut taken,
                            &mut company,
                            &mut bands,
                            &mut generated,
                            person,
                            job.shift,
                            *day,
                            context.rules.shift_rotation_days,
                        );
                        *period_minutes.entry(person.id).or_insert(0) += shift_minutes(job.shift);
                        job.missing -= 1;
                        progress = true;
                    }
                    if !progress {
                        break;
                    }
                }
            }
        }

        // Last resort for rows that are still open: a Zwischenschicht standing in
        // for the row. It is the emergency of the plan — it belongs to no band, so
        // the rotation does not bind it, and it covers a row whose own shift
        // nobody of that band can work (a whole crew in the other band, a weekend
        // without the weekend shift). It has to cover at least half of the row's
        // window, otherwise it would not serve the event. The candidates are tried
        // by overlap — the closest one first.
        for with_apprentices in [false, true] {
            for job in jobs.iter_mut() {
                if job.missing <= 0 {
                    continue;
                }
                for zwischen in between_shifts(context, job.shift) {
                    loop {
                        if job.missing <= 0 {
                            break;
                        }
                        let Some(person) = pick(
                            context,
                            zwischen,
                            job.row,
                            *day,
                            &records,
                            &taken,
                            &bands,
                            with_apprentices,
                            false,
                            true,
                        ) else {
                            break;
                        };
                        assign(
                            &mut records,
                            &mut taken,
                            &mut company,
                            &mut bands,
                            &mut generated,
                            person,
                            zwischen,
                            *day,
                            context.rules.shift_rotation_days,
                        );
                        *period_minutes.entry(person.id).or_insert(0) += shift_minutes(zwischen);
                        job.missing -= 1;
                    }
                    if job.missing <= 0 {
                        break;
                    }
                }
            }
        }

        for job in &jobs {
            if job.missing <= 0 {
                continue;
            }
            let subdepartment = context.subdepartments.get(&key(&job.row.location));
            let reason = match subdepartment {
                Some(subdepartment)
                    if !context
                        .people
                        .iter()
                        .any(|person| belongs_to(person, subdepartment)) =>
                {
                    GAP_SUBDEPARTMENT
                }
                _ => GAP_NO_EMPLOYEE,
            };
            generated.gaps.push(ScheduleGap {
                day: *day,
                shift_id: job.shift.id,
                position: job.row.position.clone(),
                missing: job.missing,
                reason: reason.to_string(),
            });
        }
    }

    if let Some(balance) = context.balance.as_ref() {
        fill_balances(
            context,
            balance,
            days,
            &taken,
            &mut company,
            &mut bands,
            &mut generated,
            &mut period_minutes,
        );
    }
    fill_free_days(context, days, &mut generated);
    generated
}

/// Closes the calendar of the period: a day without a cell becomes a free day
/// (`FT`) — and a day that is a Sunday or a public holiday becomes `SoFT`, because
/// a free day there would not be a day off. The first free day after a worked
/// Sunday is its compensation and stays `SoFT` (`compensate_sunday`).
fn fill_free_days(context: &Context<'_>, days: &[NaiveDate], generated: &mut Generated) {
    let Some(free) = context.free_shift else {
        return;
    };
    let compensation = context.compensation_shift.unwrap_or(free);
    for person in context.people {
        let mut taken: BTreeSet<NaiveDate> = BTreeSet::new();
        let mut sundays: Vec<NaiveDate> = Vec::new();
        for cell in context
            .history
            .iter()
            .chain(context.existing.iter())
            .filter(|cell| cell.employee_id == person.id)
        {
            taken.insert(cell.day);
            if works(context, cell) && cell.day.weekday() == chrono::Weekday::Sun {
                sundays.push(cell.day);
            }
        }
        for cell in generated
            .cells
            .iter()
            .filter(|cell| cell.employee_id == person.id)
        {
            taken.insert(cell.day);
            if cell.day.weekday() == chrono::Weekday::Sun {
                sundays.push(cell.day);
            }
        }
        // The first free day after a worked Sunday is its compensation.
        let mut compensating: BTreeSet<NaiveDate> = BTreeSet::new();
        if context.rules.compensate_sunday && context.compensation_shift.is_some() {
            for sunday in sundays {
                for offset in 1..=COMPENSATION_DAYS {
                    let Some(day) = sunday.checked_add_days(chrono::Days::new(offset)) else {
                        break;
                    };
                    if !days.contains(&day) {
                        break;
                    }
                    if taken.contains(&day) {
                        continue;
                    }
                    compensating.insert(day);
                    break;
                }
            }
        }
        for day in days {
            if taken.contains(day) {
                continue;
            }
            // A Sunday or a holiday is no working day: the free day there is the
            // compensation the calendar gives, not a day off the plan granted.
            let ruhetag = day.weekday() == chrono::Weekday::Sun || context.holidays.contains(day);
            generated.free.push(PlannedCell {
                employee_id: person.id,
                day: *day,
                shift_id: if compensating.contains(day) || ruhetag {
                    compensation
                } else {
                    free
                },
            });
        }
    }
}

/// Closes the hours an employee is short of their contract with extra shifts.
///
/// Every employee with a contract (`weekly_minutes` and `work_days`) is compared
/// with the plan: the balance carried into the period plus the minutes it gives
/// against the minutes it owes. A deficit is closed on days the employee is free,
/// preferring the shift that ends closest to zero; the last shift may leave a few
/// overtime hours rather than a gap. Days that the rules do not allow stay free.
#[allow(clippy::too_many_arguments)]
fn fill_balances(
    context: &Context<'_>,
    balance: &Balance,
    days: &[NaiveDate],
    taken: &BTreeSet<(i64, NaiveDate)>,
    company: &mut BTreeSet<NaiveDate>,
    bands: &mut HashMap<(NaiveDate, ShiftBand, String), i32>,
    generated: &mut Generated,
    period_minutes: &mut HashMap<i64, i32>,
) {
    let mut taken = taken.clone();
    // Regular employees first: their extra shift is company for an apprentice.
    for with_apprentices in [false, true] {
        for person in context.people.iter().filter(|person| {
            person.auto_plan
                && person.weekly_minutes.is_some()
                && person.apprentice == with_apprentices
        }) {
            if person.work_days.is_none() {
                continue;
            }
            let owe: i32 = days.iter().map(|day| target_minutes(person, *day)).sum();
            if owe <= 0 {
                continue;
            }
            let mut need = owe
                - balance.before.get(&person.id).copied().unwrap_or(0)
                - period_minutes.get(&person.id).copied().unwrap_or(0);
            if need <= 0 {
                continue;
            }
            // The cells of this employee that are already there: an extra shift
            // has to stay in their bands.
            let mut known: Vec<(NaiveDate, ShiftBand, i32)> = Vec::new();
            for (day, shift_id) in context
                .history
                .iter()
                .chain(context.existing.iter())
                .map(|cell| (cell.employee_id, cell.day, cell.shift_id))
                .chain(
                    generated
                        .cells
                        .iter()
                        .map(|cell| (cell.employee_id, cell.day, Some(cell.shift_id))),
                )
                .filter(|(employee_id, _, _)| *employee_id == person.id)
                .map(|(_, day, shift_id)| (day, shift_id))
                .filter(|(_, shift_id)| shift_id.is_some())
            {
                let Some(shift) = shift_of(context.shifts, shift_id) else {
                    continue;
                };
                // A weekend shift, a Zwischenschicht and a free day say nothing
                // about the band of the working days around them.
                if shift.band.is_free() || rotation_band(shift).is_none() {
                    continue;
                }
                known.push((day, shift.band, starts(shift)));
            }
            known.sort_by_key(|(day, _, _)| *day);
            // What the employee already has, independent of the order in which the
            // days are walked: the days and the minutes per week. An extra shift has
            // to fit into both, including the cells that come after it.
            let mut days_planned: BTreeSet<NaiveDate> = BTreeSet::new();
            let mut week_planned: HashMap<(i32, u32), i32> = HashMap::new();
            for cell in context
                .history
                .iter()
                .chain(context.existing.iter())
                .filter(|cell| cell.employee_id == person.id)
            {
                if is_free_cell(context.shifts, cell) {
                    continue;
                }
                days_planned.insert(cell.day);
                let (year, week) = week_of(cell.day);
                *week_planned.entry((year, week)).or_insert(0) +=
                    balance.minutes_of(cell, context.shifts);
            }
            for cell in generated
                .cells
                .iter()
                .filter(|cell| cell.employee_id == person.id)
            {
                days_planned.insert(cell.day);
                let (year, week) = week_of(cell.day);
                *week_planned.entry((year, week)).or_insert(0) +=
                    shift_of(context.shifts, Some(cell.shift_id))
                        .map(shift_minutes)
                        .unwrap_or(0);
            }
            // The books of this employee, in the order of the days.
            let mut own: HashMap<i64, Record> = HashMap::new();
            for cell in context
                .history
                .iter()
                .filter(|cell| cell.employee_id == person.id)
            {
                book_cell(
                    &mut own,
                    context.shifts,
                    cell,
                    context.rules.shift_rotation_days,
                );
            }
            for day in days {
                if let Some(cell) = context
                    .existing
                    .iter()
                    .find(|cell| cell.employee_id == person.id && cell.day == *day)
                {
                    book_cell(
                        &mut own,
                        context.shifts,
                        cell,
                        context.rules.shift_rotation_days,
                    );
                } else if let Some(cell) = generated
                    .cells
                    .iter()
                    .find(|cell| cell.employee_id == person.id && cell.day == *day)
                    && let Some(shift) = shift_of(context.shifts, Some(cell.shift_id))
                {
                    own.entry(person.id).or_default().book(
                        *day,
                        shift_minutes(shift),
                        ends(shift),
                        rotation_band(shift),
                        context.rules.shift_rotation_days,
                    );
                }
                if need <= 0
                    || taken.contains(&(person.id, *day))
                    || target_minutes(person, *day) <= 0
                {
                    continue;
                }
                if context.holidays.contains(day) && !context.rules.holiday_work_allowed {
                    continue;
                }
                if person.apprentice
                    && context.rules.apprentice_needs_company
                    && !company.contains(day)
                {
                    continue;
                }
                let record = own.entry(person.id).or_default().clone();
                let block = extra_band(&record, &known, *day, context.rules.shift_rotation_days);
                let next = known
                    .iter()
                    .find(|(next_day, _, _)| *next_day > *day)
                    .map(|(next_day, _, start)| (*next_day, *start));
                let Some(shift) = extra_shift(context, person, *day, need, block, next) else {
                    continue;
                };
                if !fits(context, shift, person, *day, &record, false) {
                    continue;
                }
                // An extra shift may not bridge two planned runs, and the week it
                // falls in already carries the cells that come after it: the record
                // only knows the days before.
                let before = if record.last_day == day.pred_opt() {
                    record.streak
                } else {
                    0
                };
                if before + 1 + run_after(&days_planned, *day)
                    > i32::from(context.rules.max_consecutive_days)
                {
                    continue;
                }
                let (year, week) = week_of(*day);
                let net = shift_minutes(shift);
                if week_planned.get(&(year, week)).copied().unwrap_or(0) + net
                    > i32::from(context.rules.max_weekly_hours) * 60
                {
                    continue;
                }
                let net = shift_minutes(shift);
                own.entry(person.id).or_default().book(
                    *day,
                    net,
                    ends(shift),
                    rotation_band(shift),
                    context.rules.shift_rotation_days,
                );
                taken.insert((person.id, *day));
                *bands
                    .entry((*day, shift.band, key(&person.position)))
                    .or_insert(0) += 1;
                days_planned.insert(*day);
                *week_planned.entry((year, week)).or_insert(0) += net;
                generated.cells.push(PlannedCell {
                    employee_id: person.id,
                    day: *day,
                    shift_id: shift.id,
                });
                generated.extra += 1;
                *period_minutes.entry(person.id).or_insert(0) += net;
                need -= net;
                if !person.apprentice {
                    company.insert(*day);
                }
            }
        }
    }
}

/// How many days after `day` are already planned, one after the other.
///
/// The cells an employee already has are the ones his records do not know yet
/// when a shift is booked for a day before them.
fn run_after(planned: &BTreeSet<NaiveDate>, day: NaiveDate) -> i32 {
    let mut run = 0;
    let mut next = day + chrono::Days::new(1);
    while planned.contains(&next) {
        run += 1;
        next = next + chrono::Days::new(1);
    }
    run
}

/// `true` when the cell is a free day (`FT`, `SoFT`): no work, no hours.
fn is_free_cell(shifts: &[Shift], cell: &Cell) -> bool {
    shift_of(shifts, cell.shift_id)
        .map(|shift| shift.band.is_free())
        .unwrap_or(false)
}

/// The band an extra shift of that day belongs to.
///
/// The running block comes first. Is there none, the next planned cell decides:
/// an extra shift that starts a block must not end up in the other band a few
/// days before a cell that is already there.
fn extra_band(
    record: &Record,
    known: &[(NaiveDate, ShiftBand, i32)],
    day: NaiveDate,
    rotation_days: i16,
) -> Option<ShiftBand> {
    if let Some(bound) = record.bound(day, rotation_days) {
        return Some(bound);
    }
    known
        .iter()
        .find(|(entry_day, _, _)| {
            *entry_day > day && (*entry_day - day).num_days() < i64::from(rotation_days)
        })
        .map(|(_, band, _)| *band)
}

/// The shift that fills a deficit best: one that covers it with the smallest
/// overtime, otherwise the longest one. Zwischen- and free shifts are no extra
/// work, a working day takes no weekend shift and the weekend takes nothing else,
/// and only what the person may work comes into question.
fn extra_shift<'a>(
    context: &Context<'a>,
    person: &Person,
    day: NaiveDate,
    need: i32,
    block: Option<ShiftBand>,
    next: Option<(NaiveDate, i32)>,
) -> Option<&'a Shift> {
    let mut best: Option<((bool, i32, usize), &Shift)> = None;
    // An extra shift follows the running block like a planned one.
    let bound = block;
    let weekend = matches!(day.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun);
    for (order, shift) in context.shifts.iter().enumerate() {
        if !shift.active || shift.band.is_free() || shift.band == ShiftBand::Intermediate {
            continue;
        }
        if bound.is_some_and(|bound| match rotation_band(shift) {
            Some(band) => bound != band,
            None => false,
        }) {
            continue;
        }
        if person.apprentice != shift.apprentice_only {
            continue;
        }
        if weekend != weekend_marker(shift.short_name.as_deref(), &shift.name) {
            continue;
        }
        // The cell after this day keeps its rest period as well.
        if let Some((next_day, start)) = next
            && absolute(next_day, start) - absolute(day, ends(shift))
                < i64::from(context.rules.min_rest_hours) * 60
        {
            continue;
        }
        if shift
            .excluded_positions
            .iter()
            .any(|position| position.trim().eq_ignore_ascii_case(person.position.trim()))
        {
            continue;
        }
        let net = shift_minutes(shift);
        if net <= 0 {
            continue;
        }
        let covers = need - net <= 0;
        let ranking = (!covers, (need - net).abs(), order);
        if best.map(|(best, _)| ranking < best).unwrap_or(true) {
            best = Some((ranking, shift));
        }
    }
    best.map(|(_, shift)| shift)
}

/// One object of the day: a row of a shift that still needs somebody.
struct Job<'a> {
    shift: &'a Shift,
    row: &'a DemandEntry,
    /// Employees the row still misses.
    missing: i32,
}
/// Comparison key of a name: trimmed and lower case.
fn key(text: &str) -> String {
    text.trim().to_lowercase()
}

/// `true` when the person works in the sub-department a house asks for.
///
/// The sub-department alone decides, never the department above it: a location
/// names the sub-department that works there, and the department is only the roof
/// over several houses. So an employee without a sub-department is planned by hand
/// only, whatever the department says.
fn belongs_to(person: &Person, subdepartment: &str) -> bool {
    person
        .subdepartment
        .trim()
        .eq_ignore_ascii_case(subdepartment)
}

/// Position and sub-department of every cell already planned in that shift on that
/// day.
fn planned_cells(context: &Context<'_>, day: NaiveDate, shift_id: i64) -> Vec<(String, String)> {
    context
        .existing
        .iter()
        .filter(|cell| cell.day == day && works(context, cell) && cell.shift_id == Some(shift_id))
        .filter_map(|cell| {
            context
                .people
                .iter()
                .find(|person| person.id == cell.employee_id)
                // A cell serves the row of its position and of its sub-department —
                // a house sees only the cells of its own crew as covered.
                .map(|person| (key(&person.position), key(&person.subdepartment)))
        })
        .collect()
}

/// How much of every row the cells that stay already cover.
///
/// A cell serves the row of its own position and of its own sub-department: the
/// demand of one house is not covered by somebody who works in the other one.
/// Whatever is left serves the open row of that house.
fn cover(rows: &[&DemandEntry], planned: &[(String, String)], context: &Context<'_>) -> Vec<i32> {
    let mut covered = vec![0i32; rows.len()];
    let mut index = 0;
    while index < rows.len() {
        let location = key(&rows[index].location);
        let mut end = index + 1;
        while end < rows.len() && key(&rows[end].location) == location {
            end += 1;
        }
        let subdepartment = context.subdepartments.get(&location).map(|name| key(name));
        let mut left = planned
            .iter()
            .filter(|(_, own)| match &subdepartment {
                Some(subdepartment) => own == subdepartment,
                None => true,
            })
            .count() as i32;
        for row_index in index..end {
            let row = rows[row_index];
            if row.position == ANY_POSITION {
                continue;
            }
            let position = key(&row.position);
            let own = planned
                .iter()
                .filter(|(own, own_subdepartment)| {
                    *own == position
                        && match &subdepartment {
                            Some(subdepartment) => own_subdepartment == subdepartment,
                            None => true,
                        }
                })
                .count() as i32;
            let used = own.min(row.needed).min(left);
            covered[row_index] = used;
            left -= used;
        }
        for row_index in index..end {
            let row = rows[row_index];
            if row.position != ANY_POSITION {
                continue;
            }
            let used = left.min(row.needed);
            covered[row_index] = used;
            left -= used;
        }
        index = end;
    }
    covered
}

/// The shift of the catalogue.
fn shift_of(shifts: &[Shift], shift_id: Option<i64>) -> Option<&Shift> {
    let id = shift_id?;
    shifts.iter().find(|shift| shift.id == id)
}

/// Books a cell that exists already — history or kept — into the run.
fn book_cell(
    records: &mut HashMap<i64, Record>,
    shifts: &[Shift],
    cell: &Cell,
    rotation_days: i16,
) {
    let shift = shift_of(shifts, cell.shift_id);
    // A free day is no work: it books no hours and keeps no rest time — but it is the
    // compensation day a worked Sunday asks for.
    if shift.map(|shift| shift.band.is_free()).unwrap_or(false) {
        records.entry(cell.employee_id).or_default().rest(cell.day);
        return;
    }
    // A shift cell keeps no times of its own, they come from the catalogue.
    let start = cell
        .starts_at
        .as_deref()
        .and_then(minutes)
        .or_else(|| shift.map(starts));
    let end = cell
        .ends_at
        .as_deref()
        .and_then(minutes)
        .or_else(|| shift.map(ends));
    let (Some(start), Some(end)) = (start, end) else {
        return;
    };
    let pause = cell
        .break_minutes
        .map(i32::from)
        .or_else(|| shift.map(|shift| i32::from(shift.break_minutes)))
        .unwrap_or(0);
    let net = (end - start).rem_euclid(24 * 60) - pause;
    let band = shift.and_then(rotation_band);
    records.entry(cell.employee_id).or_default().book(
        cell.day,
        net.max(0),
        end,
        band,
        rotation_days,
    );
}

/// Writes one assignment into the books of the run.
#[allow(clippy::too_many_arguments)]
fn assign(
    records: &mut HashMap<i64, Record>,
    taken: &mut BTreeSet<(i64, NaiveDate)>,
    company: &mut BTreeSet<NaiveDate>,
    bands: &mut HashMap<(NaiveDate, ShiftBand, String), i32>,
    generated: &mut Generated,
    person: &Person,
    shift: &Shift,
    day: NaiveDate,
    rotation_days: i16,
) {
    records.entry(person.id).or_default().book(
        day,
        shift_minutes(shift),
        ends(shift),
        rotation_band(shift),
        rotation_days,
    );
    taken.insert((person.id, day));
    *bands
        .entry((day, shift.band, key(&person.position)))
        .or_insert(0) += 1;
    if !person.apprentice {
        company.insert(day);
    }
    generated.cells.push(PlannedCell {
        employee_id: person.id,
        day,
        shift_id: shift.id,
    });
}

/// Minutes two shift windows share; a window running past midnight is cut at it.
fn overlap(a: &Shift, b: &Shift) -> i32 {
    let (Some(a_start), Some(a_end)) = (minutes(&a.starts_at), minutes(&a.ends_at)) else {
        return 0;
    };
    let (Some(b_start), Some(b_end)) = (minutes(&b.starts_at), minutes(&b.ends_at)) else {
        return 0;
    };
    if a_end <= a_start || b_end <= b_start {
        return 0;
    }
    (a_end.min(b_end) - a_start.max(b_start)).max(0)
}

/// The Zwischenschichten that can stand in for a row of that shift, best first.
///
/// One qualifies when it covers at least half of the row's window — the event has
/// to be served, not just the first hours of it. Candidates that cover the same
/// window keep the order of the catalogue.
fn between_shifts<'a>(context: &Context<'a>, shift: &Shift) -> Vec<&'a Shift> {
    let (Some(start), Some(end)) = (minutes(&shift.starts_at), minutes(&shift.ends_at)) else {
        return Vec::new();
    };
    let row = (end - start).rem_euclid(24 * 60);
    if row <= 0 {
        return Vec::new();
    }
    let mut kandidaten: Vec<&Shift> = context
        .shifts
        .iter()
        .filter(|kandidat| kandidat.active && kandidat.band == ShiftBand::Intermediate)
        .filter(|kandidat| !kandidat.apprentice_only)
        .filter(|kandidat| overlap(shift, kandidat) * 2 >= row)
        .collect();
    kandidaten.sort_by_key(|kandidat| {
        (
            std::cmp::Reverse(overlap(shift, kandidat)),
            kandidat.sort_order,
        )
    });
    kandidaten
}

/// The sub-department a row asks for, when its location is bound to one.
fn row_subdepartment<'a>(context: &'a Context<'_>, row: &DemandEntry) -> Option<&'a str> {
    context
        .subdepartments
        .get(&key(&row.location))
        .map(|name| name.as_str())
}

/// The best free employee for one row, if any fits.
#[allow(clippy::too_many_arguments)]
fn pick<'a>(
    context: &Context<'a>,
    shift: &Shift,
    row: &DemandEntry,
    day: NaiveDate,
    records: &HashMap<i64, Record>,
    taken: &BTreeSet<(i64, NaiveDate)>,
    bands: &HashMap<(NaiveDate, ShiftBand, String), i32>,
    with_apprentices: bool,
    only_stays: bool,
    relaxed: bool,
) -> Option<&'a Person> {
    let open = row.position == ANY_POSITION;
    let subdepartment = row_subdepartment(context, row);
    let mut best: Option<((bool, i32, i32, std::cmp::Reverse<bool>, i64), &Person)> = None;
    for person in context.people {
        if taken.contains(&(person.id, day)) {
            continue;
        }
        // Somebody marked `nicht automatisch disponierbar` is planned by hand only.
        if !person.auto_plan {
            continue;
        }
        if person.apprentice != with_apprentices {
            continue;
        }
        // The generator gives an apprentice only their own shifts, and nobody else
        // works theirs. Hand-written cells may differ (see `schedule_repo::set`).
        if person.apprentice != shift.apprentice_only {
            continue;
        }
        if let Some(subdepartment) = subdepartment
            && !belongs_to(person, subdepartment)
        {
            continue;
        }
        if !open
            && !person
                .position
                .trim()
                .eq_ignore_ascii_case(row.position.trim())
        {
            continue;
        }
        if shift
            .excluded_positions
            .iter()
            .any(|excluded| excluded.trim().eq_ignore_ascii_case(person.position.trim()))
        {
            continue;
        }
        let record = records.get(&person.id).cloned().unwrap_or_default();
        if !fits(context, shift, person, day, &record, relaxed) {
            continue;
        }
        let stays = record.bound(day, context.rules.shift_rotation_days) == Some(shift.band);
        // The pass that keeps the blocks of the day together only takes the
        // employees who are already in that band.
        if only_stays && !stays {
            continue;
        }
        // Fewest booked hours first — but a block that has to reach
        // `min_consecutive_days` is continued first, so a day off only follows a
        // block of that length. Then a position that the band does not carry yet
        // (the counter shift), then the employee whose running block is that band
        // (they stay), then the lower id.
        let minutes = record.booked_minutes;
        let streak = record.streak;
        // A block was started and has not reached the minimum yet.
        let continues = streak >= 1 && streak < i32::from(context.rules.min_consecutive_days);
        let same_band = bands
            .get(&(day, shift.band, key(&person.position)))
            .copied()
            .unwrap_or(0);
        let key = (
            !continues,
            minutes,
            same_band,
            std::cmp::Reverse(stays),
            person.id,
        );
        if best.map(|(best_key, _)| key < best_key).unwrap_or(true) {
            best = Some((key, person));
        }
    }
    best.map(|(_, person)| person)
}

/// Whether the rules allow the employee that shift on that day.
///
/// `relaxed` allows leaving a band block in its last days (see [`RELAX_DAYS`]);
/// the rest of the rules always hold.
fn fits(
    context: &Context<'_>,
    shift: &Shift,
    person: &Person,
    day: NaiveDate,
    record: &Record,
    relaxed: bool,
) -> bool {
    let rules = context.rules;
    let rest = i64::from(rules.min_rest_hours) * 60;
    // Rest period between two cells — the ordinary one, a Monday after Sunday work
    // included.
    if let Some(last_end) = record.last_end {
        let pause = absolute(day, starts(shift)) - last_end;
        if pause < rest {
            return false;
        }
    }
    // The compensation day a worked Sunday asks for: with `extended_sunday_rest` the
    // schedule after that day owes 24 hours plus the ordinary rest.
    if rules.extended_sunday_rest
        && let Some(base) = record.compensation_end
        && absolute(day, starts(shift)) - base < rest + 24 * 60
    {
        return false;
    }
    // Working time per day and per week.
    let net = shift_minutes(shift);
    if net > i32::from(rules.max_daily_hours) * 60 {
        return false;
    }
    let (year, week) = week_of(day);
    let week_minutes = record.week_minutes.get(&(year, week)).copied().unwrap_or(0);
    if week_minutes + net > i32::from(rules.max_weekly_hours) * 60 {
        return false;
    }
    // Days in a row.
    let streak = if record.last_day == day.pred_opt() {
        record.streak + 1
    } else {
        1
    };
    if streak > i32::from(rules.max_consecutive_days) {
        return false;
    }
    // Days off per week and the agreed working days.
    let week_days = record.week_days.get(&(year, week)).copied().unwrap_or(0);
    let free_days = 7 - i32::from(rules.min_days_off_per_week);
    let allowed = person
        .work_days
        .map(i32::from)
        .unwrap_or(free_days)
        .min(free_days);
    if week_days + 1 > allowed {
        return false;
    }
    // Sunday work.
    if day.weekday() == chrono::Weekday::Sun && !rules.sunday_work_allowed {
        return false;
    }
    // Saturday and Sunday take no regular late shift (`S`), only the weekend
    // shift (`S-WE`).
    if matches!(day.weekday(), chrono::Weekday::Sat | chrono::Weekday::Sun)
        && !shift.works_weekend()
    {
        return false;
    }
    // The shift rotation: a running block only allows its own band, a
    // Zwischenschicht and a weekend shift stay the exception. In the last days of
    // a block the plan may switch when the row cannot be filled otherwise
    // (`relaxed`).
    if let Some(band) = rotation_band(shift)
        && let Some(bound) = record.bound(day, rules.shift_rotation_days)
        && bound != band
        && !band.is_free()
        && band != ShiftBand::Intermediate
        && !(relaxed && record.block_ends_soon(day, rules.shift_rotation_days))
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn shift(
        id: i64,
        short: &str,
        starts_at: &str,
        ends_at: &str,
        band: ShiftBand,
        apprentice_only: bool,
    ) -> Shift {
        Shift {
            id,
            name: short.to_string(),
            short_name: Some(short.to_string()),
            starts_at: starts_at.to_string(),
            ends_at: ends_at.to_string(),
            break_minutes: 30,
            band,
            apprentice_only,
            color: None,
            department: None,
            active: true,
            sort_order: id as i32,
            excluded_positions: Vec::new(),
        }
    }

    fn person(id: i64, name: &str, position: &str, apprentice: bool) -> Person {
        Person {
            id,
            name: name.to_string(),
            position: position.to_string(),
            department: String::new(),
            subdepartment: String::new(),
            apprentice,
            auto_plan: true,
            work_days: None,
            weekly_minutes: None,
        }
    }

    /// The same employee, working in a sub-department.
    fn of_subdepartment(mut person: Person, subdepartment: &str) -> Person {
        person.subdepartment = subdepartment.to_string();
        person
    }

    /// The same employee, bound to a department but to no sub-department.
    fn of_department(mut person: Person, department: &str) -> Person {
        person.department = department.to_string();
        person
    }

    /// One demand row with a location.
    fn row(shift_id: i64, position: &str, location: &str, needed: i32) -> DemandEntry {
        DemandEntry {
            shift_id,
            position: position.to_string(),
            location: location.to_string(),
            needed,
        }
    }

    /// A house and the sub-department that works there.
    fn houses(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(house, subdepartment)| (house.to_lowercase(), (*subdepartment).to_string()))
            .collect()
    }

    /// A shift cell as the board keeps it: the times come from the catalogue.
    fn cell(employee_id: i64, day: &str, shift_id: i64) -> Cell {
        Cell {
            employee_id,
            day: date(day),
            kind: KIND_SHIFT.to_string(),
            shift_id: Some(shift_id),
            starts_at: None,
            ends_at: None,
            break_minutes: None,
        }
    }

    fn demand_row(
        day: &str,
        shift_id: i64,
        position: &str,
        needed: i32,
    ) -> (NaiveDate, Vec<DemandEntry>) {
        (
            date(day),
            vec![DemandEntry {
                shift_id,
                position: position.to_string(),
                location: String::new(),
                needed,
            }],
        )
    }

    struct Setup<'a> {
        rules: &'a PlanningRules,
        shifts: &'a [Shift],
        people: &'a [Person],
        existing: &'a [Cell],
        history: &'a [Cell],
        holidays: &'a [NaiveDate],
        demand: &'a BTreeMap<NaiveDate, Vec<DemandEntry>>,
        subdepartments: &'a HashMap<String, String>,
        foremanless: &'a BTreeSet<String>,
        balance: Option<Balance>,
    }

    fn run(setup: &Setup<'_>, days: &[NaiveDate]) -> Generated {
        run_parts(setup, days, None, None)
    }

    /// The same run with the free days of the calendar (`FT` / `SoFT`).
    fn run_parts(
        setup: &Setup<'_>,
        days: &[NaiveDate],
        free_shift: Option<i64>,
        compensation_shift: Option<i64>,
    ) -> Generated {
        let context = Context {
            rules: setup.rules,
            shifts: setup.shifts,
            people: setup.people,
            history: setup.history,
            existing: setup.existing,
            holidays: setup.holidays,
            demand: setup.demand,
            subdepartments: setup.subdepartments,
            foremanless: setup.foremanless,
            balance: setup.balance.clone(),
            free_shift,
            compensation_shift,
        };
        generate(&context, days)
    }

    #[test]
    fn a_day_without_demand_writes_nothing() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Meister", "Meister", false)];
        let demand = BTreeMap::new();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn the_demand_is_filled_with_the_matching_position() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![
            person(1, "Meister", "Meister", false),
            person(2, "Stellwerker", "Stellwerker", false),
        ];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.cells[0].employee_id, 1);
        assert!(generated.gaps.is_empty());
    }

    /// One week, Monday to Friday.
    fn week() -> Vec<NaiveDate> {
        (7..=11)
            .map(|day| date(&format!("2026-12-{day:02}")))
            .collect()
    }

    fn with_contract(mut person: Person, weekly_minutes: i32, work_days: i16) -> Person {
        person.weekly_minutes = Some(weekly_minutes);
        person.work_days = Some(work_days);
        person
    }

    fn empty_balance() -> Option<Balance> {
        Some(Balance {
            before: HashMap::new(),
            vacation_minutes: 480,
            sick_minutes: 480,
        })
    }

    #[test]
    fn a_deficit_is_closed_with_extra_shifts() {
        // 30 hours a week, one demanded shift on Monday: the other four days get
        // topped up, the last shift covers the rest of the deficit.
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![with_contract(
            person(1, "Meister", "Meister", false),
            1800,
            5,
        )];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-07", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &week());
        assert_eq!(generated.cells.len(), 5);
        assert_eq!(generated.extra, 4);
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn a_balanced_period_gets_no_extra_shift() {
        // 7 hours a week in five days is 84 minutes a day: the one demanded shift
        // is exactly what the week owes.
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![with_contract(
            person(1, "Meister", "Meister", false),
            420,
            5,
        )];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-07", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &week());
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.extra, 0);
    }

    #[test]
    fn a_deficit_from_before_the_period_is_carried_in() {
        // The week owes 7 hours and gets one shift; a deficit of the same size
        // carried in from the months before is closed with one extra shift.
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![with_contract(
            person(1, "Meister", "Meister", false),
            420,
            5,
        )];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-07", 1, "Meister", 1);
        demand.insert(day, rows);
        let balance = Some(Balance {
            before: HashMap::from([(1, -420)]),
            vacation_minutes: 480,
            sick_minutes: 480,
        });
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance,
        };
        let generated = run(&setup, &week());
        assert_eq!(generated.cells.len(), 2);
        assert_eq!(generated.extra, 1);
    }

    #[test]
    fn an_apprentice_gets_extras_next_to_a_colleague() {
        // No demand at all: the regular employee fills the week first, which makes
        // the days company for the apprentice — so both contracts are served, each
        // in its own shift.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(6, "F-AZ", "08:00", "15:30", ShiftBand::Early, true),
        ];
        let people = vec![
            with_contract(person(1, "Meister", "Meister", false), 1800, 5),
            with_contract(person(2, "Azubi", "Meister", true), 1800, 5),
        ];
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &BTreeMap::new(),
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &week());
        assert_eq!(generated.extra, 10);
        assert_eq!(
            generated
                .cells
                .iter()
                .filter(|cell| cell.employee_id == 2)
                .count(),
            5
        );
    }

    #[test]
    fn an_extra_shift_keeps_the_maximum_week() {
        // The demanded shift fills the week to its limit, so no extra fits.
        let rules = PlanningRules {
            max_weekly_hours: 7,
            ..PlanningRules::default()
        };
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![with_contract(
            person(1, "Meister", "Meister", false),
            1800,
            5,
        )];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-07", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &week());
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.extra, 0);
    }

    #[test]
    fn a_person_who_is_not_plannable_gets_no_cell() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let mut manual = person(1, "Meister", "Meister", false);
        manual.auto_plan = false;
        let people = vec![manual];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].missing, 1);
    }

    #[test]
    fn a_cell_of_a_person_who_is_not_plannable_still_covers_the_row() {
        // A cell set by hand keeps its employee, even when that employee may no
        // longer be planned automatically.
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let mut manual = person(1, "Meister", "Meister", false);
        manual.auto_plan = false;
        let people = vec![manual];
        let existing = vec![Cell {
            employee_id: 1,
            day: date("2026-12-01"),
            kind: KIND_SHIFT.to_string(),
            shift_id: Some(1),
            starts_at: None,
            ends_at: None,
            break_minutes: None,
        }];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert!(generated.gaps.is_empty());
        assert_eq!(generated.kept, 1);
    }

    #[test]
    fn a_house_without_a_foreman_plans_no_meister() {
        // The demand names a foreman for the podium, but the house is run without
        // one: no cell and no gap — the row simply does not exist.
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Meister", "Meister", false)];
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-01"), vec![row(1, "Meister", "Podium", 1)]);
        let foremanless = BTreeSet::from(["podium".to_string()]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &foremanless,
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn a_missing_position_becomes_a_gap() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Stellwerker", "Stellwerker", false)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);
        assert_eq!(generated.gaps[0].missing, 1);
    }

    #[test]
    fn the_rest_period_keeps_an_employee_out() {
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        let history = vec![cell(1, "2026-11-30", 2)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        // 08:00 is only nine hours after 23:00.
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);
    }

    #[test]
    fn a_holiday_without_work_leaves_the_demand_open() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Meister", "Meister", false)];
        let holidays = vec![date("2026-12-25")];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-25", 1, "Meister", 2);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &holidays,
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-25")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].reason, GAP_HOLIDAY);
        assert_eq!(generated.gaps[0].missing, 2);
    }

    #[test]
    fn a_planned_cell_keeps_its_employee() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Meister", "Meister", false)];
        let existing = vec![cell(1, "2026-12-01", 1)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.kept, 1);
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn an_apprentice_never_fills_a_shift_alone() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![
            person(1, "Meister", "Meister", false),
            person(2, "Azubi", "Auszubildender", true),
        ];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Auszubildender", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        // Nobody else is in the house that day.
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);
    }

    #[test]
    fn a_non_apprentice_stays_out_of_an_apprentice_shift() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "S-AZ", "14:30", "23:00", ShiftBand::Late, true)];
        let people = vec![person(1, "Meister", "Meister", false)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].missing, 1);
    }

    #[test]
    fn the_fewest_hours_win_the_next_day() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![
            person(1, "Anna", "Meister", false),
            person(2, "Bernd", "Meister", false),
        ];
        let mut demand = BTreeMap::new();
        let (day1, rows1) = demand_row("2026-12-01", 1, "Meister", 1);
        let (day2, rows2) = demand_row("2026-12-02", 1, "Meister", 1);
        demand.insert(day1, rows1);
        demand.insert(day2, rows2);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01"), date("2026-12-02")]);
        assert_eq!(generated.cells.len(), 2);
        // The hours are balanced between the two.
        assert_ne!(
            generated.cells[0].employee_id,
            generated.cells[1].employee_id
        );
    }

    #[test]
    fn a_planned_cell_of_another_position_does_not_cover_the_row() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(2, "Stellwerker", "Stellwerker", false)];
        let existing = vec![cell(2, "2026-12-01", 1)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].missing, 1);
    }

    #[test]
    fn the_band_of_yesterday_outranks_the_lower_id() {
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:30", "23:00", ShiftBand::Late, false),
        ];
        let people = vec![
            person(1, "Anna", "Meister", false),
            person(2, "Bernd", "Meister", false),
        ];
        // Both worked yesterday with the same hours, in opposite bands.
        let history = vec![cell(1, "2026-11-30", 2), cell(2, "2026-11-30", 1)];
        let mut demand = BTreeMap::new();
        let day = date("2026-12-01");
        demand.insert(
            day,
            vec![
                DemandEntry {
                    shift_id: 1,
                    position: "Meister".to_string(),
                    location: String::new(),
                    needed: 1,
                },
                DemandEntry {
                    shift_id: 2,
                    position: "Meister".to_string(),
                    location: String::new(),
                    needed: 1,
                },
            ],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[day]);
        assert_eq!(generated.cells.len(), 2);
        let early = generated.cells.iter().find(|cell| cell.shift_id == 1);
        let late = generated.cells.iter().find(|cell| cell.shift_id == 2);
        assert_eq!(early.map(|cell| cell.employee_id), Some(2));
        assert_eq!(late.map(|cell| cell.employee_id), Some(1));
    }

    #[test]
    fn a_kept_cell_keeps_the_rest_period() {
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        // The late shift of the first day stays; the early shift of the next
        // day is only nine hours later.
        let existing = vec![cell(1, "2026-12-01", 2)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-02", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01"), date("2026-12-02")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);
    }

    #[test]
    fn a_cell_of_a_named_position_also_serves_the_open_row() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Meister", "Meister", false)];
        let existing = vec![cell(1, "2026-12-01", 1)];
        let mut demand = BTreeMap::new();
        let day = date("2026-12-01");
        demand.insert(
            day,
            vec![
                DemandEntry {
                    shift_id: 1,
                    position: "Meister".to_string(),
                    location: String::new(),
                    needed: 1,
                },
                DemandEntry {
                    shift_id: 1,
                    position: ANY_POSITION.to_string(),
                    location: String::new(),
                    needed: 1,
                },
            ],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        // The one cell fills the row of its position; the open row needs nobody
        // else as long as nobody is free.
        let generated = run(&setup, &[day]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].position, ANY_POSITION);
    }

    #[test]
    fn a_house_takes_only_its_own_sub_department() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![
            of_subdepartment(person(1, "Haus", "Meister", false), "Großes Haus"),
            of_subdepartment(person(2, "Podium", "Meister", false), "Podium"),
        ];
        let subdepartments = houses(&[("Großes Haus", "Großes Haus"), ("Podium", "Podium")]);
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-01"), vec![row(1, "Meister", "Podium", 1)]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &subdepartments,
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.cells[0].employee_id, 2);
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn a_department_alone_does_not_open_a_house() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        // The department matches the house, the sub-department does not exist: the
        // generator plans such an employee by hand only.
        let people = vec![of_department(
            person(1, "Podium", "Meister", false),
            "Podium",
        )];
        let subdepartments = houses(&[("Podium", "Podium")]);
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-01"), vec![row(1, "Meister", "Podium", 1)]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &subdepartments,
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].reason, GAP_SUBDEPARTMENT);
    }

    #[test]
    fn a_house_bound_to_a_sub_department_takes_its_people() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        // Two houses of one department: only the sub-department counts.
        let podium = of_subdepartment(
            of_department(person(1, "Podium", "Meister", false), "Beleuchtung"),
            "Podium",
        );
        let house = of_subdepartment(
            of_department(person(2, "Haus", "Meister", false), "Beleuchtung"),
            "Großes Haus",
        );
        let people = vec![podium, house];
        let subdepartments = houses(&[("Podium", "Podium"), ("Großes Haus", "Großes Haus")]);
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-01"), vec![row(1, "Meister", "Podium", 1)]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &subdepartments,
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.cells[0].employee_id, 1);
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn a_house_without_a_sub_department_takes_anyone() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![of_department(
            person(1, "Haus", "Meister", false),
            "Beleuchtung gr. Haus",
        )];
        let subdepartments = houses(&[("Podium", "Podium")]);
        let mut demand = BTreeMap::new();
        demand.insert(
            date("2026-12-01"),
            vec![row(1, "Meister", "Großes Haus", 1)],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &subdepartments,
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert_eq!(generated.cells.len(), 1);
    }

    #[test]
    fn a_house_whose_sub_department_is_empty_stays_open_with_its_reason() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Haus", "Meister", false)];
        let subdepartments = houses(&[("Podium", "Podium")]);
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-01"), vec![row(1, "Meister", "Podium", 1)]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &subdepartments,
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].reason, GAP_SUBDEPARTMENT);
    }

    #[test]
    fn two_houses_of_one_evening_get_two_crews() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(2, "S", "15:00", "23:00", ShiftBand::Late, false)];
        let people = vec![
            of_subdepartment(person(1, "Haus", "Meister", false), "Großes Haus"),
            of_subdepartment(person(2, "Podium", "Meister", false), "Podium"),
        ];
        let subdepartments = houses(&[("Großes Haus", "Großes Haus"), ("Podium", "Podium")]);
        let mut demand = BTreeMap::new();
        demand.insert(
            date("2026-12-04"),
            vec![
                row(2, "Meister", "Großes Haus", 1),
                row(2, "Meister", "Podium", 1),
            ],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &subdepartments,
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-04")]);
        assert_eq!(generated.cells.len(), 2);
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn the_open_row_prefers_a_position_the_band_does_not_carry_yet() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![
            person(1, "Meister eins", "Meister", false),
            person(2, "Meister zwei", "Meister", false),
            person(3, "Stellwerker", "Stellwerker", false),
            person(4, "Beleuchter", "Beleuchter - Brücke", false),
        ];
        let mut demand = BTreeMap::new();
        demand.insert(
            date("2026-12-01"),
            vec![
                row(1, "Meister", "Großes Haus", 1),
                row(1, "Stellwerker", "Großes Haus", 1),
                row(1, ANY_POSITION, "Podium", 1),
            ],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert_eq!(generated.cells.len(), 3);
        // The second Meister would be a second Meister in the same band; the
        // Beleuchter is not in that band yet.
        assert!(generated.cells.iter().any(|cell| cell.employee_id == 4));
        assert!(!generated.cells.iter().any(|cell| cell.employee_id == 2));
    }

    #[test]
    fn a_day_off_starts_the_row_again() {
        let rules = PlanningRules::default();
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1, "Meister", "Meister", false)];
        // Six days in a row are the limit; the seventh breaks it.
        let history: Vec<Cell> = (1..=6)
            .map(|day| cell(1, &format!("2026-12-0{day}"), 1))
            .collect();
        let mut demand = BTreeMap::new();
        let (siebter, rows) = demand_row("2026-12-07", 1, "Meister", 1);
        demand.insert(siebter, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-07")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);

        // With a day off in between the row starts again.
        let mut demand = BTreeMap::new();
        let (achter, rows) = demand_row("2026-12-08", 1, "Meister", 1);
        demand.insert(achter, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-08")]);
        assert_eq!(generated.cells.len(), 1);
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn the_counter_shift_staffs_both_bands_first() {
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
        ];
        let people = vec![
            person(
                1,
                "Oberbeleuchter eins",
                "Oberbeleuchter - Bühne links",
                false,
            ),
            person(
                2,
                "Oberbeleuchter zwei",
                "Oberbeleuchter - Bühne links",
                false,
            ),
        ];
        let mut demand = BTreeMap::new();
        demand.insert(
            date("2026-12-01"),
            vec![
                row(1, "Oberbeleuchter - Bühne links", "Großes Haus", 2),
                row(2, "Oberbeleuchter - Bühne links", "Großes Haus", 1),
            ],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        // Two employees, three seats: one in the early shift, one in the late
        // shift, the second seat of the early shift stays open.
        assert_eq!(generated.cells.len(), 2);
        assert_ne!(generated.cells[0].shift_id, generated.cells[1].shift_id);
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].shift_id, 1);
        assert_eq!(generated.gaps[0].missing, 1);
    }

    #[test]
    fn a_free_day_costs_no_hours_and_no_rest() {
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(3, "FREI", "00:00", "00:00", ShiftBand::Free, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        // A free day before the period is no work: nothing blocks the next day.
        let history = vec![cell(1, "2026-11-30", 3)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert_eq!(generated.cells.len(), 1);
        assert!(generated.gaps.is_empty());
    }

    #[test]
    fn a_free_day_is_no_company_for_an_apprentice() {
        // The colleague with the free day is off, so the apprentice would work
        // alone: no cell, a gap instead.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(3, "FREI", "00:00", "23:59", ShiftBand::Free, false),
        ];
        let people = vec![
            person(1, "Meister", "Meister", false),
            person(2, "Azubi", "Meister", true),
        ];
        let existing = vec![cell(1, "2026-12-01", 3)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);
    }

    #[test]
    fn a_free_day_restarts_the_block_but_keeps_the_rotation() {
        // A free day is not booked: `min`/`max_consecutive_days` count from the
        // next worked day on, while `shift_rotation_days` keeps the band.
        let mut record = Record::default();
        for day in ["01", "02", "03"] {
            record.book(
                date(&format!("2026-12-{day}")),
                450,
                900,
                Some(ShiftBand::Early),
                14,
            );
        }
        assert_eq!(record.streak, 3);
        // The free day on the 04. books nothing, so the rotation still holds.
        assert_eq!(record.bound(date("2026-12-05"), 14), Some(ShiftBand::Early));
        // The next worked day starts a new block.
        record.book(date("2026-12-05"), 450, 900, Some(ShiftBand::Early), 14);
        assert_eq!(record.streak, 1);
    }

    #[test]
    fn a_running_block_holds_the_band_for_the_rotation_days() {
        // Two days of late shift: the employee is bound to that band for the
        // configured window and is free for the other one afterwards.
        let mut record = Record::default();
        record.book(date("2026-12-01"), 450, 1380, Some(ShiftBand::Late), 14);
        record.book(date("2026-12-02"), 450, 1380, Some(ShiftBand::Late), 14);
        assert_eq!(record.bound(date("2026-12-03"), 14), Some(ShiftBand::Late));
        // The window counts from the first day of the block.
        assert_eq!(record.bound(date("2026-12-14"), 14), Some(ShiftBand::Late));
        assert_eq!(record.bound(date("2026-12-15"), 14), None);
        // The next worked day starts the new block with its own window.
        record.book(date("2026-12-15"), 450, 900, Some(ShiftBand::Early), 14);
        assert_eq!(record.bound(date("2026-12-16"), 14), Some(ShiftBand::Early));
        assert_eq!(record.bound(date("2026-12-29"), 14), None);
    }

    #[test]
    fn a_zwischen_shift_does_not_break_the_block() {
        let mut record = Record::default();
        record.book(date("2026-12-01"), 450, 900, Some(ShiftBand::Early), 14);
        record.book(
            date("2026-12-02"),
            300,
            800,
            Some(ShiftBand::Intermediate),
            14,
        );
        assert_eq!(record.bound(date("2026-12-03"), 14), Some(ShiftBand::Early));
    }

    #[test]
    fn a_free_day_lets_the_block_start_again() {
        // Three days before the period, a free day on the first day of it and a
        // seat every day: the free day ends the run of working days, so the
        // employee can work another three days instead of none.
        let rules = PlanningRules {
            max_consecutive_days: 3,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(3, "FREI", "00:00", "23:59", ShiftBand::Free, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        let history = vec![
            cell(1, "2026-11-28", 1),
            cell(1, "2026-11-29", 1),
            cell(1, "2026-11-30", 1),
        ];
        let existing = vec![cell(1, "2026-12-01", 3)];
        let mut demand = BTreeMap::new();
        for day in ["02", "03", "04", "05"] {
            let (day, rows) = demand_row(&format!("2026-12-{day}"), 1, "Meister", 1);
            demand.insert(day, rows);
        }
        let days: Vec<NaiveDate> = (2..=5)
            .map(|day| date(&format!("2026-12-0{day}")))
            .collect();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        let days: Vec<NaiveDate> = generated.cells.iter().map(|cell| cell.day).collect();
        assert_eq!(
            days,
            vec![date("2026-12-02"), date("2026-12-03"), date("2026-12-04")]
        );
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].day, date("2026-12-05"));
    }

    #[test]
    fn a_free_day_keeps_its_employee_and_covers_no_demand() {
        // The free day stays in the plan, its employee is not planned elsewhere
        // that day, and its shift covers no row of the demand.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(3, "FREI", "00:00", "23:59", ShiftBand::Free, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        let existing = vec![cell(1, "2026-12-01", 3)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 3, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.kept, 1);
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].shift_id, 3);
        assert_eq!(generated.gaps[0].missing, 1);
    }

    #[test]
    fn a_short_block_works_before_somebody_rests() {
        // Two employees, one seat a day. With a minimum block of two days the
        // one who started a block is preferred, so nobody rests after one day.
        let rules = PlanningRules {
            min_consecutive_days: 2,
            ..PlanningRules::default()
        };
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![
            person(1, "Anna", "Meister", false),
            person(2, "Bernd", "Meister", false),
        ];
        let mut demand = BTreeMap::new();
        for day in ["01", "02", "03", "04"] {
            let (day, rows) = demand_row(&format!("2026-12-{day}"), 1, "Meister", 1);
            demand.insert(day, rows);
        }
        let days: Vec<NaiveDate> = (1..=4)
            .map(|day| date(&format!("2026-12-0{day}")))
            .collect();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        assert_eq!(generated.cells.len(), 4);
        // Every employee works in a block of at least two days.
        for person in [1, 2] {
            let eigene: Vec<NaiveDate> = generated
                .cells
                .iter()
                .filter(|cell| cell.employee_id == person)
                .map(|cell| cell.day)
                .collect();
            assert!(!eigene.is_empty());
            for paar in eigene.windows(2) {
                assert_eq!(
                    (paar[1] - paar[0]).num_days(),
                    1,
                    "employee {person} has a single day between free days"
                );
            }
        }
    }

    #[test]
    fn the_position_that_the_shift_excludes_is_not_taken() {
        let rules = PlanningRules::default();
        let mut blocks = shift(1, "F", "08:00", "15:30", ShiftBand::Early, false);
        blocks.excluded_positions = vec!["Auszubildender".to_string()];
        let shifts = vec![blocks];
        let people = vec![person(1, "Azubi", "Auszubildender", true)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 1, "Auszubildender", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
    }

    #[test]
    fn a_running_block_keeps_the_employee_out_of_the_other_band() {
        // Anna worked the early shift on the 30. and the block of the rotation
        // still runs: the late shift of the next day stays open instead of
        // switching her back and forth.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "14:00", "21:30", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let history = vec![cell(1, "2026-11-30", 1)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-01", 2, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);
    }

    #[test]
    fn a_block_ends_after_the_rotation_days() {
        // Three weeks after the early shift the block is over: the late shift is
        // the first of the new block.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "14:00", "21:30", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let history = vec![cell(1, "2026-11-30", 1)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-21", 2, "Meister", 1);
        demand.insert(day, rows);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-21")]);
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.cells[0].shift_id, 2);
    }

    #[test]
    fn the_blocks_of_two_employees_staff_the_two_bands() {
        // Anna stayed early, Bernd late: both keep their band, nobody switches.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "14:00", "21:30", ShiftBand::Late, false),
        ];
        let people = vec![
            person(1, "Anna", "Meister", false),
            person(2, "Bernd", "Meister", false),
        ];
        let history = vec![cell(1, "2026-11-30", 1), cell(2, "2026-11-30", 2)];
        let mut demand = BTreeMap::new();
        demand.insert(
            date("2026-12-01"),
            vec![row(1, "Meister", "", 1), row(2, "Meister", "", 1)],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.gaps.is_empty());
        let mut zu: Vec<(i64, i64)> = generated
            .cells
            .iter()
            .map(|cell| (cell.employee_id, cell.shift_id))
            .collect();
        zu.sort();
        assert_eq!(zu, vec![(1, 1), (2, 2)]);
    }

    #[test]
    fn an_extra_shift_stays_in_the_band_of_the_block() {
        // The late shift alone would close the deficit better, but Anna is in an
        // early block: the extra hours come from early shifts.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "14:00", "22:30", ShiftBand::Late, false),
        ];
        let people = vec![with_contract(person(1, "Anna", "Meister", false), 3000, 5)];
        let history = vec![cell(1, "2026-12-04", 1)];
        let demand = BTreeMap::new();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &week());
        assert_eq!(generated.cells.len(), 5);
        assert_eq!(generated.extra, 5);
        assert!(
            generated.cells.iter().all(|cell| cell.shift_id == 1),
            "the extra shift left the block"
        );
    }

    #[test]
    fn an_extra_shift_takes_the_band_of_the_cell_after_it() {
        // The early shift would close the deficit better, but the Wednesday of
        // the week is planned late: the extra shifts of Monday and Tuesday go
        // into the same block instead of forcing a switch.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "16:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "22:30", ShiftBand::Late, false),
        ];
        let people = vec![with_contract(person(1, "Anna", "Meister", false), 3000, 5)];
        let existing = vec![cell(1, "2026-12-09", 2)];
        let demand = BTreeMap::new();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &week());
        assert_eq!(generated.extra, 4);
        assert!(
            generated.cells.iter().all(|cell| cell.shift_id == 2),
            "an extra shift broke the block of the planned cell"
        );
    }

    #[test]
    fn a_monday_after_a_sunday_owes_no_compensation_yet() {
        // Sunday 15:00–23:00 and Monday worked right after: the Monday only keeps the
        // ordinary rest, the compensation day is still owed. The Tuesday free pays it,
        // and the Wednesday schedule is then measured from the Monday.
        let mut record = Record::default();
        record.book(date("2026-12-06"), 480, 23 * 60, None, 14);
        record.book(date("2026-12-07"), 480, 23 * 60, None, 14);
        assert_eq!(record.compensation_end, None);
        record.rest(date("2026-12-08"));
        assert_eq!(
            record.compensation_end,
            Some(absolute(date("2026-12-07"), 23 * 60))
        );
    }

    #[test]
    fn an_extra_shift_keeps_the_rest_to_the_day_after() {
        // Sunday takes only the weekend shift (14:00–22:00), the Monday is planned
        // early already: the ten hours between them are too few, so no extra shift fits
        // and the deficit stays instead of breaking the rest period.
        let rules = PlanningRules {
            sunday_work_allowed: true,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
        ];
        let people = vec![with_contract(
            person(1, "Meister", "Meister", false),
            3000,
            7,
        )];
        let existing = vec![cell(1, "2026-12-07", 1)];
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &existing,
            history: &[],
            holidays: &[],
            demand: &BTreeMap::new(),
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &[date("2026-12-06"), date("2026-12-07")]);
        assert_eq!(generated.extra, 0);
        assert!(generated.cells.is_empty());
    }

    #[test]
    fn a_free_employee_is_left_for_the_other_band() {
        // Anna is in an early block, Bernd is free. The early row goes to Anna
        // although Bernd has the fewer hours, so Bernd can take the late one —
        // with the hours as the first criterion the late row would stay open.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "14:00", "21:30", ShiftBand::Late, false),
        ];
        let people = vec![
            person(1, "Anna", "Meister", false),
            person(2, "Bernd", "Meister", false),
        ];
        let history = vec![cell(1, "2026-11-30", 1)];
        let mut demand = BTreeMap::new();
        demand.insert(
            date("2026-12-01"),
            vec![row(1, "Meister", "", 1), row(2, "Meister", "", 1)],
        );
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert!(generated.gaps.is_empty());
        let mut zu: Vec<(i64, i64)> = generated
            .cells
            .iter()
            .map(|cell| (cell.employee_id, cell.shift_id))
            .collect();
        zu.sort();
        assert_eq!(zu, vec![(1, 1), (2, 2)]);
    }

    #[test]
    fn a_regular_late_shift_is_not_worked_on_the_weekend() {
        // Saturday the 05.12.2026 takes the weekend shift, not the regular one.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
            shift(3, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-05"), vec![row(2, "Meister", "", 1)]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-05")]);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].reason, GAP_NO_EMPLOYEE);

        // The weekend shift of the same band fills the day.
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-05"), vec![row(3, "Meister", "", 1)]);
        let setup = Setup {
            demand: &demand,
            ..setup
        };
        let generated = run(&setup, &[date("2026-12-05")]);
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.cells[0].shift_id, 3);
    }

    #[test]
    fn an_apprentice_works_only_apprentice_shifts() {
        // Two seats of the regular early shift and one apprentice: the apprentice
        // stays out of it, so the second seat stays open.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(6, "F-AZ", "08:00", "15:30", ShiftBand::Early, true),
        ];
        let people = vec![
            person(1, "Meister", "Meister", false),
            person(2, "Azubi", "Meister", true),
        ];
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-01"), vec![row(1, "Meister", "", 2)]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.cells[0].employee_id, 1);
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].missing, 1);

        // Its own shift takes the apprentice, next to the colleague.
        let mut demand = BTreeMap::new();
        demand.insert(
            date("2026-12-01"),
            vec![row(1, "Meister", "", 1), row(6, "Meister", "", 1)],
        );
        let setup = Setup {
            demand: &demand,
            ..setup
        };
        let generated = run(&setup, &[date("2026-12-01")]);
        let mut zu: Vec<(i64, i64)> = generated
            .cells
            .iter()
            .map(|cell| (cell.employee_id, cell.shift_id))
            .collect();
        zu.sort();
        assert_eq!(zu, vec![(1, 1), (2, 6)]);
    }

    #[test]
    fn every_empty_day_becomes_a_free_day() {
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(34, "FT", "00:00", "23:59", ShiftBand::Free, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &BTreeMap::new(),
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run_parts(
            &setup,
            &[date("2026-12-01"), date("2026-12-02")],
            Some(34),
            None,
        );
        assert!(generated.cells.is_empty());
        assert_eq!(generated.free.len(), 2);
        assert!(generated.free.iter().all(|cell| cell.shift_id == 34));
    }

    #[test]
    fn a_worked_sunday_gives_the_day_after_as_compensation() {
        // Sunday work on the 06.12.2026: the Monday is the `SoFT`, a day without a
        // Sunday keeps the plain free day.
        let rules = PlanningRules {
            sunday_work_allowed: true,
            compensate_sunday: true,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(3, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
            shift(34, "FT", "00:00", "23:59", ShiftBand::Free, false),
            shift(67, "SoFT", "00:00", "23:59", ShiftBand::Free, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        let mut demand = BTreeMap::new();
        demand.insert(date("2026-12-06"), vec![row(3, "Meister", "", 1)]);
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run_parts(
            &setup,
            &[date("2026-12-06"), date("2026-12-07"), date("2026-12-08")],
            Some(34),
            Some(67),
        );
        assert_eq!(generated.cells.len(), 1);
        let frei: Vec<(NaiveDate, i64)> = generated
            .free
            .iter()
            .map(|cell| (cell.day, cell.shift_id))
            .collect();
        assert_eq!(
            frei,
            vec![(date("2026-12-07"), 67), (date("2026-12-08"), 34),]
        );
    }

    #[test]
    fn a_free_day_on_a_sunday_or_a_holiday_is_soft() {
        // 06.12.2026 is a Sunday and the 08.12.2026 is taken as a holiday: both get
        // the compensation day, the days around them keep the plain free day.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(34, "FT", "00:00", "23:59", ShiftBand::Free, false),
            shift(67, "SoFT", "00:00", "23:59", ShiftBand::Free, false),
        ];
        let people = vec![person(1, "Meister", "Meister", false)];
        let holidays = vec![date("2026-12-08")];
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &holidays,
            demand: &BTreeMap::new(),
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run_parts(
            &setup,
            &[
                date("2026-12-05"),
                date("2026-12-06"),
                date("2026-12-07"),
                date("2026-12-08"),
            ],
            Some(34),
            Some(67),
        );
        let frei: Vec<(NaiveDate, i64)> = generated
            .free
            .iter()
            .map(|cell| (cell.day, cell.shift_id))
            .collect();
        assert_eq!(
            frei,
            vec![
                (date("2026-12-05"), 34),
                (date("2026-12-06"), 67),
                (date("2026-12-07"), 34),
                (date("2026-12-08"), 67),
            ]
        );
    }

    #[test]
    fn a_block_is_left_shortly_before_it_ends() {
        // One employee, one seat a day. He starts in F on the 01.12.; the block
        // runs `shift_rotation_days` = 6 days, so from the 05.12. on (the last
        // `RELAX_DAYS` days) the plan may move him to S when F is the only other
        // row — the rest period of 11 hours holds from 15:30 to 15:00 the next day.
        let rules = PlanningRules {
            shift_rotation_days: 6,
            min_consecutive_days: 1,
            max_consecutive_days: 6,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let mut demand = BTreeMap::new();
        // 01.-03.12. are Tuesday to Thursday, 04.12. a Friday.
        for day in ["01", "02", "03"] {
            let (day, rows) = demand_row(&format!("2026-12-{day}"), 1, "Meister", 1);
            demand.insert(day, rows);
        }
        for day in ["04", "07"] {
            let (day, rows) = demand_row(&format!("2026-12-{day}"), 2, "Meister", 1);
            demand.insert(day, rows);
        }
        let days: Vec<NaiveDate> = ["01", "02", "03", "04", "07"]
            .iter()
            .map(|day| date(&format!("2026-12-{day}")))
            .collect();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        // The late row of the 04.12. is staffed although the block has not run
        // out yet.
        assert_eq!(generated.cells.len(), 5);
        assert!(generated.gaps.is_empty());
        assert!(generated.cells.iter().any(|cell| cell.shift_id == 2));
    }

    #[test]
    fn a_fresh_block_is_not_left() {
        // The same plan, but the other band asks on the second day of the block:
        // that is no emergency but a break, so the row stays open.
        let rules = PlanningRules {
            shift_rotation_days: 6,
            min_consecutive_days: 1,
            max_consecutive_days: 6,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let mut demand = BTreeMap::new();
        for day in ["01", "02"] {
            let (day, rows) = demand_row(&format!("2026-12-{day}"), 1, "Meister", 1);
            demand.insert(day, rows);
        }
        let (day, rows) = demand_row("2026-12-03", 2, "Meister", 1);
        demand.insert(day, rows);
        let days: Vec<NaiveDate> = ["01", "02", "03"]
            .iter()
            .map(|day| date(&format!("2026-12-{day}")))
            .collect();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        assert_eq!(generated.cells.len(), 2);
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].day, date("2026-12-03"));
    }

    #[test]
    fn a_weekend_shift_leaves_the_block_alone() {
        // Anna is in the early block. On Saturday the houses only have the weekend
        // shift, so she may work it — it is no band of its own, the block stays
        // early and Monday is early again.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
            shift(3, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let mut demand = BTreeMap::new();
        for (day, shift_id) in [("04", 1), ("05", 3), ("07", 1)] {
            let (day, rows) = demand_row(&format!("2026-12-{day}"), shift_id, "Meister", 1);
            demand.insert(day, rows);
        }
        let days: Vec<NaiveDate> = ["04", "05", "07"]
            .iter()
            .map(|day| date(&format!("2026-12-{day}")))
            .collect();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        assert!(generated.gaps.is_empty());
        assert_eq!(generated.cells.len(), 3);
        assert!(
            generated
                .cells
                .iter()
                .any(|cell| cell.day == date("2026-12-05") && cell.shift_id == 3)
        );
    }

    #[test]
    fn an_open_row_is_covered_by_a_zwischenschicht() {
        // Anna worked the late band the day before, so her block is fresh and the
        // early row is out of reach. A Zwischenschicht is not bound to a band and
        // covers more than half of the row, so it takes it — ZS1 would start too
        // early for the rest period, so ZS2 stands in.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
            shift(5, "ZS1", "09:00", "17:00", ShiftBand::Intermediate, false),
            shift(6, "ZS2", "10:00", "18:00", ShiftBand::Intermediate, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-02", 1, "Meister", 1);
        demand.insert(day, rows);
        // 01.12. is a Tuesday, the block runs from there.
        let history = vec![cell(1, "2026-12-01", 2)];
        let days = vec![date("2026-12-02")];
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        assert!(generated.gaps.is_empty());
        assert_eq!(generated.cells.len(), 1);
        assert_eq!(generated.cells[0].shift_id, 6);
    }

    #[test]
    fn a_zwischenschicht_that_ends_before_the_row_is_no_stand_in() {
        // The late row would need a Zwischenschicht ending at 17:00 or 18:00: it
        // covers two or three hours of an eight-hour evening, so the row stays
        // open instead of looking served.
        let rules = PlanningRules::default();
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
            shift(5, "ZS1", "09:00", "17:00", ShiftBand::Intermediate, false),
            shift(6, "ZS2", "10:00", "18:00", ShiftBand::Intermediate, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let mut demand = BTreeMap::new();
        let (day, rows) = demand_row("2026-12-02", 2, "Meister", 1);
        demand.insert(day, rows);
        let history = vec![cell(1, "2026-12-01", 1)];
        let days = vec![date("2026-12-02")];
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &history,
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        assert!(generated.cells.is_empty());
        assert_eq!(generated.gaps.len(), 1);
    }

    #[test]
    fn a_seventh_day_in_a_row_stays_open() {
        // Six days of demand and a Sunday evening after them: the seventh day is
        // over the limit, so the row stays a gap even though the weekend work
        // itself is allowed.
        let rules = PlanningRules {
            sunday_work_allowed: true,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
            shift(3, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1, "Anna", "Meister", false)];
        let mut demand = BTreeMap::new();
        for day in ["02", "03", "04", "05", "06", "07"] {
            let (day, rows) = demand_row(&format!("2026-11-{day}"), 1, "Meister", 1);
            demand.insert(day, rows);
        }
        let (day, rows) = demand_row("2026-11-08", 3, "Meister", 1);
        demand.insert(day, rows);
        let days: Vec<NaiveDate> = (2..=8)
            .map(|day| date(&format!("2026-11-0{day}")))
            .collect();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: None,
        };
        let generated = run(&setup, &days);
        let days: Vec<NaiveDate> = generated.cells.iter().map(|cell| cell.day).collect();
        assert_eq!(
            days,
            (2..=7)
                .map(|day| date(&format!("2026-11-0{day}")))
                .collect::<Vec<_>>()
        );
        assert_eq!(generated.gaps.len(), 1);
        assert_eq!(generated.gaps[0].day, date("2026-11-08"));
    }

    #[test]
    fn an_extra_shift_does_not_bridge_two_runs_into_seven_days() {
        // Demand on Monday to Wednesday and on Sunday; the balance fills the gap in
        // between with extra shifts. Those may not turn the two runs into seven days
        // in a row — nor push the week over its hours.
        let rules = PlanningRules {
            sunday_work_allowed: true,
            compensate_sunday: true,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(3, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
        ];
        let people = vec![with_contract(
            person(1, "Meister", "Meister", false),
            3150,
            6,
        )];
        let mut demand = BTreeMap::new();
        for day in ["02", "03", "04"] {
            let (day, rows) = demand_row(&format!("2026-11-{day}"), 1, "Meister", 1);
            demand.insert(day, rows);
        }
        // 08.11.2026 is a Sunday: the evening asks for the weekend shift.
        let (day, rows) = demand_row("2026-11-08", 3, "Meister", 1);
        demand.insert(day, rows);
        let days: Vec<NaiveDate> = (2..=8)
            .map(|day| date(&format!("2026-11-0{day}")))
            .collect();
        let setup = Setup {
            rules: &rules,
            shifts: &shifts,
            people: &people,
            existing: &[],
            history: &[],
            holidays: &[],
            demand: &demand,
            subdepartments: &HashMap::new(),
            foremanless: &BTreeSet::new(),
            balance: empty_balance(),
        };
        let generated = run(&setup, &days);
        let mut run = 0;
        let mut longest = 0;
        let mut vorher: Option<NaiveDate> = None;
        for cell in &generated.cells {
            run = if vorher == Some(cell.day - chrono::Days::new(1)) {
                run + 1
            } else {
                1
            };
            longest = longest.max(run);
            vorher = Some(cell.day);
        }
        assert!(
            longest <= i32::from(rules.max_consecutive_days),
            "{:?}",
            generated
                .cells
                .iter()
                .map(|cell| cell.day)
                .collect::<Vec<_>>()
        );
        let week = generated
            .cells
            .iter()
            .filter(|cell| cell.day.iso_week() == date("2026-11-02").iso_week())
            .count() as i32;
        assert!(week * 450 <= i32::from(rules.max_weekly_hours) * 60);
    }
}
