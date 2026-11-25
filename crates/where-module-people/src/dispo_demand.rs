//! The staffing demand a day's disposition asks for.
//!
//! The disposition names events with a start time (and often a flag), but no
//! shift. This module turns both into demand: every event of the day contributes
//! the staffing of its flag to the shift that serves it.
//!
//! The shift catalogue is the single source of truth, there is no boundary to
//! maintain. A shift serves an event when it holds its start, keeps
//! [`MIN_AFTER_START`] minutes of it afterwards and begins at least [`MIN_LEAD`]
//! minutes before it. Two of those decide between the candidates: **the band comes
//! first** — a Zwischenschicht is the exception and only takes an event that no
//! regular shift can serve — and then **the start closest to the lead**: a 15:00
//! performance wants a shift from 12:00, and when no band shift holds it, the
//! `ZS2` (10:00) is the nearest one. On a Saturday or a Sunday the shift that
//! works the weekend wins over the regular one; on a working day it is the other
//! way round. Apprentice shifts carry no responsibility (apprentices assist, they
//! never take over a shift) and stay out.
//!
//! When no shift begins early enough — an event before the first shift, a 10:00
//! probe — the one that begins last before it takes the event, the band again
//! first. A shift that has less than [`MIN_AFTER_START`] left hands the event on
//! to the shift that follows it (16:00 belongs to the late shift when the
//! catalogue has no Zwischenschicht left that runs long enough), and an event
//! before the first or after the last shift keeps no shift at all and stays
//! unclear.
//!
//! The demand is kept per **location**, and inside one location and shift the
//! events do not add up but take the **maximum**: the crew of a house works its
//! band, so a second event of the same house in the same band — the same evening,
//! a double performance in the morning — is covered by the same people. Two
//! houses staff separately, so their demands stay two rows.
//!
//! An event without a time, with an unknown flag or without a shift to serve it
//! stays an [`UnclearEvent`] and marks its day with `?`.

use std::collections::BTreeMap;

use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::demand::{DemandEntry, is_foreman};
use crate::dispo::DispoEntry;
use crate::flags::Flag;
use crate::shifts::{Shift, ShiftBand, weekend_marker};

/// Minutes of the shift that have to be left after the event has begun: the crew
/// stays for the rest of the show and its teardown.
pub const MIN_AFTER_START: i32 = 120;

/// Minutes before the event the shift has to begin: the crew builds up, checks the
/// light and is ready when the curtain rises. A shift that begins later than that
/// only serves the event when nothing earlier is left — the shift closest to that
/// mark wins, a 15:00 performance therefore wants the `ZS2` (10:00).
pub const MIN_LEAD: i32 = 180;

/// The event has no start time, so no shift can be derived.
pub const UNCLEAR_NO_TIME: &str = "no_time";
/// The event names a flag that the master data does not know.
pub const UNCLEAR_UNKNOWN_FLAG: &str = "unknown_flag";
/// The flag of the event has no staffing, so it asks for nobody.
pub const UNCLEAR_NO_STAFFING: &str = "no_staffing";
/// No regular shift of the required band exists.
pub const UNCLEAR_NO_SHIFT: &str = "no_shift";

/// An event that adds nothing to the demand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct UnclearEvent {
    pub entry_id: i64,
    pub location_id: i64,
    /// Start as `HH:MM` when the event has one.
    pub starts_at: Option<String>,
    pub title: String,
    /// The flag of the text, when it names one.
    pub flag: Option<String>,
    /// One of [`UNCLEAR_NO_TIME`], [`UNCLEAR_NO_FLAG`], [`UNCLEAR_UNKNOWN_FLAG`],
    /// [`UNCLEAR_NO_SHIFT`].
    pub reason: String,
    /// Set aside by the planner: still listed, but it no longer marks the day.
    pub ignored: bool,
}

/// What the disposition of one day asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct DayProposal {
    pub day: NaiveDate,
    /// Demand rows per shift and position, ordered by shift and position.
    pub entries: Vec<DemandEntry>,
    /// Events that could not be read into a shift.
    pub unclear: Vec<UnclearEvent>,
    /// `true` when the day carries at least one unclear event — the board shows
    /// a `?` for it.
    pub marked: bool,
}

/// Reads `HH:MM` as minutes since midnight.
fn minutes(text: &str) -> Option<i32> {
    let mut parts = text.trim().split(':');
    let hours: i32 = parts.next()?.trim().parse().ok()?;
    let mins: i32 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || !(0..=23).contains(&hours) || !(0..=59).contains(&mins) {
        return None;
    }
    Some(hours * 60 + mins)
}

/// `true` when the shift window contains the time; an end before the start runs
/// past midnight.
fn contains(shift: &Shift, time: i32) -> bool {
    let (Some(start), Some(end)) = (minutes(&shift.starts_at), minutes(&shift.ends_at)) else {
        return false;
    };
    if end > start {
        start <= time && time < end
    } else {
        time >= start || time < end
    }
}

