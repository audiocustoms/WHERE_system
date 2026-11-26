//! Rules the schedule planning follows.
//!
//! The defaults are the usual values for rest period, maximum working time and
//! breaks and can be adjusted per installation. There is exactly one set of
//! rules (`where_people_planning`, `id = 1`).

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::DomainError;

/// The rules of the schedule planning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PlanningRules {
    /// Rest period between two shifts in hours (default 11).
    pub min_rest_hours: i16,
    /// Maximum working time per day in hours (default 10).
    pub max_daily_hours: i16,
    /// Maximum working time per week in hours (default 48).
    pub max_weekly_hours: i16,
    /// From this working time the short break is due (default 6).
    pub break_after_hours: i16,
    /// Length of the short break in minutes (default 30).
    pub break_minutes: i16,
    /// From this working time the long break is due (default 9).
    pub long_break_after_hours: i16,
    /// Length of the long break in minutes (default 60).
    pub long_break_minutes: i16,
    /// Maximum number of consecutive working days.
    pub max_consecutive_days: i16,
    /// Working days in a row before a day off may follow (1 = no minimum).
    pub min_consecutive_days: i16,
    /// Days an employee keeps the same shift before the planning may move them
    /// from early to late or the other way round (default 14).
    pub shift_rotation_days: i16,
    /// Minimum number of days off per week.
    pub min_days_off_per_week: i16,
    /// Is work allowed on Sundays?
    pub sunday_work_allowed: bool,
    /// Is a worked Sunday compensated on another weekday?
    pub compensate_sunday: bool,
    /// After a Sunday shift the rest period is one full day plus the regular
    /// rest period — 24 + 11 hours with the default 11 hours (Germany).
    pub extended_sunday_rest: bool,
    /// With a six-day week every sixth week has to leave a whole weekend
    /// (Saturday and Sunday together) free (Germany).
    pub free_weekend_every_six_weeks: bool,
    /// An apprentice may never be alone in a shift (Germany).
    pub apprentice_needs_company: bool,
    /// Import the nationwide public holidays of Germany.
    pub import_public_holidays: bool,
    /// Import the regional public holidays of `holiday_state`.
    pub import_regional_holidays: bool,
    /// Import the school holidays of `holiday_state`.
    pub import_school_holidays: bool,
    /// Federal state (`BW`, `BY`, …) for the regional and school holidays.
    #[serde(default)]
    pub holiday_state: Option<String>,
    /// Colour of the holiday columns in the schedule plan as `#RRGGBB`.
    pub holiday_color: String,
    /// Show the public holidays on the board (independent of the import).
    pub show_holidays: bool,
    /// Show the school holidays on the board (independent of the import).
    pub show_school_holidays: bool,
    /// Colour of the school holidays in the schedule plan as `#RRGGBB`.
    pub school_holiday_color: String,
    /// Is work allowed on public holidays?
    pub holiday_work_allowed: bool,
    /// Exclude vacation and absences from the planning?
    pub respect_absence: bool,
    /// Over how many weeks or months a plan reaches. With `months` the period
    /// starts on the 1st of the month.
    pub planning_horizon: i16,
    /// Unit of the horizon: `weeks` or `months`.
    pub planning_horizon_unit: String,
    /// Hours a vacation day counts (default 8).
    pub vacation_day_hours: Decimal,
    /// Hours a sick day counts (default 8).
    pub sick_day_hours: Decimal,
    /// Colour of the vacation cells in the schedule plan as `#RRGGBB`.
    pub vacation_color: String,
    /// Colour of the sick cells in the schedule plan as `#RRGGBB`.
    pub sick_color: String,
    /// From this date the plan counts over- and under-hours; everything before
    /// stays out of the balance. Without a date the balance starts on 1 January.
    #[serde(default)]
    pub balance_from: Option<NaiveDate>,
}

/// Allowed units of the planning horizon.
pub const HORIZON_WEEKS: &str = "weeks";
/// Unit `months`.
pub const HORIZON_MONTHS: &str = "months";
/// Default colour of the vacation cells.
pub const DEFAULT_VACATION_COLOR: &str = "#0369A1";
/// Default colour of the sick cells.
pub const DEFAULT_SICK_COLOR: &str = "#B91C1C";
/// Default colour of the holiday columns in the schedule plan.
pub const DEFAULT_HOLIDAY_COLOR: &str = "#B45309";
/// Default colour of the school holidays in the schedule plan.
pub const DEFAULT_SCHOOL_HOLIDAY_COLOR: &str = "#2563EB";
/// Days an employee keeps the same shift before early and late may swap.
pub const DEFAULT_SHIFT_ROTATION_DAYS: i16 = 14;

