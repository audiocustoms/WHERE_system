//! The planning rules a series of duty cells has to keep.
//!
//! The board writes a chosen shift onto several days in a row. Before anything
//! is stored, the days are checked against `PlanningRules`: Sunday work, a
//! public holiday without `holiday_work_allowed`, an existing absence, an
//! apprentice left alone in the shift, the length of the shift, days in a row,
//! the rest period to the neighbouring days, the compensation day a worked Sunday
//! asks for (`extended_sunday_rest`), the weekly hours and the days off per week.
//! The check stops at the first day that breaks a rule, so the series stays
//! contiguous. `free_weekend_every_six_weeks` is stored for the plan proposals;
//! it spans weeks and is not part of the series check.

use std::collections::{HashMap, HashSet};

use chrono::{Datelike, Days, NaiveDate, Weekday};

use crate::planning::PlanningRules;

/// Work on that weekday is not allowed (`sunday_work_allowed`).
pub const BLOCK_SUNDAY: &str = "sunday";
/// The day already holds sick or vacation and `respect_absence` is set.
pub const BLOCK_ABSENCE: &str = "absence";
/// An apprentice would be alone in the shift (`apprentice_needs_company`).
pub const BLOCK_APPRENTICE_ALONE: &str = "apprentice_alone";
/// The day is a public holiday and `holiday_work_allowed` is off.
pub const BLOCK_HOLIDAY: &str = "holiday";
/// `max_consecutive_days` would be exceeded.
pub const BLOCK_CONSECUTIVE: &str = "max_consecutive_days";
/// The rest period to a neighbouring day is shorter than `min_rest_hours`.
pub const BLOCK_REST: &str = "min_rest_hours";
/// The compensation day a worked Sunday asks for is not granted
/// (`extended_sunday_rest`): 24 h free plus the ordinary rest to the next duty.
pub const BLOCK_COMPENSATION: &str = "compensation_rest";
/// The shift is longer than `max_daily_hours`.
pub const BLOCK_DAILY_HOURS: &str = "max_daily_hours";
/// `max_weekly_hours` would be exceeded.
pub const BLOCK_WEEKLY_HOURS: &str = "max_weekly_hours";
/// `min_days_off_per_week` would be exceeded.
pub const BLOCK_DAYS_OFF: &str = "min_days_off_per_week";
/// The employee's position is excluded from that shift.
pub const BLOCK_EXCLUDED_POSITION: &str = "excluded_position";
/// The shift is a regular late shift and the day is a Saturday or a Sunday.
pub const BLOCK_WEEKEND: &str = "weekend";
/// The shift is an apprentice shift and the employee is no apprentice.
pub const BLOCK_APPRENTICE_ONLY: &str = "apprentice_only";

const MINUTES_PER_DAY: i64 = 24 * 60;

/// A shift as the check needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanShift {
    /// Start as minutes after midnight of its day.
    pub start: i64,
    /// End as minutes after midnight of its day; smaller than `start` when the
    /// shift runs past midnight.
    pub end: i64,
    /// Worked minutes, the break taken off.
    pub net: i64,
    /// Whether the shift may be worked on Saturday and Sunday. The regular late
    /// shift may not, the weekend shift (`S-WE`) may.
    pub weekend: bool,
}

impl PlanShift {
    /// Builds the shift from its two clocks and its break.
    pub fn new(starts_at: &str, ends_at: &str, break_minutes: i16) -> Option<Self> {
        let start = minutes(starts_at)?;
        let end = minutes(ends_at)?;
        if start == end {
            return None;
        }
        let span = if end > start {
            end - start
        } else {
            MINUTES_PER_DAY - start + end
        };
        Some(Self {
            start,
            end,
            net: span - i64::from(break_minutes),
            weekend: true,
        })
    }

    /// Keeps the shift out of Saturday and Sunday (`S` without the `WE` marker).
    pub fn without_weekend(mut self) -> Self {
        self.weekend = false;
        self
    }

    /// End of the shift as minutes after midnight of its own day.
    fn end_of_day(&self) -> i64 {
        if self.end > self.start {
            self.end
        } else {
            MINUTES_PER_DAY + self.end
        }
    }
}

