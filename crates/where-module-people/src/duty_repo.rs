//! Storage of the duty entries (`where_people_duty_entries`).

use std::collections::HashSet;

use chrono::{Datelike, Days, NaiveDate, Weekday};
use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::duty::{
    CleanDuty, CleanSeries, DutyBlocked, DutyEntry, DutySeriesResult, KIND_FREE, KIND_SHIFT,
    RestoreCell,
};
use crate::duty_rules::{
    BLOCK_EXCLUDED_POSITION, Company, PlanShift, PlannedDays, plan_series_with,
};
use crate::holiday_repo::HolidayRepository;
use crate::planning::PlanningRules;
use crate::shifts::{ShiftBand, works_weekend};

const SELECT_RANGE: &str = "SELECT \"id\", \"employee_id\", \"kind\", \"shift_id\", \"starts_at\", \
     \"ends_at\", \"break_minutes\", \"note\", \"day\" \
     FROM \"where_people_duty_entries\" \
     WHERE \"day\" BETWEEN $1 AND $2 \
     ORDER BY \"day\", \"employee_id\"";

const UPSERT_ENTRY: &str = "INSERT INTO \"where_people_duty_entries\" \
     (\"employee_id\", \"kind\", \"shift_id\", \"starts_at\", \"ends_at\", \
      \"break_minutes\", \"note\", \"day\") \
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
     ON CONFLICT (\"employee_id\", \"day\") DO UPDATE SET \
        \"kind\" = excluded.\"kind\", \"shift_id\" = excluded.\"shift_id\", \
        \"starts_at\" = excluded.\"starts_at\", \"ends_at\" = excluded.\"ends_at\", \
        \"break_minutes\" = excluded.\"break_minutes\", \"note\" = excluded.\"note\", \
        \"changed_at\" = now() \
     RETURNING \"id\", \"employee_id\", \"kind\", \"shift_id\", \"starts_at\", \
     \"ends_at\", \"break_minutes\", \"note\", \"day\"";

const DELETE_ENTRY: &str =
    "DELETE FROM \"where_people_duty_entries\" WHERE \"employee_id\" = $1 AND \"day\" = $2";

/// Empties a month before an exchange file is written into it.
const DELETE_MONTH: &str =
    "DELETE FROM \"where_people_duty_entries\" WHERE \"day\" BETWEEN $1 AND $2";
/// Removes the plan of a period: every shift cell — the free days (`FT`, `SoFT`)
/// included —, every free time window and every vacation. Sick stays: it is a
/// record of the personnel file, not a part of the plan.
const DELETE_PLAN_CELLS: &str = "DELETE FROM \"where_people_duty_entries\" \
     WHERE \"day\" BETWEEN $1 AND $2 AND \"kind\" IN ('shift', 'free', 'vacation')";

/// The same reset for the departments a session may work in: the plan of the other
/// houses stays.
const DELETE_PLAN_CELLS_OF_DEPARTMENTS: &str = "DELETE FROM \"where_people_duty_entries\" e \
     USING \"where_employees\" w \
     WHERE w.\"id\" = e.\"employee_id\" AND e.\"day\" BETWEEN $1 AND $2 \
     AND e.\"kind\" IN ('shift', 'free', 'vacation') \
     AND lower(btrim(coalesce(w.\"department\", ''))) = ANY($3)";

/// Clears the planned shift cells of a period for the generator. The free days keep
/// untouched — they are the markers the generator fills itself, and a person it does
/// not plan again would lose their `FT` with them.
const DELETE_PLANNED_SHIFTS: &str = "DELETE FROM \"where_people_duty_entries\" e \
     WHERE e.\"kind\" = 'shift' AND e.\"day\" BETWEEN $1 AND $2 \
     AND NOT EXISTS (SELECT 1 FROM \"where_people_shifts\" s \
     WHERE s.\"id\" = e.\"shift_id\" AND s.\"band\" = 'frei')";

/// The same clearing for the departments a session may work in.
const DELETE_PLANNED_SHIFTS_OF_DEPARTMENTS: &str = "DELETE FROM \"where_people_duty_entries\" e \
     USING \"where_employees\" w \
     WHERE w.\"id\" = e.\"employee_id\" AND e.\"kind\" = 'shift' \
     AND e.\"day\" BETWEEN $1 AND $2 \
     AND lower(btrim(coalesce(w.\"department\", ''))) = ANY($3) \
     AND NOT EXISTS (SELECT 1 FROM \"where_people_shifts\" s \
     WHERE s.\"id\" = e.\"shift_id\" AND s.\"band\" = 'frei')";

/// Writes many shift cells at once. A shift cell carries no times of its own —
/// they come from the shift.
const INSERT_SHIFTS: &str = "INSERT INTO \"where_people_duty_entries\" \
     (\"employee_id\", \"kind\", \"shift_id\", \"starts_at\", \"ends_at\", \
      \"break_minutes\", \"day\") \
        SELECT z.\"employee_id\", 'shift', z.\"shift_id\", NULL, NULL, NULL, z.\"day\" \
     FROM unnest($1::bigint[], $2::date[], $3::bigint[]) AS z(\"employee_id\", \"day\", \"shift_id\") \
     ON CONFLICT (\"employee_id\", \"day\") DO UPDATE SET \
        \"kind\" = excluded.\"kind\", \"shift_id\" = excluded.\"shift_id\", \
        \"starts_at\" = excluded.\"starts_at\", \"ends_at\" = excluded.\"ends_at\", \
        \"break_minutes\" = excluded.\"break_minutes\", \"changed_at\" = now()";
const UPSERT_SERIES: &str = "INSERT INTO \"where_people_duty_entries\" \
     (\"employee_id\", \"kind\", \"shift_id\", \"starts_at\", \"ends_at\", \
      \"break_minutes\", \"day\") \
     SELECT $1, $2, $3, $4, $5, $6, \"day\"::date \
     FROM generate_series($7::date, $7::date + ($8::int - 1), interval '1 day') AS \"day\" \
     ON CONFLICT (\"employee_id\", \"day\") DO UPDATE SET \
        \"kind\" = excluded.\"kind\", \"shift_id\" = excluded.\"shift_id\", \
        \"starts_at\" = excluded.\"starts_at\", \"ends_at\" = excluded.\"ends_at\", \
        \"break_minutes\" = excluded.\"break_minutes\", \
        \"changed_at\" = now() \
     RETURNING \"id\", \"employee_id\", \"kind\", \"shift_id\", \"starts_at\", \
     \"ends_at\", \"break_minutes\", \"note\", \"day\"";

const SELECT_SHIFT: &str = "SELECT \"starts_at\", \"ends_at\", \"break_minutes\", \"band\", \
     \"short_name\", \"name\" FROM \"where_people_shifts\" WHERE \"id\" = $1";

const SELECT_APPRENTICE: &str = "SELECT \"is_apprentice\" FROM \"where_employees\" \
     WHERE \"id\" = $1";

/// `true` when the shift is a whole free day.
const SELECT_FREE_SHIFT: &str = "SELECT \"band\" FROM \"where_people_shifts\" WHERE \"id\" = $1";

const SELECT_EMPLOYEE_POSITION: &str = "SELECT \"position\" FROM \"where_employees\" \
     WHERE \"id\" = $1";

const SELECT_SHIFT_EXCLUSIONS: &str = "SELECT \"position\" \
     FROM \"where_people_shift_excluded_positions\" WHERE \"shift_id\" = $1";

const SELECT_SHIFT_APPRENTICE_ONLY: &str = "SELECT \"apprentice_only\", \"band\" \
     FROM \"where_people_shifts\" WHERE \"id\" = $1";

/// Days on which the same shift holds a colleague who is no apprentice. Position,
/// department and location play no part.
const SELECT_COMPANY_DAYS: &str = "SELECT DISTINCT e.\"day\" \
     FROM \"where_people_duty_entries\" e \
     JOIN \"where_employees\" w ON w.\"id\" = e.\"employee_id\" \
     WHERE e.\"shift_id\" = $1 AND e.\"employee_id\" <> $2 AND e.\"day\" BETWEEN $3 AND $4 \
       AND NOT w.\"is_apprentice\"";

/// Days on which the employee has a colleague at all — a free time has no shift
/// to compare with. Only a colleague who is no apprentice counts.
const SELECT_ANY_COMPANY_DAYS: &str = "SELECT DISTINCT e.\"day\" \
     FROM \"where_people_duty_entries\" e \
     JOIN \"where_employees\" w ON w.\"id\" = e.\"employee_id\" \
     WHERE e.\"employee_id\" <> $1 AND e.\"day\" BETWEEN $2 AND $3 \
       AND e.\"kind\" IN ('shift', 'free') \
       AND NOT w.\"is_apprentice\"";

const SELECT_PLANNED: &str = "SELECT e.\"kind\", e.\"day\", e.\"starts_at\", e.\"ends_at\", \
     e.\"break_minutes\", s.\"starts_at\" AS \"shift_starts_at\", \
     s.\"ends_at\" AS \"shift_ends_at\", s.\"break_minutes\" AS \"shift_break_minutes\", \
     s.\"band\" AS \"shift_band\" \
     FROM \"where_people_duty_entries\" e \
     LEFT JOIN \"where_people_shifts\" s ON s.\"id\" = e.\"shift_id\" \
     WHERE e.\"employee_id\" = $1 AND e.\"day\" BETWEEN $2 AND $3";

fn row_to_entry(row: &sqlx::postgres::PgRow) -> DutyEntry {
    DutyEntry {
        id: row.get("id"),
        employee_id: row.get("employee_id"),
        kind: row.get("kind"),
        shift_id: row.get("shift_id"),
        starts_at: row.get("starts_at"),
        ends_at: row.get("ends_at"),
        break_minutes: row.get("break_minutes"),
        note: row.get("note"),
        day: row.get("day"),
    }
}

/// Refusal of a shift on a public holiday that may not be worked.
fn holiday_work() -> DomainError {
    DomainError::Conflict("Work on a public holiday is not allowed.".to_string())
}

/// Refusal of a shift that excludes the position of the employee.
fn excluded_position(position: &str) -> DomainError {
    DomainError::Conflict(format!(
        "The position {position} is excluded from this shift."
    ))
}

/// Refusal of an apprentice shift for somebody who is not an apprentice.
fn apprentice_only_shift() -> DomainError {
    DomainError::Conflict("Only apprentices may be planned into this shift.".to_string())
}

/// Refusal of the regular late shift on a Saturday or a Sunday.
fn weekend_shift() -> DomainError {
    DomainError::Conflict("The late shift is not worked on Saturday or Sunday.".to_string())
}

/// Reads and writes the entries of the duty plan.
#[derive(Clone)]
pub struct DutyRepository {
    pool: PgPool,
}

impl DutyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DutyEntry>, DomainError> {
        let rows = sqlx::query(SELECT_RANGE)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.iter().map(row_to_entry).collect())
    }

    /// Sets the cell of one employee on one day. A shift that an apprentice may not
    /// take, that excludes the position or that falls on a public holiday is refused.
    /// A lone apprentice is not: the planner may write that cell by hand, and the
    /// print check warns about it.
    pub async fn set(
        &self,
        entry: &CleanDuty,
        rules: &PlanningRules,
    ) -> Result<DutyEntry, DomainError> {
        if entry.kind == KIND_SHIFT || entry.kind == KIND_FREE {
            if let Some(shift_id) = entry.shift_id {
                if let Some(position) = self.excluded_for_shift(entry.employee_id, shift_id).await?
                {
                    return Err(excluded_position(&position));
                }
                if let Some(error) = self
                    .apprentice_mismatch(entry.employee_id, shift_id)
                    .await?
                {
                    return Err(error);
                }
                if self.weekend_refused(shift_id, entry.day).await? {
                    return Err(weekend_shift());
                }
            }
            if !rules.holiday_work_allowed
                && self
                    .holidays(entry.day, entry.day)
                    .await?
                    .contains(&entry.day)
            {
                return Err(holiday_work());
            }
        }
        let row = sqlx::query(UPSERT_ENTRY)
            .bind(entry.employee_id)
            .bind(&entry.kind)
            .bind(entry.shift_id)
            .bind(&entry.starts_at)
            .bind(&entry.ends_at)
            .bind(entry.break_minutes)
            .bind(&entry.note)
            .bind(entry.day)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row_to_entry(&row))
    }

    /// Removes the whole plan of the period and returns how many cells it was — every
    /// shift (the free days `FT`/`SoFT` included), every free time window and every
    /// vacation. Sick stays.
    pub async fn clear_shifts(&self, from: NaiveDate, to: NaiveDate) -> Result<usize, DomainError> {
        let affected = sqlx::query(DELETE_PLAN_CELLS)
            .bind(from)
            .bind(to)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(affected.rows_affected() as usize)
    }

    /// The same reset for the departments a session may work in — the plan of the
    /// other houses keeps its cells.
    pub async fn clear_shifts_of(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        departments: &[String],
    ) -> Result<usize, DomainError> {
        let affected = sqlx::query(DELETE_PLAN_CELLS_OF_DEPARTMENTS)
            .bind(from)
            .bind(to)
            .bind(departments)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(affected.rows_affected() as usize)
    }

    /// Removes the planned shift cells of a period and returns how many it was; the
    /// free days stay, because the generator fills them itself.
    pub async fn clear_planned_shifts(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<usize, DomainError> {
        let affected = sqlx::query(DELETE_PLANNED_SHIFTS)
            .bind(from)
            .bind(to)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(affected.rows_affected() as usize)
    }

    /// The same for the departments a session may work in — the planned shifts of the
    /// other houses stay.
    pub async fn clear_planned_shifts_of(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        departments: &[String],
    ) -> Result<usize, DomainError> {
        let affected = sqlx::query(DELETE_PLANNED_SHIFTS_OF_DEPARTMENTS)
            .bind(from)
            .bind(to)
            .bind(departments)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(affected.rows_affected() as usize)
    }

    /// Writes a month of the plan as an exchange file carried it: the month is
    /// emptied first, then every cell is written — a restore reproduces a plan, it
    /// does not plan, so no rule is asked.
    pub async fn restore(
        &self,
        from: NaiveDate,
        to: NaiveDate,
        cells: &[RestoreCell],
        replace: bool,
    ) -> Result<usize, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if replace {
            sqlx::query(DELETE_MONTH)
                .bind(from)
                .bind(to)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        let mut written = 0usize;
        for cell in cells {
            sqlx::query(UPSERT_ENTRY)
                .bind(cell.employee_id)
                .bind(&cell.kind)
                .bind(cell.shift_id)
                .bind(&cell.starts_at)
                .bind(&cell.ends_at)
                .bind(cell.break_minutes)
                .bind(&cell.note)
                .bind(cell.day)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            written += 1;
        }
        tx.commit().await.map_err(db_error)?;
        Ok(written)
    }

    /// Writes the cells of a generated plan in one statement.
    pub async fn write_shifts(
        &self,
        cells: &[(i64, NaiveDate, i64)],
    ) -> Result<usize, DomainError> {
        if cells.is_empty() {
            return Ok(0);
        }
        let employees: Vec<i64> = cells.iter().map(|(employee, _, _)| *employee).collect();
        let days: Vec<NaiveDate> = cells.iter().map(|(_, day, _)| *day).collect();
        let shifts: Vec<i64> = cells.iter().map(|(_, _, shift)| *shift).collect();
        let affected = sqlx::query(INSERT_SHIFTS)
            .bind(&employees)
            .bind(&days)
            .bind(&shifts)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(affected.rows_affected() as usize)
    }

    pub async fn clear(&self, employee_id: i64, day: NaiveDate) -> Result<(), DomainError> {
        sqlx::query(DELETE_ENTRY)
            .bind(employee_id)
            .bind(day)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(())
    }

    /// Writes the same cell onto `series.days` days in a row, the first day
    /// included, and returns them in the order of their days. A shift only
    /// reaches as far as the planning rules allow — the apprentice rule excepted:
    /// a lone apprentice is no reason to stop a series the planner asks for, the
    /// print check warns about it.
    pub async fn set_series(
        &self,
        series: &CleanSeries,
        rules: &PlanningRules,
    ) -> Result<DutySeriesResult, DomainError> {
        if series.kind != KIND_SHIFT && series.kind != KIND_FREE {
            // Sick and vacation keep no working-time rule.
            let entries = self.write_series(series, series.days).await?;
            return Ok(DutySeriesResult {
                entries,
                requested: series.days,
                blocked: None,
            });
        }
        let shift = match series.shift_id {
            Some(shift_id) => self.shift(shift_id).await?,
            None => series.plan_shift().ok_or_else(|| {
                DomainError::Validation("The times of the free series are missing.".to_string())
            })?,
        };
        // A free day breaks no rule and asks for nobody: every day is allowed.
        if let Some(shift_id) = series.shift_id
            && self.is_free_shift(shift_id).await?
        {
            let entries = self.write_series(series, series.days).await?;
            return Ok(DutySeriesResult {
                entries,
                requested: series.days,
                blocked: None,
            });
        }
        // A position that the shift does not have stops the series before its first day.
        if let Some(shift_id) = series.shift_id
            && self
                .excluded_for_shift(series.employee_id, shift_id)
                .await?
                .is_some()
        {
            return Ok(DutySeriesResult {
                entries: Vec::new(),
                requested: series.days,
                blocked: Some(DutyBlocked {
                    day: series.day,
                    reason: BLOCK_EXCLUDED_POSITION.to_string(),
                }),
            });
        }
        // An apprentice shift stays closed for everybody else; an apprentice may take
        // a regular one.
        if let Some(shift_id) = series.shift_id {
            if let Some(error) = self
                .apprentice_mismatch(series.employee_id, shift_id)
                .await?
            {
                return Err(error);
            }
        }
        let planned = self
            .planned_days(series.employee_id, series.day, series.days)
            .await?;
        let to = series
            .day
            .checked_add_days(Days::new(series.days.max(0) as u64))
            .unwrap_or(series.day);
        // A lone apprentice is a warning of the print check, not a stop: the planner
        // often writes the shifts of an apprentice before the generator fills the rest.
        let mut rules = rules.clone();
        rules.apprentice_needs_company = false;
        let company = self
            .company(series.employee_id, series.shift_id, series.day, to)
            .await?;
        let holidays = self.holidays(series.day, to).await?;
        let plan = plan_series_with(
            &rules,
            shift,
            series.day,
            series.days,
            &planned,
            &company,
            &holidays,
        );
        let blocked = plan.blocked.map(|(day, reason)| DutyBlocked {
            day,
            reason: reason.to_string(),
        });
        if plan.allowed == 0 {
            return Ok(DutySeriesResult {
                entries: Vec::new(),
                requested: series.days,
                blocked,
            });
        }
        let entries = self.write_series(series, plan.allowed).await?;
        Ok(DutySeriesResult {
            entries,
            requested: series.days,
            blocked,
        })
    }

    /// The refusal when shift and employee do not belong together.
    ///
    /// The apprentice shifts of the theatre (`F-AZ`, `S-AZ`, `S-WE-AZ`, `BS`) carry
    /// `apprentice_only`; everybody else stays out of them. An apprentice may take a
    /// regular shift as well — the planner often writes those before the generator
    /// fills the rest —, and the print check warns when nobody works it with them.
    async fn apprentice_mismatch(
        &self,
        employee_id: i64,
        shift_id: i64,
    ) -> Result<Option<DomainError>, DomainError> {
        let row = sqlx::query(SELECT_SHIFT_APPRENTICE_ONLY)
            .bind(shift_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        let Some(row) = row else {
            return Ok(None);
        };
        // A free day is no work and belongs to everybody.
        if ShiftBand::parse(&row.get::<String, _>("band")).is_free() {
            return Ok(None);
        }
        let reserved: bool = row.get("apprentice_only");
        if !reserved {
            return Ok(None);
        }
        let apprentice: bool = sqlx::query_scalar(SELECT_APPRENTICE)
            .bind(employee_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok((!apprentice).then(apprentice_only_shift))
    }

    /// `true` when the shift is a regular late shift and the day is a weekend day.
    async fn weekend_refused(&self, shift_id: i64, day: NaiveDate) -> Result<bool, DomainError> {
        if !matches!(day.weekday(), Weekday::Sat | Weekday::Sun) {
            return Ok(false);
        }
        let row = sqlx::query(SELECT_SHIFT)
            .bind(shift_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        let Some(row) = row else {
            return Ok(false);
        };
        let band = ShiftBand::parse(&row.get::<String, _>("band"));
        let short_name: Option<String> = row.get("short_name");
        let name: String = row.get("name");
        Ok(!works_weekend(band, short_name.as_deref(), &name))
    }

    /// The position of the employee if the shift excludes it.
    async fn excluded_for_shift(
        &self,
        employee_id: i64,
        shift_id: i64,
    ) -> Result<Option<String>, DomainError> {
        let position: Option<String> = sqlx::query_scalar(SELECT_EMPLOYEE_POSITION)
            .bind(employee_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "employee",
                id: employee_id as i32,
            })?;
        let Some(position) = position.filter(|value| !value.trim().is_empty()) else {
            return Ok(None);
        };
        let rows = sqlx::query(SELECT_SHIFT_EXCLUSIONS)
            .bind(shift_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let name = position.trim();
        Ok(rows
            .iter()
            .map(|row| row.get::<String, _>("position"))
            .find(|excluded| excluded.trim().eq_ignore_ascii_case(name))
            .map(|excluded| excluded.trim().to_string()))
    }

    /// The shift as the rule check needs it.
    /// `true` when the shift is a whole free day.
    async fn is_free_shift(&self, shift_id: i64) -> Result<bool, DomainError> {
        let band: Option<String> = sqlx::query_scalar(SELECT_FREE_SHIFT)
            .bind(shift_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        match band {
            Some(band) => Ok(ShiftBand::parse(&band).is_free()),
            None => Err(DomainError::NotFound {
                entity: "shift",
                id: shift_id as i32,
            }),
        }
    }

    async fn shift(&self, shift_id: i64) -> Result<PlanShift, DomainError> {
        let row = sqlx::query(SELECT_SHIFT)
            .bind(shift_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        let Some(row) = row else {
            return Err(DomainError::NotFound {
                entity: "shift",
                id: shift_id as i32,
            });
        };
        let starts_at: String = row.get("starts_at");
        let ends_at: String = row.get("ends_at");
        let break_minutes: i16 = row.get("break_minutes");
        let band = ShiftBand::parse(&row.get::<String, _>("band"));
        let short_name: Option<String> = row.get("short_name");
        let name: String = row.get("name");
        let weekend = works_weekend(band, short_name.as_deref(), &name);
        let shift = PlanShift::new(&starts_at, &ends_at, break_minutes).ok_or_else(|| {
            DomainError::Validation("The times of the shift cannot be read.".to_string())
        })?;
        Ok(if weekend {
            shift
        } else {
            shift.without_weekend()
        })
    }

    /// The days around the series that are already planned. A week before and
    /// after is enough for the rest period, the weekly hours and the days off.
    async fn planned_days(
        &self,
        employee_id: i64,
        day: NaiveDate,
        days: i64,
    ) -> Result<PlannedDays, DomainError> {
        let from = day.checked_sub_days(Days::new(7)).unwrap_or(day);
        let to = day
            .checked_add_days(Days::new(days.max(0) as u64 + 7))
            .unwrap_or(day);
        let rows = sqlx::query(SELECT_PLANNED)
            .bind(employee_id)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut planned = PlannedDays::new();
        for row in &rows {
            let kind: String = row.get("kind");
            // A free day is a day off, not a worked shift: it stays out of the
            // check, which otherwise reads its `00:00`–`23:59` as a 24-hour
            // shift and breaks the rest period of the neighbouring days.
            let free = row
                .try_get::<Option<String>, _>("shift_band")
                .ok()
                .flatten()
                .map(|band| ShiftBand::parse(&band).is_free())
                .unwrap_or(false);
            if free {
                continue;
            }
            let shift = match kind.as_str() {
                KIND_FREE => {
                    let starts_at: Option<String> = row.get("starts_at");
                    let ends_at: Option<String> = row.get("ends_at");
                    let break_minutes: Option<i16> = row.get("break_minutes");
                    match (starts_at, ends_at) {
                        (Some(starts_at), Some(ends_at)) => {
                            PlanShift::new(&starts_at, &ends_at, break_minutes.unwrap_or(0))
                        }
                        _ => None,
                    }
                }
                KIND_SHIFT => {
                    let starts_at: Option<String> = row.get("shift_starts_at");
                    let ends_at: Option<String> = row.get("shift_ends_at");
                    let break_minutes: Option<i16> = row.get("shift_break_minutes");
                    match (starts_at, ends_at) {
                        (Some(starts_at), Some(ends_at)) => {
                            PlanShift::new(&starts_at, &ends_at, break_minutes.unwrap_or(0))
                        }
                        _ => None,
                    }
                }
                _ => None,
            };
            planned.insert(row.get("day"), shift);
        }
        Ok(planned)
    }

    /// The other employees between `from` and `to`, and whether the employee is an
    /// apprentice. With a shift the colleagues of that shift count; a free time
    /// has no shift, so any planned colleague of the day does. The days of the
    /// others are only read for an apprentice.
    async fn company(
        &self,
        employee_id: i64,
        shift_id: Option<i64>,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Company, DomainError> {
        let apprentice: bool = sqlx::query_scalar(SELECT_APPRENTICE)
            .bind(employee_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "employee",
                id: employee_id as i32,
            })?;
        let mut company = Company {
            apprentice,
            days: HashSet::new(),
        };
        if !apprentice {
            return Ok(company);
        }
        let rows = match shift_id {
            Some(shift_id) => {
                sqlx::query(SELECT_COMPANY_DAYS)
                    .bind(shift_id)
                    .bind(employee_id)
                    .bind(from)
                    .bind(to)
                    .fetch_all(&self.pool)
                    .await
            }
            None => {
                sqlx::query(SELECT_ANY_COMPANY_DAYS)
                    .bind(employee_id)
                    .bind(from)
                    .bind(to)
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .map_err(db_error)?;
        for row in &rows {
            company.days.insert(row.get("day"));
        }
        Ok(company)
    }

    /// The public holiday days between `from` and `to`.
    async fn holidays(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<HashSet<NaiveDate>, DomainError> {
        HolidayRepository::new(self.pool.clone())
            .days(from, to)
            .await
    }

    /// The insert itself, for the days the rules allowed.
    async fn write_series(
        &self,
        series: &CleanSeries,
        days: i64,
    ) -> Result<Vec<DutyEntry>, DomainError> {
        let rows = sqlx::query(UPSERT_SERIES)
            .bind(series.employee_id)
            .bind(&series.kind)
            .bind(series.shift_id)
            .bind(&series.starts_at)
            .bind(&series.ends_at)
            .bind(series.break_minutes)
            .bind(series.day)
            .bind(days as i32)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut entries: Vec<DutyEntry> = rows.iter().map(row_to_entry).collect();
        entries.sort_by_key(|entry| entry.day);
        Ok(entries)
    }
}