impl Default for PlanningRules {
    fn default() -> Self {
        Self {
            min_rest_hours: 11,
            max_daily_hours: 10,
            max_weekly_hours: 48,
            break_after_hours: 6,
            break_minutes: 30,
            long_break_after_hours: 9,
            long_break_minutes: 60,
            max_consecutive_days: 6,
            min_consecutive_days: 1,
            shift_rotation_days: DEFAULT_SHIFT_ROTATION_DAYS,
            min_days_off_per_week: 1,
            sunday_work_allowed: false,
            compensate_sunday: false,
            extended_sunday_rest: true,
            free_weekend_every_six_weeks: true,
            apprentice_needs_company: true,
            import_public_holidays: false,
            import_regional_holidays: false,
            import_school_holidays: false,
            holiday_state: None,
            holiday_color: DEFAULT_HOLIDAY_COLOR.to_string(),
            show_holidays: true,
            show_school_holidays: true,
            school_holiday_color: DEFAULT_SCHOOL_HOLIDAY_COLOR.to_string(),
            holiday_work_allowed: false,
            respect_absence: true,
            planning_horizon: 4,
            planning_horizon_unit: HORIZON_WEEKS.to_string(),
            vacation_day_hours: Decimal::new(800, 2),
            sick_day_hours: Decimal::new(800, 2),
            vacation_color: DEFAULT_VACATION_COLOR.to_string(),
            sick_color: DEFAULT_SICK_COLOR.to_string(),
            balance_from: None,
        }
    }
}

