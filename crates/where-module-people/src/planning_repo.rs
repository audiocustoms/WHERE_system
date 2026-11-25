//! Storage of the planning rules (`where_people_planning`, one row).

use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::planning::PlanningRules;

const SELECT_RULES: &str = "SELECT \"min_rest_hours\", \"max_daily_hours\", \
     \"max_weekly_hours\", \"break_after_hours\", \"break_minutes\", \
     \"long_break_after_hours\", \"long_break_minutes\", \"max_consecutive_days\", \
     \"shift_rotation_days\", \"min_days_off_per_week\", \"sunday_work_allowed\", \"compensate_sunday\", \
     \"extended_sunday_rest\", \"free_weekend_every_six_weeks\", \"apprentice_needs_company\", \
     \"import_public_holidays\", \"import_regional_holidays\", \
     \"import_school_holidays\", \"holiday_state\", \"holiday_color\", \
     \"show_holidays\", \"show_school_holidays\", \"school_holiday_color\", \
     \"holiday_work_allowed\", \"respect_absence\", \"planning_horizon\", \
     \"planning_horizon_unit\", \"vacation_day_hours\", \"sick_day_hours\", \
     \"vacation_color\", \"sick_color\", \"balance_from\", \"min_consecutive_days\" \
     FROM \"where_people_planning\" WHERE \"id\" = 1";

