//! Checking a plan that is already written.
//!
//! The series check of `schedule_rules` stops at the first day that breaks a rule,
//! because it writes as far as it may. A written plan cannot stop anywhere: the
//! audit walks every worked cell of a period and reports each one that breaks a
//! rule — that is what stands in the way of a printed plan. The cell itself does
//! not count as planned for its own check, so a rest period, a run of days or the
//! weekly hours are always measured against the neighbours.

use std::collections::{HashMap, HashSet};

use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::planning::PlanningRules;
use crate::schedule::{KIND_SICK, KIND_VACATION};
use crate::schedule_rules::{
    BLOCK_APPRENTICE_ONLY, BLOCK_EXCLUDED_POSITION, BLOCK_WEEKEND, Company, PlanShift, PlannedDays,
    block_reason,
};
use crate::shifts::Shift;

/// One cell that breaks a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ScheduleViolation {
    pub employee_id: i64,
    pub day: NaiveDate,
    /// The rule that is broken: `min_rest_hours`, `weekend`, `apprentice_only`, …
    pub reason: String,
}

/// What the audit found.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ScheduleAudit {
    /// Worked cells that were checked.
    pub checked: usize,
    /// The cells that break a rule, in the order of the days.
    pub violations: Vec<ScheduleViolation>,
}

/// A cell as the audit reads it.
#[derive(Debug, Clone)]
pub struct AuditCell {
    pub employee_id: i64,
    pub day: NaiveDate,
    pub kind: String,
    pub shift_id: Option<i64>,
}

/// What the audit needs to know about an employee.
#[derive(Debug, Clone)]
pub struct AuditPerson {
    pub id: i64,
    pub apprentice: bool,
    pub position: String,
}

/// The shift as the rule check needs it, with the weekend marker of the catalogue.
fn plan_shift(shift: &Shift) -> Option<PlanShift> {
    let plan = PlanShift::new(&shift.starts_at, &shift.ends_at, shift.break_minutes)?;
    Some(if shift.works_weekend() {
        plan
    } else {
        plan.without_weekend()
    })
}