/// How long a shift already runs when the event starts (0 = the event opens it).
fn since_start(shift: &Shift, time: i32) -> i32 {
    let Some(start) = minutes(&shift.starts_at) else {
        return i32::MAX;
    };
    (time - start).rem_euclid(24 * 60)
}

/// Minutes of the shift that are left after the event has begun; a shift running
/// past midnight counts on into the next morning.
fn minutes_left(shift: &Shift, time: i32) -> i32 {
    let (Some(start), Some(end)) = (minutes(&shift.starts_at), minutes(&shift.ends_at)) else {
        return 0;
    };
    let end = if end > start { end } else { end + 24 * 60 };
    end - time
}

/// Whether the shift can carry a demand row at all: an inactive one, an apprentice
/// shift (apprentices assist, they never take over a shift) and a free day never do.
fn usable(shift: &Shift) -> bool {
    shift.active && !shift.apprentice_only && !shift.band.is_free()
}

/// Whether the day is a Saturday or a Sunday.
fn is_weekend(day: NaiveDate) -> bool {
    matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
}

/// `true` when the shift belongs to the other kind of day: a weekend shift on a
/// working day, a regular one on Saturday or Sunday.
fn foreign(shift: &Shift, weekend: bool) -> bool {
    if weekend {
        !shift.works_weekend()
    } else {
        weekend_marker(shift.short_name.as_deref(), &shift.name)
    }
}

/// `true` when the three hours before the event are staffed.
///
/// The shift does not have to carry them alone: when the house already works an
/// earlier band that day, the two crews share the run-up. An early shift ending at
/// 15:30 covers two and a half of the three hours a 16:00 show needs, so the late
/// shift only bridges the rest — the crews overlap around the curtain instead of one
/// of them having to be there three hours early.
fn lead_covered(
    time: i32,
    candidate: &Shift,
    shifts: &[Shift],
    crews: &[(NaiveDate, i64, ShiftBand)],
    day: NaiveDate,
    location_id: i64,
) -> bool {
    let from = time - MIN_LEAD;
    let mut spans: Vec<(i32, i32)> = Vec::new();
    let mut add = |shift: &Shift| {
        let (Some(start), Some(end)) = (minutes(&shift.starts_at), minutes(&shift.ends_at)) else {
            return;
        };
        if end <= start {
            return;
        }
        let start = start.max(from);
        let end = end.min(time);
        if start < end {
            spans.push((start, end));
        }
    };
    add(candidate);
    let weekend = is_weekend(day);
    for shift in shifts {
        if !usable(shift) || foreign(shift, weekend) {
            continue;
        }
        let crewing = crews.iter().any(|(crew_day, location, band)| {
            *crew_day == day && *location == location_id && *band == shift.band
        });
        if crewing {
            add(shift);
        }
    }
    spans.sort_unstable();
    let mut reached = from;
    for (start, end) in spans {
        if start > reached {
            return false;
        }
        reached = reached.max(end);
        if reached >= time {
            return true;
        }
    }
    reached >= time
}

/// The shift that serves an event of that day at that time, in three steps.
///
/// 1. It holds the start, keeps [`MIN_AFTER_START`] minutes of it afterwards and
///    covers [`MIN_LEAD`] minutes before it — alone or together with the crews the
///    house already works that day ([`lead_covered`]). Of those the regular shifts
///    come first — a Zwischenschicht is the exception, never a competitor —, then
///    the one closest to the lead. A lone 15:00 performance therefore lands in the
///    `ZS2` (10:00–18:00): `F` keeps half an hour of it and `S` begins with the
///    curtain.
/// 2. When no shift begins early enough — a 10:00 probe, an event before the first
///    shift — the one that began last before it takes the event, the band first.
/// 3. When nothing keeps two hours of the event (a show shortly before the end of
///    the last shift), the band comes first and then the most rest that is left.
///
/// An event that no shift holds at all keeps none and stays unclear.
fn serving_shift<'a>(
    time: i32,
    day: NaiveDate,
    shifts: &'a [Shift],
    crews: &[(NaiveDate, i64, ShiftBand)],
    location_id: i64,
) -> Option<&'a Shift> {
    let weekend = is_weekend(day);
    let holds = |shift: &&Shift| usable(shift) && contains(shift, time);
    let keeps_rest = |shift: &&Shift| minutes_left(shift, time) >= MIN_AFTER_START;
    let covers_lead = |shift: &&Shift| {
        since_start(shift, time) >= MIN_LEAD
            || lead_covered(time, shift, shifts, crews, day, location_id)
    };
    let key = |shift: &&Shift| {
        (
            shift.band == ShiftBand::Intermediate,
            foreign(shift, weekend),
            (since_start(shift, time) - MIN_LEAD).abs(),
            shift.sort_order,
            shift.id,
        )
    };
    if let Some(shift) = shifts
        .iter()
        .filter(holds)
        .filter(keeps_rest)
        .filter(covers_lead)
        .min_by_key(key)
    {
        return Some(shift);
    }
    if let Some(shift) = shifts
        .iter()
        .filter(holds)
        .filter(keeps_rest)
        .min_by_key(|shift| {
            (
                shift.band == ShiftBand::Intermediate,
                foreign(shift, weekend),
                -since_start(shift, time),
                shift.sort_order,
                shift.id,
            )
        })
    {
        return Some(shift);
    }
    shifts.iter().filter(holds).min_by_key(|shift| {
        (
            shift.band == ShiftBand::Intermediate,
            foreign(shift, weekend),
            -minutes_left(shift, time),
            shift.sort_order,
            shift.id,
        )
    })
}

