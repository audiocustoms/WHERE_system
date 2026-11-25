//! Storage of the shifts (`where_people_shifts`).

use std::collections::HashMap;

use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::shifts::{CleanShift, Shift, ShiftBand};
const SELECT_SHIFTS: &str = "SELECT \"id\", \"name\", \"short_name\", \"starts_at\", \
     \"ends_at\", \"break_minutes\", \"band\", \"apprentice_only\", \"color\", \"department\", \"active\", \
     \"sort_order\" \
     FROM \"where_people_shifts\" ORDER BY \"sort_order\", lower(\"name\")";

const INSERT_SHIFT: &str = "INSERT INTO \"where_people_shifts\" \
     (\"name\", \"short_name\", \"starts_at\", \"ends_at\", \"break_minutes\", \
      \"color\", \"department\", \"active\", \"sort_order\", \"band\", \"apprentice_only\") \
     SELECT $1, $2, $3, $4, $5, $6, $7, $8, coalesce(max(\"sort_order\") + 1, 0), $9, $10 \
     FROM \"where_people_shifts\" \
     RETURNING \"id\", \"name\", \"short_name\", \"starts_at\", \"ends_at\", \
     \"break_minutes\", \"band\", \"apprentice_only\", \"color\", \"department\", \
     \"active\", \"sort_order\"";

const UPDATE_SHIFT: &str = "UPDATE \"where_people_shifts\" SET \"name\" = $2, \
     \"short_name\" = $3, \"starts_at\" = $4, \"ends_at\" = $5, \"break_minutes\" = $6, \
     \"color\" = $7, \"active\" = $8, \"band\" = $9, \"apprentice_only\" = $10, \
     \"department\" = $11, \
     \"changed_at\" = now() \
     WHERE \"id\" = $1 \
     RETURNING \"id\", \"name\", \"short_name\", \"starts_at\", \"ends_at\", \
     \"break_minutes\", \"band\", \"apprentice_only\", \"color\", \"department\", \
     \"active\", \"sort_order\"";

const DELETE_SHIFT: &str = "DELETE FROM \"where_people_shifts\" WHERE \"id\" = $1";

const REORDER_SHIFT: &str = "UPDATE \"where_people_shifts\" SET \"sort_order\" = $2, \
     \"changed_at\" = now() WHERE \"id\" = $1";

const SELECT_ONE: &str = "SELECT \"id\", \"name\", \"short_name\", \"starts_at\", \
     \"ends_at\", \"break_minutes\", \"band\", \"apprentice_only\", \"color\", \"department\", \
     \"active\", \"sort_order\" \
     FROM \"where_people_shifts\" WHERE \"id\" = $1";

const SELECT_EXCLUSIONS: &str = "SELECT \"shift_id\", \"position\" \
     FROM \"where_people_shift_excluded_positions\" \
     ORDER BY \"shift_id\", lower(\"position\")";

const SELECT_EXCLUSIONS_ONE: &str = "SELECT \"position\" \
     FROM \"where_people_shift_excluded_positions\" \
     WHERE \"shift_id\" = $1 ORDER BY lower(\"position\")";

const DELETE_EXCLUSIONS: &str =
    "DELETE FROM \"where_people_shift_excluded_positions\" WHERE \"shift_id\" = $1";

const INSERT_EXCLUSION: &str = "INSERT INTO \"where_people_shift_excluded_positions\" \
     (\"shift_id\", \"position\") VALUES ($1, $2) \
     ON CONFLICT (\"shift_id\", \"position\") DO NOTHING";

fn row_to_shift(row: &sqlx::postgres::PgRow) -> Shift {
    Shift {
        id: row.get("id"),
        name: row.get("name"),
        short_name: row.get("short_name"),
        starts_at: row.get("starts_at"),
        ends_at: row.get("ends_at"),
        break_minutes: row.get("break_minutes"),
        band: ShiftBand::parse(&row.get::<String, _>("band")),
        apprentice_only: row.get("apprentice_only"),
        color: row.get("color"),
        department: row.get("department"),
        active: row.get("active"),
        sort_order: row.get("sort_order"),
        excluded_positions: Vec::new(),
    }
}