const UPSERT_RULES: &str = "INSERT INTO \"where_people_planning\" \
     (\"id\", \"min_rest_hours\", \"max_daily_hours\", \"max_weekly_hours\", \
      \"break_after_hours\", \"break_minutes\", \"long_break_after_hours\", \
      \"long_break_minutes\", \"max_consecutive_days\", \"min_days_off_per_week\", \
      \"sunday_work_allowed\", \"compensate_sunday\", \"extended_sunday_rest\", \
      \"free_weekend_every_six_weeks\", \"apprentice_needs_company\", \"import_public_holidays\", \
      \"import_regional_holidays\", \"import_school_holidays\", \"holiday_state\", \"holiday_color\", \
     \"show_holidays\", \"show_school_holidays\", \"school_holiday_color\", \
      \"holiday_work_allowed\", \
      \"respect_absence\", \"planning_horizon\", \"planning_horizon_unit\", \
      \"vacation_day_hours\", \"sick_day_hours\", \"vacation_color\", \
      \"sick_color\", \"balance_from\", \"shift_rotation_days\", \"min_consecutive_days\") \
     VALUES (1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
     $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33) \
     ON CONFLICT (\"id\") DO UPDATE SET \
        \"min_rest_hours\" = excluded.\"min_rest_hours\", \
        \"max_daily_hours\" = excluded.\"max_daily_hours\", \
        \"max_weekly_hours\" = excluded.\"max_weekly_hours\", \
        \"break_after_hours\" = excluded.\"break_after_hours\", \
        \"break_minutes\" = excluded.\"break_minutes\", \
        \"long_break_after_hours\" = excluded.\"long_break_after_hours\", \
        \"long_break_minutes\" = excluded.\"long_break_minutes\", \
        \"max_consecutive_days\" = excluded.\"max_consecutive_days\", \
        \"min_days_off_per_week\" = excluded.\"min_days_off_per_week\", \
        \"sunday_work_allowed\" = excluded.\"sunday_work_allowed\", \
        \"compensate_sunday\" = excluded.\"compensate_sunday\", \
        \"extended_sunday_rest\" = excluded.\"extended_sunday_rest\", \
        \"free_weekend_every_six_weeks\" = excluded.\"free_weekend_every_six_weeks\", \
        \"apprentice_needs_company\" = excluded.\"apprentice_needs_company\", \
        \"import_public_holidays\" = excluded.\"import_public_holidays\", \
        \"import_regional_holidays\" = excluded.\"import_regional_holidays\", \
        \"import_school_holidays\" = excluded.\"import_school_holidays\", \
        \"holiday_state\" = excluded.\"holiday_state\", \
        \"holiday_color\" = excluded.\"holiday_color\", \
        \"show_holidays\" = excluded.\"show_holidays\", \
        \"show_school_holidays\" = excluded.\"show_school_holidays\", \
        \"school_holiday_color\" = excluded.\"school_holiday_color\", \
        \"holiday_work_allowed\" = excluded.\"holiday_work_allowed\", \
        \"respect_absence\" = excluded.\"respect_absence\", \
        \"planning_horizon\" = excluded.\"planning_horizon\", \
        \"planning_horizon_unit\" = excluded.\"planning_horizon_unit\", \
        \"vacation_day_hours\" = excluded.\"vacation_day_hours\", \
        \"sick_day_hours\" = excluded.\"sick_day_hours\", \
        \"vacation_color\" = excluded.\"vacation_color\", \
        \"sick_color\" = excluded.\"sick_color\", \
        \"balance_from\" = excluded.\"balance_from\", \
        \"shift_rotation_days\" = excluded.\"shift_rotation_days\", \
        \"min_consecutive_days\" = excluded.\"min_consecutive_days\", \
        \"changed_at\" = now() \
     RETURNING \"min_rest_hours\", \"max_daily_hours\", \"max_weekly_hours\", \
     \"break_after_hours\", \"break_minutes\", \"long_break_after_hours\", \
     \"long_break_minutes\", \"max_consecutive_days\", \"min_days_off_per_week\", \
     \"sunday_work_allowed\", \"compensate_sunday\", \"extended_sunday_rest\", \
     \"free_weekend_every_six_weeks\", \"apprentice_needs_company\", \
     \"import_public_holidays\", \"import_regional_holidays\", \
     \"import_school_holidays\", \"holiday_state\", \"holiday_color\", \
     \"show_holidays\", \"show_school_holidays\", \"school_holiday_color\", \
     \"holiday_work_allowed\", \
     \"respect_absence\", \"planning_horizon\", \"planning_horizon_unit\", \
     \"vacation_day_hours\", \"sick_day_hours\", \"vacation_color\", \"sick_color\", \
     \"balance_from\", \"shift_rotation_days\", \"min_consecutive_days\"";

fn row_to_rules(row: &sqlx::postgres::PgRow) -> PlanningRules {
    PlanningRules {
        min_rest_hours: row.get("min_rest_hours"),
        max_daily_hours: row.get("max_daily_hours"),
        max_weekly_hours: row.get("max_weekly_hours"),
        break_after_hours: row.get("break_after_hours"),
        break_minutes: row.get("break_minutes"),
        long_break_after_hours: row.get("long_break_after_hours"),
        long_break_minutes: row.get("long_break_minutes"),
        max_consecutive_days: row.get("max_consecutive_days"),
        min_consecutive_days: row.get("min_consecutive_days"),
        shift_rotation_days: row.get("shift_rotation_days"),
        min_days_off_per_week: row.get("min_days_off_per_week"),
        sunday_work_allowed: row.get("sunday_work_allowed"),
        compensate_sunday: row.get("compensate_sunday"),
        extended_sunday_rest: row.get("extended_sunday_rest"),
        free_weekend_every_six_weeks: row.get("free_weekend_every_six_weeks"),
        apprentice_needs_company: row.get("apprentice_needs_company"),
        import_public_holidays: row.get("import_public_holidays"),
        import_regional_holidays: row.get("import_regional_holidays"),
        import_school_holidays: row.get("import_school_holidays"),
        holiday_state: row.get("holiday_state"),
        holiday_color: row.get("holiday_color"),
        show_holidays: row.get("show_holidays"),
        show_school_holidays: row.get("show_school_holidays"),
        school_holiday_color: row.get("school_holiday_color"),
        holiday_work_allowed: row.get("holiday_work_allowed"),
        respect_absence: row.get("respect_absence"),
        planning_horizon: row.get("planning_horizon"),
        planning_horizon_unit: row.get("planning_horizon_unit"),
        vacation_day_hours: row.get("vacation_day_hours"),
        sick_day_hours: row.get("sick_day_hours"),
        vacation_color: row.get("vacation_color"),
        sick_color: row.get("sick_color"),
        balance_from: row.get("balance_from"),
    }
}

/// Reads and writes the one set of rules.
#[derive(Clone)]
pub struct PlanningRepository {
    pool: PgPool,
}

impl PlanningRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Without a row the legal defaults apply.
    pub async fn get(&self) -> Result<PlanningRules, DomainError> {
        let row = sqlx::query(SELECT_RULES)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row
            .as_ref()
            .map(row_to_rules)
            .unwrap_or_else(PlanningRules::default))
    }

    pub async fn save(&self, rules: &PlanningRules) -> Result<PlanningRules, DomainError> {
        // Without Sunday work there is nothing to compensate and no extended rest.
        let rules = PlanningRules {
            compensate_sunday: rules.compensate_sunday && rules.sunday_work_allowed,
            extended_sunday_rest: rules.extended_sunday_rest && rules.sunday_work_allowed,
            vacation_color: rules.vacation_color.trim().to_ascii_uppercase(),
            sick_color: rules.sick_color.trim().to_ascii_uppercase(),
            holiday_color: rules.holiday_color.trim().to_ascii_uppercase(),
            school_holiday_color: rules.school_holiday_color.trim().to_ascii_uppercase(),
            // The state is a two-letter code; an empty field means "none".
            holiday_state: rules
                .holiday_state
                .as_deref()
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(str::to_ascii_uppercase),
            ..rules.clone()
        };
        rules.validate()?;
        let row = sqlx::query(UPSERT_RULES)
            .bind(rules.min_rest_hours)
            .bind(rules.max_daily_hours)
            .bind(rules.max_weekly_hours)
            .bind(rules.break_after_hours)
            .bind(rules.break_minutes)
            .bind(rules.long_break_after_hours)
            .bind(rules.long_break_minutes)
            .bind(rules.max_consecutive_days)
            .bind(rules.min_days_off_per_week)
            .bind(rules.sunday_work_allowed)
            .bind(rules.compensate_sunday)
            .bind(rules.extended_sunday_rest)
            .bind(rules.free_weekend_every_six_weeks)
            .bind(rules.apprentice_needs_company)
            .bind(rules.import_public_holidays)
            .bind(rules.import_regional_holidays)
            .bind(rules.import_school_holidays)
            .bind(&rules.holiday_state)
            .bind(&rules.holiday_color)
            .bind(rules.show_holidays)
            .bind(rules.show_school_holidays)
            .bind(&rules.school_holiday_color)
            .bind(rules.holiday_work_allowed)
            .bind(rules.respect_absence)
            .bind(rules.planning_horizon)
            .bind(&rules.planning_horizon_unit)
            .bind(rules.vacation_day_hours)
            .bind(rules.sick_day_hours)
            .bind(&rules.vacation_color)
            .bind(&rules.sick_color)
            .bind(rules.balance_from)
            .bind(rules.shift_rotation_days)
            .bind(rules.min_consecutive_days)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row_to_rules(&row))
    }
}