/// The shift an event is planned in: the serving one, unless that is a
/// Zwischenschicht while the house already works a band that day.
///
/// A house whose disposition carries an early event has an early crew — a 15:15
/// show, a quarter of an hour before `F` ends, is covered by that crew and not by
/// a Zwischenschicht of its own. Only a day on which the house works no band at all
/// (a lone evening performance, a matinee without a flag) keeps the
/// Zwischenschicht. The crew counts per house and only when its shift still holds
/// the start of the event.
fn planned_shift<'a>(
    time: i32,
    day: NaiveDate,
    shifts: &'a [Shift],
    crews: &[(NaiveDate, i64, ShiftBand)],
    location_id: i64,
) -> Option<&'a Shift> {
    let weekend = is_weekend(day);
    // The crew of the house is already there when the event begins: it takes the
    // event, whatever shift the catalogue would have picked for it.
    if let Some(shift) = shifts
        .iter()
        .filter(|candidate| usable(candidate) && candidate.band != ShiftBand::Intermediate)
        .filter(|candidate| contains(candidate, time))
        .filter(|candidate| {
            crews.iter().any(|(crew_day, location, band)| {
                *crew_day == day && *location == location_id && *band == candidate.band
            })
        })
        .min_by_key(|candidate| {
            (
                since_start(candidate, time) < MIN_LEAD,
                foreign(candidate, weekend),
                (since_start(candidate, time) - MIN_LEAD).abs(),
                candidate.sort_order,
                candidate.id,
            )
        })
    {
        return Some(shift);
    }
    serving_shift(time, day, shifts, crews, location_id)
}

/// A house of the disposition as the derivation sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct House {
    pub name: String,
    /// `false` = the house is run without a foreman, so its staffing rows for the
    /// position `Meister` are dropped.
    pub needs_foreman: bool,
}