impl PlanningRules {
    /// Checks the values before they are stored.
    pub fn validate(&self) -> Result<(), DomainError> {
        let fail = |text: &str| DomainError::Validation(text.to_string());
        if !(0..=24).contains(&self.min_rest_hours) {
            return Err(fail("The rest period is between 0 and 24 hours."));
        }
        if !(1..=24).contains(&self.max_daily_hours) {
            return Err(fail(
                "The maximum daily working time is between 1 and 24 hours.",
            ));
        }
        if !(1..=168).contains(&self.max_weekly_hours) {
            return Err(fail(
                "The maximum weekly working time is between 1 and 168 hours.",
            ));
        }
        if self.max_weekly_hours < self.max_daily_hours {
            return Err(fail(
                "The weekly working time is smaller than the daily working time.",
            ));
        }
        if !(1..=24).contains(&self.break_after_hours)
            || !(1..=24).contains(&self.long_break_after_hours)
        {
            return Err(fail("The break thresholds are between 1 and 24 hours."));
        }
        if !(0..=240).contains(&self.break_minutes) || !(0..=240).contains(&self.long_break_minutes)
        {
            return Err(fail("The breaks are between 0 and 240 minutes."));
        }
        if self.long_break_after_hours < self.break_after_hours {
            return Err(fail("The long break starts before the short break."));
        }
        if self.long_break_minutes < self.break_minutes {
            return Err(fail("The long break is shorter than the short break."));
        }
        if !(1..=31).contains(&self.max_consecutive_days) {
            return Err(fail("The consecutive working days are between 1 and 31."));
        }
        if !(1..=31).contains(&self.min_consecutive_days) {
            return Err(fail(
                "The least consecutive working days are between 1 and 31.",
            ));
        }
        if self.min_consecutive_days > self.max_consecutive_days {
            return Err(fail(
                "The least consecutive working days are above the most.",
            ));
        }
        if !(1..=365).contains(&self.shift_rotation_days) {
            return Err(fail("The shift rotation is between 1 and 365 days."));
        }
        if !(0..=7).contains(&self.min_days_off_per_week) {
            return Err(fail("The days off per week are between 0 and 7."));
        }
        match self.planning_horizon_unit.as_str() {
            HORIZON_WEEKS => {
                if !(1..=52).contains(&self.planning_horizon) {
                    return Err(fail("The planning horizon is between 1 and 52 weeks."));
                }
            }
            HORIZON_MONTHS => {
                if !(1..=12).contains(&self.planning_horizon) {
                    return Err(fail("The planning horizon is between 1 and 12 months."));
                }
            }
            _ => {
                return Err(fail("The planning horizon is given in weeks or months."));
            }
        }
        if self.vacation_day_hours < Decimal::ZERO || self.vacation_day_hours > Decimal::from(24) {
            return Err(fail("A vacation day counts between 0 and 24 hours."));
        }
        if self.sick_day_hours < Decimal::ZERO || self.sick_day_hours > Decimal::from(24) {
            return Err(fail("A sick day counts between 0 and 24 hours."));
        }
        if !crate::shifts::is_color(&self.vacation_color)
            || !crate::shifts::is_color(&self.sick_color)
            || !crate::shifts::is_color(&self.holiday_color)
            || !crate::shifts::is_color(&self.school_holiday_color)
        {
            return Err(fail("The colours are given as #RRGGBB."));
        }
        if let Some(state) = &self.holiday_state
            && !crate::holidays::is_state(state)
        {
            return Err(fail("The federal state is not one of the 16."));
        }
        if (self.import_regional_holidays || self.import_school_holidays)
            && self.holiday_state.is_none()
        {
            return Err(fail("Regional and school holidays need a federal state."));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_usual_values() {
        let rules = PlanningRules::default();
        assert_eq!(rules.min_rest_hours, 11);
        assert_eq!(rules.max_daily_hours, 10);
        assert_eq!(rules.max_weekly_hours, 48);
        assert_eq!(rules.break_minutes, 30);
        assert_eq!(rules.long_break_minutes, 60);
        assert_eq!(rules.shift_rotation_days, 14);
        assert!(!rules.sunday_work_allowed);
        assert!(!rules.compensate_sunday);
        assert!(rules.extended_sunday_rest);
        assert!(rules.free_weekend_every_six_weeks);
        assert!(rules.apprentice_needs_company);
        assert!(rules.validate().is_ok());
    }

    #[test]
    fn a_work_week_needs_room_for_the_longest_day() {
        let mut rules = PlanningRules::default();
        rules.max_weekly_hours = 5;
        assert!(rules.validate().is_err());
    }

    #[test]
    fn the_pauses_keep_their_order() {
        let mut rules = PlanningRules::default();
        rules.long_break_minutes = 20;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.long_break_after_hours = 4;
        assert!(rules.validate().is_err());
    }

    #[test]
    fn every_value_keeps_to_its_range() {
        let mut rules = PlanningRules::default();
        rules.min_rest_hours = 25;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.max_daily_hours = 0;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.max_consecutive_days = 0;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.min_consecutive_days = 0;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.min_consecutive_days = rules.max_consecutive_days + 1;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.min_consecutive_days = rules.max_consecutive_days;
        assert!(rules.validate().is_ok());
        let mut rules = PlanningRules::default();
        rules.shift_rotation_days = 0;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.shift_rotation_days = 366;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.min_days_off_per_week = 8;
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.planning_horizon = 53;
        assert!(rules.validate().is_err());
    }

    #[test]
    fn the_absence_days_count_hours_up_to_a_day() {
        let mut rules = PlanningRules::default();
        assert_eq!(rules.vacation_day_hours.to_string(), "8.00");
        assert_eq!(rules.sick_day_hours.to_string(), "8.00");
        rules.vacation_day_hours = Decimal::new(750, 2);
        assert!(rules.validate().is_ok());
        rules.vacation_day_hours = Decimal::from(25);
        assert!(rules.validate().is_err());
        let mut rules = PlanningRules::default();
        rules.sick_day_hours = Decimal::new(-50, 2);
        assert!(rules.validate().is_err());
    }

    #[test]
    fn the_absence_colours_are_hex_triplets() {
        let mut rules = PlanningRules::default();
        assert_eq!(rules.vacation_color, DEFAULT_VACATION_COLOR);
        assert_eq!(rules.sick_color, DEFAULT_SICK_COLOR);
        assert_eq!(rules.holiday_color, DEFAULT_HOLIDAY_COLOR);
        assert_eq!(rules.school_holiday_color, DEFAULT_SCHOOL_HOLIDAY_COLOR);
        assert!(rules.validate().is_ok());
        rules.holiday_color = "braun".to_string();
        assert!(rules.validate().is_err());
        rules.holiday_color = DEFAULT_HOLIDAY_COLOR.to_string();
        rules.sick_color = "#1f6feb".to_string();
        assert!(rules.validate().is_ok());
        rules.sick_color = "red".to_string();
        assert!(rules.validate().is_err());
        rules.sick_color = DEFAULT_SICK_COLOR.to_string();
        rules.vacation_color = "#12345".to_string();
        assert!(rules.validate().is_err());
    }

    #[test]
    fn the_holiday_import_needs_a_federal_state() {
        let mut rules = PlanningRules::default();
        assert!(rules.validate().is_ok());
        rules.import_public_holidays = true;
        assert!(rules.validate().is_ok());
        rules.import_regional_holidays = true;
        assert!(rules.validate().is_err());
        rules.holiday_state = Some("BY".to_string());
        assert!(rules.validate().is_ok());
        rules.import_school_holidays = true;
        assert!(rules.validate().is_ok());
        rules.holiday_state = Some("XX".to_string());
        assert!(rules.validate().is_err());
    }

    #[test]
    fn the_horizon_counts_weeks_or_months() {
        let mut rules = PlanningRules::default();
        assert_eq!(rules.planning_horizon_unit, HORIZON_WEEKS);
        rules.planning_horizon = 52;
        assert!(rules.validate().is_ok());
        rules.planning_horizon = 53;
        assert!(rules.validate().is_err());
        rules.planning_horizon_unit = HORIZON_MONTHS.to_string();
        rules.planning_horizon = 12;
        assert!(rules.validate().is_ok());
        rules.planning_horizon = 13;
        assert!(rules.validate().is_err());
        rules.planning_horizon = 3;
        rules.planning_horizon_unit = "days".to_string();
        assert!(rules.validate().is_err());
    }
}