fn minutes(text: &str) -> Option<i64> {
    let mut parts = text.trim().split(':');
    let hours: i64 = parts.next()?.trim().parse().ok()?;
    let mins: i64 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || !(0..24).contains(&hours) || !(0..60).contains(&mins) {
        return None;
    }
    Some(hours * 60 + mins)
}

/// The days already planned for the employee; `Some` is a shift, `None` sick or
/// vacation, and a day that is missing is free.
pub type PlannedDays = HashMap<NaiveDate, Option<PlanShift>>;

/// What the check allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesPlan {
    /// Days that may be written, counted from the first day.
    pub allowed: i64,
    /// First day that stays free and the rule that stopped the series.
    pub blocked: Option<(NaiveDate, &'static str)>,
}

/// Checks `days` days in a row from `first` for `shift` and returns how many of
/// them the rules allow.
pub fn plan_series(
    rules: &PlanningRules,
    shift: PlanShift,
    first: NaiveDate,
    days: i64,
    planned: &PlannedDays,
) -> SeriesPlan {
    plan_series_with(
        rules,
        shift,
        first,
        days,
        planned,
        &Company::default(),
        &HashSet::new(),
    )
}

/// What the check needs to know about the other employees of the shift.
#[derive(Debug, Clone, Default)]
pub struct Company {
    /// The employee is an apprentice.
    pub apprentice: bool,
    /// Days on which the same shift already holds a colleague who is no
    /// apprentice — position, department and location do not matter.
    pub days: HashSet<NaiveDate>,
}

impl Company {
    /// Another employee works the same shift on `day`.
    pub fn has(&self, day: NaiveDate) -> bool {
        self.days.contains(&day)
    }

    /// Whether writing the cell on `day` would leave an apprentice alone.
    pub fn leaves_apprentice_alone(&self, rules: &PlanningRules, day: NaiveDate) -> bool {
        rules.apprentice_needs_company && self.apprentice && !self.has(day)
    }
}

/// Like [`plan_series`], but with the other employees of the shift and the
/// public holidays of the period in view.
pub fn plan_series_with(
    rules: &PlanningRules,
    shift: PlanShift,
    first: NaiveDate,
    days: i64,
    planned: &PlannedDays,
    company: &Company,
    holidays: &HashSet<NaiveDate>,
) -> SeriesPlan {
    // The days the series already claims count as planned for the days after them.
    let mut current = planned.clone();
    let mut allowed = 0;
    for offset in 0..days.max(0) {
        let Some(day) = first.checked_add_days(Days::new(offset as u64)) else {
            break;
        };
        if let Some(reason) = block_reason(rules, shift, day, &current, company, holidays) {
            return SeriesPlan {
                allowed,
                blocked: Some((day, reason)),
            };
        }
        current.insert(day, Some(shift));
        allowed += 1;
    }
    SeriesPlan {
        allowed,
        blocked: None,
    }
}

/// The first rule `day` breaks, if any.
pub fn block_reason(
    rules: &PlanningRules,
    shift: PlanShift,
    day: NaiveDate,
    planned: &PlannedDays,
    company: &Company,
    holidays: &HashSet<NaiveDate>,
) -> Option<&'static str> {
    if day.weekday() == Weekday::Sun && !rules.sunday_work_allowed {
        return Some(BLOCK_SUNDAY);
    }
    if matches!(day.weekday(), Weekday::Sat | Weekday::Sun) && !shift.weekend {
        return Some(BLOCK_WEEKEND);
    }
    if !rules.holiday_work_allowed && holidays.contains(&day) {
        return Some(BLOCK_HOLIDAY);
    }
    if rules.respect_absence && matches!(planned.get(&day), Some(None)) {
        return Some(BLOCK_ABSENCE);
    }
    if company.leaves_apprentice_alone(rules, day) {
        return Some(BLOCK_APPRENTICE_ALONE);
    }
    if shift.net > i64::from(rules.max_daily_hours) * 60 {
        return Some(BLOCK_DAILY_HOURS);
    }
    if run_ending_at(day, planned) + 1 > i64::from(rules.max_consecutive_days) {
        return Some(BLOCK_CONSECUTIVE);
    }
    if !rest_kept(rules, shift, day, planned) {
        return Some(BLOCK_REST);
    }
    if !compensation_kept(rules, shift, day, planned) {
        return Some(BLOCK_COMPENSATION);
    }
    if week_minutes(day, planned) + shift.net > i64::from(rules.max_weekly_hours) * 60 {
        return Some(BLOCK_WEEKLY_HOURS);
    }
    if days_off_in_week(day, planned) < i64::from(rules.min_days_off_per_week) {
        return Some(BLOCK_DAYS_OFF);
    }
    None
}