/// Builds the demand of every day that carries disposition entries.
///
/// `locations` names the houses: two events in two houses need their staff
/// separately, so the rows of one day are kept apart by location. A house that
/// needs no foreman is not asked for one, whatever the flag staffing says.
pub fn derive(
    entries: &[DispoEntry],
    flags: &[Flag],
    shifts: &[Shift],
    locations: &BTreeMap<i64, House>,
) -> Vec<DayProposal> {
    // The band crews of the day: an event that no band shift holds still belongs to
    // the crew its house works — the Zwischenschicht is the exception, not a third
    // crew next to an early and a late one.
    let mut crews: Vec<(NaiveDate, i64, ShiftBand)> = Vec::new();
    for entry in entries {
        let (Some(time), Some(flag)) = (
            entry.starts_at.as_deref().and_then(minutes),
            known_flag(entry, flags),
        ) else {
            continue;
        };
        if entry.ignored || flag.staffing.is_empty() {
            continue;
        }
        if let Some(shift) = serving_shift(time, entry.day, shifts, &[], -1)
            && shift.band != ShiftBand::Intermediate
        {
            let crew = (entry.day, entry.location_id, shift.band);
            if !crews.contains(&crew) {
                crews.push(crew);
            }
        }
    }

    let mut days: BTreeMap<NaiveDate, (BTreeMap<(i64, String, String), i32>, Vec<UnclearEvent>)> =
        BTreeMap::new();
    for entry in entries {
        let (demand, unclear) = days.entry(entry.day).or_default();
        let mut mark = |reason: &str| {
            unclear.push(UnclearEvent {
                entry_id: entry.id,
                location_id: entry.location_id,
                starts_at: entry.starts_at.clone(),
                title: entry.title.clone(),
                flag: entry.flag.clone(),
                reason: reason.to_string(),
                ignored: entry.ignored,
            });
        };
        let Some(time) = entry.starts_at.as_deref().and_then(minutes) else {
            mark(UNCLEAR_NO_TIME);
            continue;
        };
        let code = entry.flag.as_deref().map(str::trim).unwrap_or_default();
        if code.is_empty() {
            // An event without a flag is no error: it simply asks for nobody.
            continue;
        }
        let Some(flag) = known_flag(entry, flags) else {
            mark(UNCLEAR_UNKNOWN_FLAG);
            continue;
        };
        let Some(shift) = planned_shift(time, entry.day, shifts, &crews, entry.location_id) else {
            mark(UNCLEAR_NO_SHIFT);
            continue;
        };
        let shift_id = shift.id;
        // Every house plans its own crew per flag: the rows of this location count,
        // a house without rows for the flag asks for nobody.
        let staffing: Vec<&crate::flags::FlagStaffing> = flag
            .staffing
            .iter()
            .filter(|row| row.location_id == entry.location_id)
            .collect();
        if staffing.is_empty() {
            mark(UNCLEAR_NO_STAFFING);
            continue;
        }
        // An entry the planner set aside is kept, but it asks for nobody.
        if entry.ignored {
            continue;
        }
        let house = locations.get(&entry.location_id);
        let location = house.map(|house| house.name.clone()).unwrap_or_default();
        let needs_foreman = house.map(|house| house.needs_foreman).unwrap_or(true);
        for row in staffing {
            // A bar or a foyer is run without a foreman: the staffing of the flag
            // asks for one, the house does not.
            if !needs_foreman && is_foreman(&row.position) {
                continue;
            }
            // One house and one band mean one crew: a second event of the same
            // house in the same band is covered by the same people, so the
            // demand of a row is the largest one of its events and not the sum.
            let ziel = demand
                .entry((shift_id, row.position.clone(), location.clone()))
                .or_insert(0);
            *ziel = (*ziel).max(row.needed);
        }
    }
    days.into_iter()
        .map(|(day, (demand, unclear))| DayProposal {
            day,
            entries: demand
                .into_iter()
                .filter(|(_, needed)| *needed > 0)
                .map(|((shift_id, position, location), needed)| DemandEntry {
                    shift_id,
                    position,
                    location,
                    needed,
                })
                .collect(),
            marked: unclear.iter().any(|event| !event.ignored),
            unclear,
        })
        .collect()
}