/// CRUD over the shifts of the duty planning.
#[derive(Clone)]
pub struct ShiftRepository {
    pool: PgPool,
}

impl ShiftRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<Shift>, DomainError> {
        let rows = sqlx::query(SELECT_SHIFTS)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut shifts: Vec<Shift> = rows.iter().map(row_to_shift).collect();
        let exclusions = self.all_exclusions().await?;
        for shift in shifts.iter_mut() {
            if let Some(positions) = exclusions.get(&shift.id) {
                shift.excluded_positions = positions.clone();
            }
        }
        Ok(shifts)
    }

    /// One shift with its excluded positions.
    pub async fn get(&self, id: i64) -> Result<Shift, DomainError> {
        let row = sqlx::query(SELECT_ONE)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))?;
        let mut shift = row_to_shift(&row);
        shift.excluded_positions = self.excluded_positions(id).await?;
        Ok(shift)
    }

    /// The positions a shift excludes, in alphabetical order.
    pub async fn excluded_positions(&self, shift_id: i64) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query(SELECT_EXCLUSIONS_ONE)
            .bind(shift_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.iter().map(|row| row.get("position")).collect())
    }

    /// Replaces the excluded positions of a shift and returns it as it now stands.
    pub async fn set_exclusions(
        &self,
        shift_id: i64,
        positions: &[String],
    ) -> Result<Shift, DomainError> {
        if sqlx::query(SELECT_ONE)
            .bind(shift_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .is_none()
        {
            return Err(self.not_found(shift_id));
        }
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(DELETE_EXCLUSIONS)
            .bind(shift_id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for position in positions {
            sqlx::query(INSERT_EXCLUSION)
                .bind(shift_id)
                .bind(position)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        tx.commit().await.map_err(db_error)?;
        self.get(shift_id).await
    }

    /// Every exclusion, grouped by shift.
    async fn all_exclusions(&self) -> Result<HashMap<i64, Vec<String>>, DomainError> {
        let rows = sqlx::query(SELECT_EXCLUSIONS)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut grouped: HashMap<i64, Vec<String>> = HashMap::new();
        for row in rows.iter() {
            grouped
                .entry(row.get("shift_id"))
                .or_default()
                .push(row.get("position"));
        }
        Ok(grouped)
    }

    pub async fn create(&self, shift: &CleanShift) -> Result<Shift, DomainError> {
        let row = sqlx::query(INSERT_SHIFT)
            .bind(&shift.name)
            .bind(&shift.short_name)
            .bind(&shift.starts_at)
            .bind(&shift.ends_at)
            .bind(shift.break_minutes)
            .bind(&shift.color)
            .bind(&shift.department)
            .bind(shift.active)
            .bind(shift.band.as_str())
            .bind(shift.apprentice_only)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row_to_shift(&row))
    }

    pub async fn update(&self, id: i64, shift: &CleanShift) -> Result<Shift, DomainError> {
        let row = sqlx::query(UPDATE_SHIFT)
            .bind(id)
            .bind(&shift.name)
            .bind(&shift.short_name)
            .bind(&shift.starts_at)
            .bind(&shift.ends_at)
            .bind(shift.break_minutes)
            .bind(&shift.color)
            .bind(shift.active)
            .bind(shift.band.as_str())
            .bind(shift.apprentice_only)
            .bind(&shift.department)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| self.not_found(id))?;
        Ok(row_to_shift(&row))
    }

    pub async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let affected = sqlx::query(DELETE_SHIFT)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if affected == 0 {
            return Err(self.not_found(id));
        }
        Ok(())
    }

    /// Writes the given order and returns the shifts as they now stand.
    pub async fn reorder(&self, ids: &[i64]) -> Result<Vec<Shift>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        for (position, id) in ids.iter().enumerate() {
            let affected = sqlx::query(REORDER_SHIFT)
                .bind(id)
                .bind(position as i32)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?
                .rows_affected();
            if affected == 0 {
                return Err(self.not_found(*id));
            }
        }
        tx.commit().await.map_err(db_error)?;
        self.list().await
    }

    fn not_found(&self, id: i64) -> DomainError {
        DomainError::NotFound {
            entity: "shift",
            id: id as i32,
        }
    }
}