/// Shift days directly before `day`, not counting `day` itself.
fn run_ending_at(day: NaiveDate, planned: &PlannedDays) -> i64 {
    let mut run = 0;
    let mut cursor = day;
    while let Some(before) = cursor.checked_sub_days(Days::new(1)) {
        if !matches!(planned.get(&before), Some(Some(_))) {
            break;
        }
        run += 1;
        cursor = before;
    }
    run
}

/// Both neighbours keep the rest period: the shift of the day before ends early
/// enough and the shift of the day after starts late enough. It is always the
/// ordinary rest of the settings — the extra day after a Sunday belongs to the
/// compensation day, see [`compensation_kept`].
fn rest_kept(
    rules: &PlanningRules,
    shift: PlanShift,
    day: NaiveDate,
    planned: &PlannedDays,
) -> bool {
    let rest = i64::from(rules.min_rest_hours) * 60;
    for back in 1..=2u64 {
        let Some(before) = day.checked_sub_days(Days::new(back)) else {
            break;
        };
        let Some(Some(other)) = planned.get(&before) else {
            continue;
        };
        let end = other.end_of_day() - back as i64 * MINUTES_PER_DAY;
        if shift.start - end < rest {
            return false;
        }
    }
    for ahead in 1..=2u64 {
        let Some(after) = day.checked_add_days(Days::new(ahead)) else {
            break;
        };
        let Some(Some(other)) = planned.get(&after) else {
            continue;
        };
        let start = other.start + ahead as i64 * MINUTES_PER_DAY;
        if start - shift.end_of_day() < rest {
            return false;
        }
    }
    true
}

/// End of the shift on `day` in minutes counted from a fixed origin, so that two
/// days can be compared; `None` when the day carries no work.
fn end_of(planned: &PlannedDays, day: NaiveDate) -> Option<i64> {
    let shift = (*planned.get(&day)?)?;
    Some(day.num_days_from_ce() as i64 * MINUTES_PER_DAY + shift.end_of_day())
}

/// After a Sunday the calendar owes a compensation day (§ 11 Abs. 3 ArbZG): one
/// whole day without work, and the duty after it may only start once the ordinary
/// rest period has passed as well — 24 h plus `min_rest_hours`
/// (`extended_sunday_rest`).
///
/// The days **before** that compensation day keep the ordinary rest: a Monday
/// after Sunday work is allowed when the 11 hours hold, and the compensation may
/// fall on any later day of the week. Only the first duty after the compensation
/// day is measured, because that day is the rest the law asks for.
fn compensation_kept(
    rules: &PlanningRules,
    shift: PlanShift,
    day: NaiveDate,
    planned: &PlannedDays,
) -> bool {
    if !rules.extended_sunday_rest {
        return true;
    }
    let needed = i64::from(rules.min_rest_hours) * 60 + MINUTES_PER_DAY;
    let start = day.num_days_from_ce() as i64 * MINUTES_PER_DAY + shift.start;
    // Every worked Sunday within the week before this day owes a compensation day.
    for back in 1..=7u64 {
        let Some(sunday) = day.checked_sub_days(Days::new(back)) else {
            break;
        };
        if sunday.weekday() != Weekday::Sun || end_of(planned, sunday).is_none() {
            continue;
        }
        // The compensation day is the first day after the Sunday without work.
        let mut compensation = None;
        for offset in 1..back {
            let Some(candidate) = sunday.checked_add_days(Days::new(offset)) else {
                break;
            };
            if end_of(planned, candidate).is_none() {
                compensation = Some(candidate);
                break;
            }
        }
        // No free day in between: the compensation is still owed, and this day is one
        // of those that only keep the ordinary rest.
        let Some(free) = compensation else {
            continue;
        };
        // The duty the compensation day follows.
        let mut before = None;
        for offset in 1..=8u64 {
            let Some(previous) = free.checked_sub_days(Days::new(offset)) else {
                break;
            };
            if let Some(end) = end_of(planned, previous) {
                before = Some(end);
                break;
            }
        }
        let Some(before_end) = before else {
            continue;
        };
        // Only the first duty after the compensation day is measured.
        let mut served = false;
        let mut cursor = free;
        while let Some(candidate) = cursor.checked_add_days(Days::new(1)) {
            if candidate >= day {
                break;
            }
            if end_of(planned, candidate).is_some() {
                served = true;
                break;
            }
            cursor = candidate;
        }
        if served {
            continue;
        }
        if start - before_end < needed {
            return false;
        }
    }
    true
}