/// The flag an event names, when the master data knows it.
fn known_flag<'a>(entry: &DispoEntry, flags: &'a [Flag]) -> Option<&'a Flag> {
    let code = entry.flag.as_deref().map(str::trim).unwrap_or_default();
    flags
        .iter()
        .find(|flag| flag.code.eq_ignore_ascii_case(code))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flags::FlagStaffing;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    /// The houses the disposition knows.
    fn houses() -> BTreeMap<i64, House> {
        house_map(true)
    }

    /// The same houses, without a foreman.
    fn houses_without_foreman() -> BTreeMap<i64, House> {
        house_map(false)
    }

    fn house_map(needs_foreman: bool) -> BTreeMap<i64, House> {
        BTreeMap::from([
            (
                1,
                House {
                    name: "Großes Haus".to_string(),
                    needs_foreman,
                },
            ),
            (
                2,
                House {
                    name: "Podium".to_string(),
                    needs_foreman,
                },
            ),
        ])
    }

    fn shift(
        id: i64,
        short: &str,
        starts_at: &str,
        ends_at: &str,
        band: ShiftBand,
        azubi: bool,
    ) -> Shift {
        Shift {
            id,
            name: short.to_string(),
            short_name: Some(short.to_string()),
            starts_at: starts_at.to_string(),
            ends_at: ends_at.to_string(),
            break_minutes: 30,
            band,
            apprentice_only: azubi,
            color: None,
            department: None,
            active: true,
            sort_order: id as i32,
            excluded_positions: Vec::new(),
        }
    }

    /// The shifts of the theatre.
    fn catalogue() -> Vec<Shift> {
        vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
            shift(3, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
            shift(4, "S-AZ", "14:30", "23:00", ShiftBand::Late, true),
            shift(5, "ZS1", "09:00", "17:00", ShiftBand::Intermediate, false),
            shift(7, "ZS2", "10:00", "18:00", ShiftBand::Intermediate, false),
            // A free day covers the whole day and still never takes an event.
            shift(6, "FREI", "00:00", "00:00", ShiftBand::Free, false),
        ]
    }

    /// A flag whose rows stand in every house the tests use (1, 2 and 7).
    fn flag(code: &str, rows: &[(&str, i32)]) -> Flag {
        let mut flag = flag_at(1, code, rows);
        flag.staffing.extend(flag_at(2, code, rows).staffing);
        flag.staffing.extend(flag_at(7, code, rows).staffing);
        flag
    }

    fn flag_at(location_id: i64, code: &str, rows: &[(&str, i32)]) -> Flag {
        Flag {
            id: 1,
            code: code.to_string(),
            name: code.to_string(),
            sort_order: 0,
            staffing: rows
                .iter()
                .map(|(position, needed)| FlagStaffing {
                    location_id,
                    position: (*position).to_string(),
                    needed: *needed,
                })
                .collect(),
            excluded_positions: Vec::new(),
        }
    }

    fn entry(id: i64, time: Option<&str>, code: Option<&str>) -> DispoEntry {
        DispoEntry {
            id,
            location_id: 7,
            day: date("2026-12-01"),
            starts_at: time.map(str::to_string),
            title: "Max und Moritz".to_string(),
            flag: code.map(str::to_string),
            sort_order: 0,
            ignored: false,
        }
    }

    #[test]
    fn an_ignored_event_asks_for_nobody_and_marks_no_day() {
        // The text without a time stayed unclear — until the planner set it aside:
        // it is still listed, but the day is no longer marked and nobody is asked.
        let mut weg = entry(1, None, Some("Vst"));
        weg.ignored = true;
        let proposals = derive(
            &[weg],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert!(proposals[0].entries.is_empty());
        assert_eq!(proposals[0].unclear.len(), 1);
        assert!(proposals[0].unclear[0].ignored);
        assert!(!proposals[0].marked);
    }

    #[test]
    fn an_ignored_event_with_a_time_asks_for_nobody() {
        // Even a readable event may be set aside; it then contributes no row.
        let mut weg = entry(1, Some("19:30"), Some("Vst"));
        weg.ignored = true;
        let proposals = derive(
            &[weg],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert!(proposals[0].entries.is_empty());
        assert!(proposals[0].unclear.is_empty());
        assert!(!proposals[0].marked);
    }

    #[test]
    fn an_open_event_keeps_the_day_marked_next_to_an_ignored_one() {
        // One entry is set aside, the other one still has no time: the day keeps
        // its mark, because a conflict is still open.
        let mut weg = entry(1, None, Some("Vst"));
        weg.ignored = true;
        let proposals = derive(
            &[weg, entry(2, None, Some("Vst"))],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert!(proposals[0].entries.is_empty());
        assert_eq!(proposals[0].unclear.len(), 2);
        assert!(proposals[0].marked);
    }

    #[test]
    fn two_houses_of_the_same_evening_stay_two_rows() {
        // One performance in the big house and one in the podium: the same flag,
        // the same shift, but two crews.
        let mut first = entry(1, Some("19:00"), Some("Vst"));
        first.location_id = 1;
        let mut zweite = entry(2, Some("19:30"), Some("Vst"));
        zweite.location_id = 2;
        let proposals = derive(
            &[first, zweite],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].entries.len(), 2);
        assert!(proposals[0].entries.iter().all(|row| row.needed == 1));
        assert_eq!(proposals[0].entries[0].location, "Großes Haus");
        assert_eq!(proposals[0].entries[1].location, "Podium");
    }

    #[test]
    fn two_events_of_one_house_in_one_band_are_one_crew() {
        // A double performance in the morning: 09:00 and 11:00, same house, same
        // band. The crew stays and covers both, so the demand stays one row.
        let mut first = entry(1, Some("09:00"), Some("Vst"));
        first.location_id = 1;
        let mut zweite = entry(2, Some("11:00"), Some("Vst"));
        zweite.location_id = 1;
        let proposals = derive(
            &[first, zweite],
            &[flag("Vst", &[("Meister", 1), ("Stellwerker", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 2);
        assert!(proposals[0].entries.iter().all(|row| row.needed == 1));
        assert!(
            proposals[0]
                .entries
                .iter()
                .all(|row| row.location == "Großes Haus")
        );
    }

    #[test]
    fn a_house_without_rows_for_the_flag_asks_for_nobody() {
        // Every location plans its own crew: the rows of the big house do not
        // count for the podium, whose event is named as unclear instead.
        let mut first = entry(1, Some("19:00"), Some("Vst"));
        first.location_id = 1;
        let mut zweite = entry(2, Some("19:30"), Some("Vst"));
        zweite.location_id = 2;
        let only_grand_hall = flag_at(1, "Vst", &[("Meister", 1)]);
        let proposals = derive(
            &[first, zweite],
            &[only_grand_hall],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 1);
        assert_eq!(proposals[0].entries[0].location, "Großes Haus");
        assert_eq!(proposals[0].unclear.len(), 1);
        assert_eq!(proposals[0].unclear[0].reason, UNCLEAR_NO_STAFFING);
    }

    #[test]
    fn the_larger_staffing_of_two_events_wins() {
        // Two flags of one band in one house: the fuller one decides.
        let mut first = entry(1, Some("19:00"), Some("BEL"));
        first.location_id = 1;
        let mut zweite = entry(2, Some("19:30"), Some("Vst"));
        zweite.location_id = 1;
        let proposals = derive(
            &[first, zweite],
            &[
                flag("BEL", &[("Meister", 2)]),
                flag("Vst", &[("Meister", 1)]),
            ],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 1);
        assert_eq!(proposals[0].entries[0].needed, 2);
    }

    #[test]
    fn two_events_of_one_house_in_two_bands_stay_two_rows() {
        let mut first = entry(1, Some("10:00"), Some("Vst"));
        first.location_id = 1;
        let mut zweite = entry(2, Some("19:00"), Some("Vst"));
        zweite.location_id = 1;
        let proposals = derive(
            &[first, zweite],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 2);
        assert!(proposals[0].entries.iter().all(|row| row.needed == 1));
        assert_ne!(
            proposals[0].entries[0].shift_id,
            proposals[0].entries[1].shift_id
        );
    }

    #[test]
    fn an_early_event_lands_in_the_early_shift() {
        // 10:00 lies in F (08:00-15:30) — the Zwischenschicht that also covers it
        // never takes demand.
        let proposals = derive(
            &[entry(1, Some("10:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].entries.len(), 1);
        assert_eq!(proposals[0].entries[0].shift_id, 1);
        assert_eq!(proposals[0].entries[0].position, "*");
        assert!(!proposals[0].marked);
    }

    #[test]
    fn a_free_day_never_takes_an_event() {
        // A free day covers all 24 hours, so it would match every time — the
        // derivation skips it deliberately.
        let proposals = derive(
            &[entry(1, Some("10:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 1);
        assert_eq!(proposals[0].entries[0].shift_id, 1);
    }

    #[test]
    fn a_late_event_lands_in_the_late_shift() {
        // 19:30 lies in S and in S-WE; both are eight hours, so the order of the
        // catalogue decides and S wins.
        let proposals = derive(
            &[entry(1, Some("19:30"), Some("Vst"))],
            &[flag("Vst", &[("Meister", 1), ("Stellwerker", 2)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 2);
        assert!(proposals[0].entries.iter().all(|row| row.shift_id == 2));
        let stellwerker = proposals[0]
            .entries
            .iter()
            .find(|row| row.position == "Stellwerker")
            .unwrap();
        assert_eq!(stellwerker.needed, 2);
    }

    #[test]
    fn a_performance_asks_for_a_shift_that_begins_three_hours_before_it() {
        // A 15:00 performance wants a shift from 12:00. The catalogue has none:
        // F ends half an hour after the curtain and S begins with it, so the
        // nearest one is ZS2 (10:00).
        let mut entry = entry(1, Some("15:00"), Some("Probe"));
        entry.day = date("2026-12-02");
        let proposals = derive(
            &[entry],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 7);
    }

    #[test]
    fn a_noon_event_stays_with_the_early_shift() {
        // 12:00 wants a shift from 09:00 — the ZS1 begins exactly then, but the
        // early shift keeps four hours of the event and is the band of the house:
        // a Zwischenschicht only stands in when no band shift serves the event.
        let proposals = derive(
            &[entry(1, Some("12:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 1);
    }

    #[test]
    fn a_saturday_evening_asks_for_the_weekend_shift() {
        // The regular late shift may not be worked at the weekend, so the
        // disposition of a Saturday asks for `S-WE` and the generator can fill it.
        let mut entry = entry(1, Some("19:30"), Some("Vst"));
        entry.day = date("2026-12-05");
        let proposals = derive(
            &[entry],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 3);
    }

    #[test]
    fn a_weekday_evening_keeps_the_regular_late_shift() {
        let mut entry = entry(1, Some("19:30"), Some("Vst"));
        entry.day = date("2026-12-02");
        let proposals = derive(
            &[entry],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 2);
    }

    #[test]
    fn a_sunday_without_a_weekend_shift_takes_the_regular_one() {
        // Only the regular late shift in the catalogue: it is the only one that
        // holds the evening, so the Sunday asks for it instead of leaving the
        // event unplanned.
        let mut entry = entry(1, Some("19:30"), Some("Vst"));
        entry.day = date("2026-12-06");
        let mut shifts = catalogue();
        shifts.retain(|shift| shift.id != 3);
        let proposals = derive(
            &[entry],
            &[flag("Vst", &[("Meister", 1)])],
            &shifts,
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 2);
        assert!(!proposals[0].marked);
    }

    #[test]
    fn a_sunday_performance_takes_the_zwischenschicht_over_the_weekend_shift() {
        // A 15:00 show on a Sunday: the weekend shift begins an hour before the
        // curtain, so it cannot take the crew there in time. The ZS2 (10:00) is
        // the only shift that begins early enough and keeps two hours.
        let mut entry = entry(1, Some("15:00"), Some("Vst"));
        entry.day = date("2026-12-06");
        let proposals = derive(
            &[entry],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 7);
    }

    #[test]
    fn an_event_shortly_before_the_end_of_the_shift_goes_to_the_next_one() {
        // 16:00: F has too little left, S begins 60 minutes before the curtain and
        // no Zwischenschicht runs long enough — the row belongs to the shift that
        // follows F.
        let mut shifts = catalogue();
        shifts.retain(|shift| shift.band != ShiftBand::Intermediate);
        let proposals = derive(
            &[entry(1, Some("16:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &shifts,
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 2);
    }

    #[test]
    fn a_midday_event_takes_the_zwischenschicht_closest_to_the_lead() {
        // 14:30 wants a shift from 11:30; F has an hour too little and S begins
        // after the curtain, so the nearest start is the ZS2 at 10:00.
        let proposals = derive(
            &[entry(1, Some("14:30"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 7);
    }

    #[test]
    fn a_morning_event_still_takes_the_early_shift() {
        // 11:00 wants a shift from 08:00 — the early shift begins exactly then.
        let proposals = derive(
            &[entry(1, Some("11:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 1);
    }

    #[test]
    fn a_zwischenschicht_stands_in_when_no_band_shift_serves_the_event() {
        // 13:45 leaves too little of F, and the catalogue has no late shift: the
        // Zwischenschicht closest to the lead (the ZS2 at 10:00) stands in.
        let mut shifts = catalogue();
        shifts.retain(|shift| shift.id != 2);
        let proposals = derive(
            &[entry(1, Some("13:45"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &shifts,
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 7);
    }

    #[test]
    fn an_evening_shortly_before_the_end_stays_with_the_late_shift() {
        // 22:45 leaves a quarter of an hour of S and nothing follows. The late
        // shift still took the crew there in time, so the night belongs to it
        // instead of becoming an open point.
        let proposals = derive(
            &[entry(1, Some("22:45"), Some("Vst"))],
            &[flag("Vst", &[("Meister", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 2);
        assert!(!proposals[0].marked);
    }

    #[test]
    fn an_event_after_the_early_shift_takes_the_zwischenschicht() {
        // 15:45 is outside F, and S would begin after the curtain: the ZS2 keeps
        // two hours of the show and begins early enough.
        let proposals = derive(
            &[entry(1, Some("15:45"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 7);
    }

    #[test]
    fn a_midday_show_joins_the_early_crew_of_its_house() {
        // An early performance makes the house work F that day. The 15:15 show
        // comes a quarter of an hour before F ends, so it belongs to that crew and
        // not to a Zwischenschicht of its own.
        let mut early = entry(1, Some("12:00"), Some("Vst"));
        early.location_id = 1;
        let mut late = entry(2, Some("15:15"), Some("BEL"));
        late.location_id = 1;
        let proposals = derive(
            &[early, late],
            &[
                flag("Vst", &[("Meister", 1)]),
                flag("BEL", &[("Oberbeleuchter", 1)]),
            ],
            &catalogue(),
            &houses(),
        );
        assert!(
            proposals[0].entries.iter().all(|row| row.shift_id == 1),
            "{:?}",
            proposals[0].entries
        );
        assert!(!proposals[0].marked);
    }

    #[test]
    fn two_crews_share_the_run_up_of_a_late_show() {
        // The early crew ends at 15:30 and covers two and a half of the three hours
        // a 16:00 show needs; the late shift bridges the rest and takes the event,
        // so neither a Zwischenschicht nor a three-hour lead of the late shift is
        // needed.
        let mut early = entry(1, Some("12:00"), Some("Vst"));
        early.location_id = 1;
        let mut late = entry(2, Some("16:00"), Some("WE"));
        late.location_id = 1;
        let proposals = derive(
            &[early, late],
            &[
                flag("Vst", &[("Meister", 1)]),
                flag("WE", &[("Stellwerker", 1)]),
            ],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 2);
        let late_row = proposals[0]
            .entries
            .iter()
            .find(|row| row.position == "Stellwerker")
            .unwrap();
        assert_eq!(late_row.shift_id, 2);
        assert!(!proposals[0].marked);
    }

    #[test]
    fn a_lone_show_after_the_early_shift_keeps_its_lead() {
        // Without an early crew that day nothing covers the run-up: the 16:00 show
        // goes to the late shift only when no Zwischenschicht is left (tier two of
        // the catalogue order), and the lead stays what it always was.
        let mut shifts = catalogue();
        shifts.retain(|shift| shift.band != ShiftBand::Intermediate);
        let proposals = derive(
            &[entry(1, Some("16:00"), Some("WE"))],
            &[flag("WE", &[("Stellwerker", 1)])],
            &shifts,
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 2);
    }

    #[test]
    fn a_lone_midday_show_keeps_the_zwischenschicht() {
        // No early crew in that house: the 15:15 show is the only event of the day,
        // and F keeps a quarter of an hour of it — a Zwischenschicht has to stand
        // in, otherwise nobody is there when the curtain rises.
        let proposals = derive(
            &[entry(1, Some("15:15"), Some("BEL"))],
            &[flag("BEL", &[("Oberbeleuchter", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 7);
    }

    #[test]
    fn a_shift_running_past_midnight_contains_the_late_evening() {
        let mut shifts = catalogue();
        shifts.push(shift(6, "N", "22:00", "06:00", ShiftBand::Late, false));
        let proposals = derive(
            &[entry(1, Some("23:30"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &shifts,
            &houses(),
        );
        // 23:30 lies in S (until 23:00? no — exactly 23:00 ends S) and in N.
        assert_eq!(proposals[0].entries[0].shift_id, 6);
    }

    #[test]
    fn an_event_outside_every_shift_marks_the_day() {
        let proposals = derive(
            &[entry(1, Some("06:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert!(proposals[0].entries.is_empty());
        assert_eq!(proposals[0].unclear[0].reason, UNCLEAR_NO_SHIFT);
        assert!(proposals[0].marked);
    }

    #[test]
    fn a_house_without_a_foreman_asks_for_no_meister() {
        let mut first = entry(1, Some("19:00"), Some("Vst"));
        first.location_id = 1;
        let proposals = derive(
            &[first],
            &[flag("Vst", &[("Meister", 1), ("Stellwerker", 1)])],
            &catalogue(),
            &houses_without_foreman(),
        );
        assert_eq!(proposals[0].entries.len(), 1);
        assert_eq!(proposals[0].entries[0].position, "Stellwerker");
    }

    #[test]
    fn two_events_of_one_shift_are_one_crew() {
        // Both entries carry the same (here unbound) location, so one crew.
        let proposals = derive(
            &[
                entry(1, Some("10:00"), Some("Probe")),
                entry(2, Some("11:00"), Some("Probe")),
            ],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert_eq!(proposals[0].entries.len(), 1);
        assert_eq!(proposals[0].entries[0].needed, 1);
    }

    #[test]
    fn events_without_time_or_with_a_broken_flag_mark_the_day() {
        let proposals = derive(
            &[
                entry(1, None, Some("Probe")),
                entry(2, Some("10:00"), Some("NJK")),
                entry(3, Some("10:00"), None),
            ],
            &[flag("Probe", &[("*", 1)])],
            &catalogue(),
            &houses(),
        );
        assert!(proposals[0].marked);
        // The event without a flag is no error — it asks for nobody.
        assert!(proposals[0].entries.is_empty());
        let reasons: Vec<&str> = proposals[0]
            .unclear
            .iter()
            .map(|event| event.reason.as_str())
            .collect();
        assert_eq!(reasons, vec![UNCLEAR_NO_TIME, UNCLEAR_UNKNOWN_FLAG]);
    }

    #[test]
    fn a_flag_without_staffing_marks_the_day() {
        let proposals = derive(
            &[entry(1, Some("10:00"), Some("Probe"))],
            &[flag("Probe", &[])],
            &catalogue(),
            &houses(),
        );
        assert!(proposals[0].entries.is_empty());
        assert_eq!(proposals[0].unclear[0].reason, UNCLEAR_NO_STAFFING);
    }

    #[test]
    fn without_a_band_shift_the_zwischenschicht_takes_the_event() {
        let mut shifts = catalogue();
        for shift in shifts.iter_mut() {
            if shift.id == 1 {
                shift.apprentice_only = true;
            }
        }
        // 10:00 lies in F (now apprentice only) and in the ZS1: the Zwischenschicht
        // stands in for the event.
        let proposals = derive(
            &[entry(1, Some("10:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &shifts,
            &houses(),
        );
        assert_eq!(proposals[0].entries[0].shift_id, 5);
    }

    #[test]
    fn without_any_shift_the_day_stays_unclear() {
        let mut shifts = catalogue();
        for shift in shifts.iter_mut() {
            shift.active = false;
        }
        let proposals = derive(
            &[entry(1, Some("10:00"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &shifts,
            &houses(),
        );
        assert!(proposals[0].entries.is_empty());
        assert_eq!(proposals[0].unclear[0].reason, UNCLEAR_NO_SHIFT);
    }

    #[test]
    fn an_inactive_shift_takes_no_demand() {
        let mut shifts = catalogue();
        for shift in shifts.iter_mut() {
            if shift.id == 2 {
                shift.active = false;
            }
        }
        let proposals = derive(
            &[entry(1, Some("19:30"), Some("Probe"))],
            &[flag("Probe", &[("*", 1)])],
            &shifts,
            &houses(),
        );
        // S is gone, so the weekend late shift covers the evening.
        assert_eq!(proposals[0].entries[0].shift_id, 3);
    }
}