/// Checks the plan of `first` … `last` and returns what breaks a rule.
///
/// `cells` may reach past the period: the days around it carry the rest period,
/// the weekly hours and the days in a row. Only the cells inside the period are
/// checked.
pub fn audit(
    rules: &PlanningRules,
    shifts: &[Shift],
    people: &[AuditPerson],
    cells: &[AuditCell],
    holidays: &HashSet<NaiveDate>,
    first: NaiveDate,
    last: NaiveDate,
) -> ScheduleAudit {
    let mut planned: HashMap<i64, PlannedDays> = HashMap::new();
    let mut worked: Vec<(i64, NaiveDate, &Shift)> = Vec::new();
    for cell in cells {
        let shift = cell
            .shift_id
            .and_then(|id| shifts.iter().find(|shift| shift.id == id));
        match shift {
            // Sick and vacation are known days: they keep a series from growing.
            None => {
                if cell.kind == KIND_SICK || cell.kind == KIND_VACATION {
                    planned
                        .entry(cell.employee_id)
                        .or_default()
                        .insert(cell.day, None);
                }
            }
            Some(shift) => {
                // A free day is no work — neither a cell of the plan nor an absence.
                if shift.band.is_free() {
                    continue;
                }
                let Some(plan) = plan_shift(shift) else {
                    continue;
                };
                planned
                    .entry(cell.employee_id)
                    .or_default()
                    .insert(cell.day, Some(plan));
                worked.push((cell.employee_id, cell.day, shift));
            }
        }
    }

    // The colleagues of a shift per day, as the apprentice rule needs them: only
    // somebody who is no apprentice counts, whatever his position, department or
    // location.
    let journeymen: HashSet<i64> = people
        .iter()
        .filter(|person| !person.apprentice)
        .map(|person| person.id)
        .collect();
    let mut alone: HashMap<(i64, i64), Company> = HashMap::new();
    for person in people.iter().filter(|person| person.apprentice) {
        for shift in shifts {
            let days: HashSet<NaiveDate> = worked
                .iter()
                .filter(|(employee, _, own)| {
                    own.id == shift.id && *employee != person.id && journeymen.contains(employee)
                })
                .map(|(_, day, _)| *day)
                .collect();
            alone.insert(
                (person.id, shift.id),
                Company {
                    // An apprentice shift takes apprentices only, so a journeyman can
                    // never stand in it — the rule asks for one and would report every
                    // such day. It applies to the shifts that are open to everybody,
                    // which is where an apprentice can stand on their own.
                    apprentice: !shift.apprentice_only,
                    days,
                },
            );
        }
    }

    let mut audit = ScheduleAudit::default();
    for (employee_id, day, shift) in worked {
        if day < first || day > last {
            continue;
        }
        audit.checked += 1;
        let Some(person) = people.iter().find(|person| person.id == employee_id) else {
            continue;
        };
        // The cell itself is no neighbour of its own check.
        let mut others = planned.get(&employee_id).cloned().unwrap_or_default();
        others.remove(&day);
        let plan = plan_shift(shift);
        let weekend = matches!(day.weekday(), Weekday::Sat | Weekday::Sun);
        let excluded = !person.position.trim().is_empty()
            && shift
                .excluded_positions
                .iter()
                .any(|position| position.trim().eq_ignore_ascii_case(person.position.trim()));
        let reason = if !person.apprentice && shift.apprentice_only {
            Some(BLOCK_APPRENTICE_ONLY)
        } else if weekend && !shift.works_weekend() {
            Some(BLOCK_WEEKEND)
        } else if excluded {
            Some(BLOCK_EXCLUDED_POSITION)
        } else {
            plan.and_then(|plan| {
                block_reason(
                    rules,
                    plan,
                    day,
                    &others,
                    alone
                        .get(&(employee_id, shift.id))
                        .unwrap_or(&Company::default()),
                    holidays,
                )
            })
        };
        if let Some(reason) = reason {
            audit.violations.push(ScheduleViolation {
                employee_id,
                day,
                reason: reason.to_string(),
            });
        }
    }
    audit
        .violations
        .sort_by(|left, right| (left.day, left.employee_id).cmp(&(right.day, right.employee_id)));
    audit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shifts::ShiftBand;

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

    fn person(id: i64) -> AuditPerson {
        AuditPerson {
            id,
            apprentice: false,
            position: "Meister".to_string(),
        }
    }

    fn cell(employee_id: i64, day: &str, shift_id: i64) -> AuditCell {
        AuditCell {
            employee_id,
            day: date(day),
            kind: "shift".to_string(),
            shift_id: Some(shift_id),
        }
    }

    fn run(shifts: &[Shift], people: &[AuditPerson], cells: &[AuditCell]) -> ScheduleAudit {
        audit(
            &PlanningRules::default(),
            shifts,
            people,
            cells,
            &HashSet::new(),
            date("2026-12-01"),
            date("2026-12-31"),
        )
    }

    #[test]
    fn a_plan_without_a_break_stays_clean() {
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1)];
        let cells: Vec<AuditCell> = ["01", "02", "03"]
            .iter()
            .map(|day| cell(1, &format!("2026-12-{day}"), 1))
            .collect();
        let audit = run(&shifts, &people, &cells);
        assert_eq!(audit.checked, 3);
        assert!(audit.violations.is_empty());
    }

    #[test]
    fn a_short_rest_period_is_reported() {
        // Late shift on the 01., early one on the 02.: nine hours rest.
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1)];
        let cells = vec![cell(1, "2026-12-01", 2), cell(1, "2026-12-02", 1)];
        let audit = run(&shifts, &people, &cells);
        assert_eq!(audit.checked, 2);
        assert_eq!(audit.violations.len(), 2);
        assert!(
            audit
                .violations
                .iter()
                .all(|violation| violation.reason == "min_rest_hours")
        );
    }

    #[test]
    fn a_regular_late_shift_on_a_saturday_is_reported() {
        let rules = PlanningRules {
            sunday_work_allowed: true,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
            shift(3, "S-WE", "14:00", "22:00", ShiftBand::Late, false),
        ];
        let people = vec![person(1)];
        // 05.12.2026 is a Saturday, the 06. a Sunday.
        let cells = vec![cell(1, "2026-12-05", 2), cell(1, "2026-12-06", 3)];
        let result = audit(
            &rules,
            &shifts,
            &people,
            &cells,
            &HashSet::new(),
            date("2026-12-01"),
            date("2026-12-31"),
        );
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].day, date("2026-12-05"));
        assert_eq!(result.violations[0].reason, "weekend");
    }

    #[test]
    fn an_apprentice_may_work_a_regular_shift_beside_a_journeyman() {
        // The planner writes those cells by hand before the generator fills the
        // rest, so they are no violation.
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(6, "F-AZ", "08:00", "15:30", ShiftBand::Early, true),
        ];
        let mut apprentice = person(2);
        apprentice.apprentice = true;
        let people = vec![person(1), apprentice];
        let cells = vec![
            cell(1, "2026-12-01", 1),
            cell(2, "2026-12-01", 1),
            cell(1, "2026-12-02", 1),
            cell(2, "2026-12-02", 1),
        ];
        let result = run(&shifts, &people, &cells);
        assert_eq!(result.checked, 4);
        assert!(result.violations.is_empty(), "{:?}", result.violations);
    }

    #[test]
    fn an_apprentice_alone_in_a_regular_shift_is_reported() {
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(2, "S", "15:00", "23:00", ShiftBand::Late, false),
        ];
        let mut apprentice = person(2);
        apprentice.apprentice = true;
        let people = vec![person(1), apprentice];
        // The journeyman works the late shift, so nobody shares the early one.
        let cells = vec![cell(1, "2026-12-01", 2), cell(2, "2026-12-01", 1)];
        let result = run(&shifts, &people, &cells);
        assert_eq!(result.checked, 2);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].employee_id, 2);
        assert_eq!(result.violations[0].day, date("2026-12-01"));
        assert_eq!(result.violations[0].reason, "apprentice_alone");
    }

    #[test]
    fn an_apprentice_shift_asks_for_no_journeyman() {
        // Only apprentices enter an apprentice shift, so it never holds the
        // journeyman the rule asks for — it does not apply there.
        let shifts = vec![shift(6, "BS", "08:00", "16:30", ShiftBand::Early, true)];
        let mut first = person(2);
        first.apprentice = true;
        let mut zweite = person(3);
        zweite.apprentice = true;
        let people = vec![first, zweite];
        let cells = vec![cell(2, "2026-12-01", 6), cell(3, "2026-12-01", 6)];
        let result = run(&shifts, &people, &cells);
        assert_eq!(result.checked, 2);
        assert!(result.violations.is_empty(), "{:?}", result.violations);
    }

    #[test]
    fn a_free_day_is_no_work_in_the_audit() {
        // The free day of the calendar is 23 hours and 59 minutes long — that must
        // not count into the weekly hours.
        let rules = PlanningRules {
            max_weekly_hours: 8,
            ..PlanningRules::default()
        };
        let shifts = vec![
            shift(1, "F", "08:00", "15:30", ShiftBand::Early, false),
            shift(34, "FT", "00:00", "23:59", ShiftBand::Free, false),
        ];
        let people = vec![person(1)];
        let cells = vec![
            cell(1, "2026-12-01", 1),
            cell(1, "2026-12-02", 34),
            cell(1, "2026-12-03", 34),
        ];
        let result = audit(
            &rules,
            &shifts,
            &people,
            &cells,
            &HashSet::new(),
            date("2026-12-01"),
            date("2026-12-31"),
        );
        assert_eq!(result.checked, 1);
        assert!(result.violations.is_empty(), "{:?}", result.violations);
    }

    #[test]
    fn a_run_longer_than_the_maximum_is_reported() {
        // Two days in a row are allowed, the third one breaks the rule.
        let rules = PlanningRules {
            max_consecutive_days: 2,
            ..PlanningRules::default()
        };
        let shifts = vec![shift(1, "F", "08:00", "15:30", ShiftBand::Early, false)];
        let people = vec![person(1)];
        let cells = vec![
            cell(1, "2026-12-01", 1),
            cell(1, "2026-12-02", 1),
            cell(1, "2026-12-03", 1),
        ];
        let result = audit(
            &rules,
            &shifts,
            &people,
            &cells,
            &HashSet::new(),
            date("2026-12-01"),
            date("2026-12-31"),
        );
        assert_eq!(result.checked, 3);
        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].day, date("2026-12-03"));
        assert_eq!(result.violations[0].reason, "max_consecutive_days");
    }
}