/// Worked minutes already planned in the calendar week of `day`.
fn week_minutes(day: NaiveDate, planned: &PlannedDays) -> i64 {
    let week = day.iso_week();
    planned
        .iter()
        .filter(|(other, _)| other.iso_week() == week)
        .filter_map(|(_, shift)| shift.map(|shift| shift.net))
        .sum()
}

/// Days off left in the calendar week of `day` when `day` is worked.
fn days_off_in_week(day: NaiveDate, planned: &PlannedDays) -> i64 {
    let week = day.iso_week();
    let worked = planned
        .iter()
        .filter(|(other, _)| other.iso_week() == week)
        .filter(|(_, shift)| shift.is_some())
        .count() as i64;
    7 - worked - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    /// `08:00–15:30` with 60 minutes break: 6.5 hours.
    fn early() -> PlanShift {
        PlanShift::new("08:00", "15:30", 60).unwrap()
    }

    fn rules() -> PlanningRules {
        PlanningRules {
            // Keep the fixtures short: no Sunday, six days in a row, 11 hours rest.
            sunday_work_allowed: false,
            max_consecutive_days: 6,
            min_rest_hours: 11,
            min_days_off_per_week: 1,
            ..PlanningRules::default()
        }
    }

    fn plan(days: &[(&str, Option<PlanShift>)]) -> PlannedDays {
        days.iter()
            .map(|(day, shift)| (date(day), *shift))
            .collect()
    }

    #[test]
    fn a_free_week_takes_the_whole_series() {
        // Monday to Friday: five days, nothing planned yet.
        let result = plan_series(&rules(), early(), date("2026-11-16"), 5, &plan(&[]));
        assert_eq!(result.allowed, 5);
        assert_eq!(result.blocked, None);
    }

    #[test]
    fn the_series_stops_before_sunday() {
        // Thursday, five days asked, Sunday is not allowed.
        let result = plan_series(&rules(), early(), date("2026-11-19"), 5, &plan(&[]));
        assert_eq!(result.allowed, 3);
        assert_eq!(result.blocked, Some((date("2026-11-22"), BLOCK_SUNDAY)));
    }

    #[test]
    fn the_days_before_count_for_the_consecutive_limit() {
        // Monday to Wednesday already planned, the series starts on Thursday:
        // six days in a row are the limit, so only three more fit.
        let planned = plan(&[
            ("2026-11-16", Some(early())),
            ("2026-11-17", Some(early())),
            ("2026-11-18", Some(early())),
        ]);
        let result = plan_series(&rules(), early(), date("2026-11-19"), 5, &planned);
        assert_eq!(result.allowed, 3);
        assert_eq!(
            result.blocked,
            Some((date("2026-11-22"), BLOCK_SUNDAY)),
            "Sunday wins over the run of six"
        );

        let mut rules = rules();
        rules.sunday_work_allowed = true;
        let result = plan_series(&rules, early(), date("2026-11-19"), 5, &planned);
        assert_eq!(result.allowed, 3);
        assert_eq!(
            result.blocked,
            Some((date("2026-11-22"), BLOCK_CONSECUTIVE))
        );
    }

    #[test]
    fn an_existing_absence_stays() {
        let planned = plan(&[("2026-11-17", None)]);
        let result = plan_series(&rules(), early(), date("2026-11-16"), 3, &planned);
        assert_eq!(result.allowed, 1);
        assert_eq!(result.blocked, Some((date("2026-11-17"), BLOCK_ABSENCE)));

        let mut rules = rules();
        rules.respect_absence = false;
        let result = plan_series(&rules, early(), date("2026-11-16"), 3, &planned);
        assert_eq!(result.allowed, 3);
    }

    #[test]
    fn too_little_rest_to_the_day_before_stops_the_series() {
        // Late shift 22:00–06:00 on Sunday, early shift 08:00–15:30 on Monday:
        // two hours of rest are too few.
        let late = PlanShift::new("22:00", "06:00", 60).unwrap();
        let mut rules = rules();
        rules.sunday_work_allowed = true;
        let result = plan_series(
            &rules,
            early(),
            date("2026-11-16"),
            2,
            &plan(&[("2026-11-15", Some(late))]),
        );
        assert_eq!(result.allowed, 0);
        assert_eq!(result.blocked, Some((date("2026-11-16"), BLOCK_REST)));
    }

    #[test]
    fn an_apprentice_is_not_left_alone_in_a_shift() {
        // The employee is an apprentice, a colleague works Monday and Tuesday.
        let company = Company {
            apprentice: true,
            days: HashSet::from([date("2026-11-16"), date("2026-11-17")]),
        };
        let result = plan_series_with(
            &rules(),
            early(),
            date("2026-11-16"),
            5,
            &plan(&[]),
            &company,
            &HashSet::new(),
        );
        assert_eq!(result.allowed, 2);
        assert_eq!(
            result.blocked,
            Some((date("2026-11-18"), BLOCK_APPRENTICE_ALONE))
        );
        // The colleague of the day before does not carry over to the next day.
        assert!(!company.has(date("2026-11-18")));
    }

    #[test]
    fn a_colleague_in_the_shift_clears_the_apprentice_rule() {
        let company = Company {
            apprentice: true,
            days: HashSet::from([date("2026-11-16")]),
        };
        let result = plan_series_with(
            &rules(),
            early(),
            date("2026-11-16"),
            1,
            &plan(&[]),
            &company,
            &HashSet::new(),
        );
        assert_eq!(result.allowed, 1);
        assert_eq!(result.blocked, None);
    }

    #[test]
    fn the_apprentice_rule_only_counts_for_apprentices_and_can_be_switched_off() {
        let alone = Company {
            apprentice: false,
            days: HashSet::new(),
        };
        let result = plan_series_with(
            &rules(),
            early(),
            date("2026-11-16"),
            3,
            &plan(&[]),
            &alone,
            &HashSet::new(),
        );
        assert_eq!(result.allowed, 3);

        let mut rules = rules();
        rules.apprentice_needs_company = false;
        let apprentice = Company {
            apprentice: true,
            days: HashSet::new(),
        };
        let result = plan_series_with(
            &rules,
            early(),
            date("2026-11-16"),
            3,
            &plan(&[]),
            &apprentice,
            &HashSet::new(),
        );
        assert_eq!(result.allowed, 3);
    }

    #[test]
    fn a_public_holiday_is_only_planned_when_it_is_allowed() {
        // 24.12.2026 (Thursday), Christmas Day, 26.12. (Saturday).
        let holidays = HashSet::from([date("2026-12-25")]);
        let mut rules = rules();
        rules.holiday_work_allowed = false;
        let result = plan_series_with(
            &rules,
            early(),
            date("2026-12-24"),
            3,
            &plan(&[]),
            &Company::default(),
            &holidays,
        );
        assert_eq!(result.allowed, 1);
        assert_eq!(result.blocked, Some((date("2026-12-25"), BLOCK_HOLIDAY)));

        rules.holiday_work_allowed = true;
        let result = plan_series_with(
            &rules,
            early(),
            date("2026-12-24"),
            3,
            &plan(&[]),
            &Company::default(),
            &holidays,
        );
        assert_eq!(result.allowed, 3);
        assert_eq!(result.blocked, None);
    }

    #[test]
    fn a_shift_that_runs_past_midnight_keeps_its_rest_to_the_next_day() {
        // Monday 22:00–06:00 already planned, Tuesday 08:00 is too early.
        let late = PlanShift::new("22:00", "06:00", 60).unwrap();
        let result = plan_series(
            &rules(),
            late,
            date("2026-11-16"),
            2,
            &plan(&[("2026-11-17", Some(early()))]),
        );
        assert_eq!(result.allowed, 0);
        assert_eq!(result.blocked, Some((date("2026-11-16"), BLOCK_REST)));
    }

    #[test]
    fn a_monday_after_a_sunday_only_keeps_the_regular_rest() {
        // Sunday 15:00–23:00 planned, Monday 15:00 leaves 16 hours: enough for the
        // rest of the settings. The extra day belongs to the compensation day, not to
        // the Monday.
        let mut rules = rules();
        rules.sunday_work_allowed = true;
        let late = PlanShift::new("15:00", "23:00", 30).unwrap();
        let planned = plan(&[("2026-11-15", Some(late))]);
        let result = plan_series(&rules, late, date("2026-11-16"), 1, &planned);
        assert_eq!(result.allowed, 1);
        assert_eq!(result.blocked, None);
        // The ordinary rest still holds: 08:00 is nine hours after 23:00.
        let result = plan_series(&rules, early(), date("2026-11-16"), 1, &planned);
        assert_eq!(result.allowed, 0);
        assert_eq!(result.blocked, Some((date("2026-11-16"), BLOCK_REST)));
    }

    #[test]
    fn the_compensation_day_after_a_sunday_carries_the_extra_day() {
        // Sunday 15:00–23:00, Monday free: the compensation day is the Monday, so the
        // next duty needs 24 + 11 hours after the Sunday — a night shift at 00:30 on
        // Tuesday has 25.5 hours, enough for the ordinary rest and too little for the
        // compensation.
        let mut rules = rules();
        rules.sunday_work_allowed = true;
        let late = PlanShift::new("15:00", "23:00", 30).unwrap();
        let night = PlanShift::new("00:30", "08:30", 30).unwrap();
        let planned = plan(&[("2026-11-15", Some(late))]);
        let result = plan_series(&rules, night, date("2026-11-17"), 1, &planned);
        assert_eq!(result.allowed, 0);
        assert_eq!(
            result.blocked,
            Some((date("2026-11-17"), BLOCK_COMPENSATION))
        );
        // Ten o'clock is the first time that fits (35 hours after 23:00).
        let late_start = PlanShift::new("10:00", "18:00", 30).unwrap();
        let result = plan_series(&rules, late_start, date("2026-11-17"), 1, &planned);
        assert_eq!(result.allowed, 1);

        // The rule is a German one and can be switched off.
        rules.extended_sunday_rest = false;
        let result = plan_series(&rules, night, date("2026-11-17"), 1, &planned);
        assert_eq!(result.allowed, 1);
    }

    #[test]
    fn the_compensation_day_may_fall_later_in_the_week() {
        // Sunday 15:00–23:00 and Monday 15:00–23:00 worked: the compensation day is the
        // Tuesday, so the Wednesday duty needs 24 + 11 hours after the Monday.
        let mut rules = rules();
        rules.sunday_work_allowed = true;
        rules.max_consecutive_days = 6;
        let late = PlanShift::new("15:00", "23:00", 30).unwrap();
        let planned = plan(&[("2026-11-15", Some(late)), ("2026-11-16", Some(late))]);
        let result = plan_series(&rules, early(), date("2026-11-18"), 1, &planned);
        assert_eq!(result.allowed, 0);
        assert_eq!(
            result.blocked,
            Some((date("2026-11-18"), BLOCK_COMPENSATION))
        );
        let late_start = PlanShift::new("10:00", "18:00", 30).unwrap();
        let result = plan_series(&rules, late_start, date("2026-11-18"), 1, &planned);
        assert_eq!(result.allowed, 1);
    }

    #[test]
    fn the_rest_before_a_sunday_stays_regular() {
        // Saturday 08:00–15:30 and Sunday 08:00 leave 16.5 hours; the extra day
        // only follows Sunday work.
        let mut rules = rules();
        rules.sunday_work_allowed = true;
        let planned = plan(&[("2026-11-14", Some(early()))]);
        let result = plan_series(&rules, early(), date("2026-11-15"), 1, &planned);
        assert_eq!(result.allowed, 1);
        assert_eq!(result.blocked, None);
    }

    #[test]
    fn the_regular_late_shift_is_no_weekend_work() {
        // Saturday the 05.12.2026: the regular late shift is refused, the weekend
        // shift of the same band is written.
        let rules = rules();
        let late = PlanShift::new("15:00", "23:00", 30)
            .unwrap()
            .without_weekend();
        let result = plan_series(&rules, late, date("2026-12-05"), 1, &PlannedDays::new());
        assert_eq!(result.allowed, 0);
        assert_eq!(result.blocked, Some((date("2026-12-05"), BLOCK_WEEKEND)));

        let we = PlanShift::new("14:00", "22:00", 30).unwrap();
        let result = plan_series(&rules, we, date("2026-12-05"), 1, &PlannedDays::new());
        assert_eq!(result.allowed, 1);
        // A weekday takes the regular one.
        let result = plan_series(&rules, late, date("2026-12-04"), 1, &PlannedDays::new());
        assert_eq!(result.allowed, 1);
    }

    #[test]
    fn a_sunday_before_a_planned_monday_keeps_the_regular_rest() {
        // Monday 08:00 already planned, the Sunday 08:00–15:30 before is written now:
        // 16.5 hours of rest are enough, and the Monday is no compensation day.
        let mut rules = rules();
        rules.sunday_work_allowed = true;
        let planned = plan(&[("2026-11-16", Some(early()))]);
        let result = plan_series(&rules, early(), date("2026-11-15"), 1, &planned);
        assert_eq!(result.allowed, 1);
        assert_eq!(result.blocked, None);
        // A late Sunday leaves only 9 hours to the Monday: the ordinary rest stops it.
        let late = PlanShift::new("15:00", "23:00", 30).unwrap();
        let result = plan_series(&rules, late, date("2026-11-15"), 1, &planned);
        assert_eq!(result.allowed, 0);
        assert_eq!(result.blocked, Some((date("2026-11-15"), BLOCK_REST)));
    }

    #[test]
    fn the_weekly_hours_stop_the_series() {
        let mut rules = rules();
        rules.max_weekly_hours = 40;
        rules.sunday_work_allowed = true;
        rules.max_consecutive_days = 7;
        rules.min_days_off_per_week = 0;
        rules.max_daily_hours = 10;
        // Six days of 6.5 hours are 39 hours; the seventh would exceed 40.
        let result = plan_series(&rules, early(), date("2026-11-16"), 7, &plan(&[]));
        assert_eq!(result.allowed, 6);
        assert_eq!(
            result.blocked,
            Some((date("2026-11-22"), BLOCK_WEEKLY_HOURS))
        );
    }

    #[test]
    fn a_week_keeps_its_day_off() {
        let mut rules = rules();
        rules.sunday_work_allowed = true;
        rules.max_consecutive_days = 7;
        let result = plan_series(&rules, early(), date("2026-11-16"), 7, &plan(&[]));
        assert_eq!(result.allowed, 6);
        assert_eq!(result.blocked, Some((date("2026-11-22"), BLOCK_DAYS_OFF)));
    }

    #[test]
    fn a_shift_longer_than_the_daily_maximum_is_refused() {
        let mut rules = rules();
        rules.max_daily_hours = 6;
        let result = plan_series(&rules, early(), date("2026-11-16"), 1, &plan(&[]));
        assert_eq!(result.allowed, 0);
        assert_eq!(
            result.blocked,
            Some((date("2026-11-16"), BLOCK_DAILY_HOURS))
        );
    }

    #[test]
    fn clocks_and_spans_are_read_like_the_shift_master() {
        assert_eq!(PlanShift::new("07:00", "15:30", 30).unwrap().net, 480);
        assert_eq!(PlanShift::new("22:00", "06:00", 0).unwrap().net, 480);
        assert!(PlanShift::new("08:00", "08:00", 0).is_none());
        assert!(PlanShift::new("24:00", "08:00", 0).is_none());
        assert!(PlanShift::new("eight", "08:00", 0).is_none());
    }
}
