//! Storage of the staffing demand (`where_people_duty_demand`).

use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::demand::{CleanDemandEntry, DayDemand, DemandEntry};

const SELECT_RANGE: &str = "SELECT \"day\", \"shift_id\", \"position\", \"location\", \
     \"needed\" FROM \"where_people_duty_demand\" \
     WHERE \"day\" BETWEEN $1 AND $2 \
     ORDER BY \"day\", \"shift_id\", \"position\", \"location\"";

const DELETE_DAY: &str = "DELETE FROM \"where_people_duty_demand\" WHERE \"day\" = $1";

const INSERT_ENTRY: &str = "INSERT INTO \"where_people_duty_demand\" \
     (\"day\", \"shift_id\", \"position\", \"location\", \"needed\") \
     VALUES ($1, $2, $3, $4, $5) \
     ON CONFLICT (\"day\", \"shift_id\", \"position\", \"location\") DO UPDATE SET \
        \"needed\" = excluded.\"needed\", \"changed_at\" = now()";

/// Reads and writes the demand per day, shift and position.
#[derive(Clone)]
pub struct DemandRepository {
    pool: PgPool,
}

impl DemandRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The demand of the period, one row per day that needs somebody.
    pub async fn range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DayDemand>, DomainError> {
        let rows = sqlx::query(SELECT_RANGE)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let mut days: Vec<DayDemand> = Vec::new();
        for row in rows.iter() {
            let day: NaiveDate = row.get("day");
            let entry = DemandEntry {
                shift_id: row.get("shift_id"),
                position: row.get("position"),
                location: row.get("location"),
                needed: row.get("needed"),
            };
            match days.last_mut() {
                Some(last) if last.day == day => last.entries.push(entry),
                _ => days.push(DayDemand {
                    day,
                    entries: vec![entry],
                }),
            }
        }
        Ok(days)
    }

    /// The demand of one day.
    pub async fn day(&self, day: NaiveDate) -> Result<DayDemand, DomainError> {
        let mut days = self.range(day, day).await?;
        Ok(days.pop().unwrap_or(DayDemand {
            day,
            entries: Vec::new(),
        }))
    }

    /// Replaces the whole day; an empty list clears it.
    pub async fn replace(
        &self,
        day: NaiveDate,
        entries: &[CleanDemandEntry],
    ) -> Result<DayDemand, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(DELETE_DAY)
            .bind(day)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for entry in entries {
            sqlx::query(INSERT_ENTRY)
                .bind(day)
                .bind(entry.shift_id)
                .bind(&entry.position)
                .bind(&entry.location)
                .bind(entry.needed)
                .execute(&mut *tx)
                .await
                .map_err(|error| match &error {
                    sqlx::Error::Database(database) if database.is_foreign_key_violation() => {
                        DomainError::NotFound {
                            entity: "shift",
                            id: entry.shift_id as i32,
                        }
                    }
                    _ => db_error(error),
                })?;
        }
        tx.commit().await.map_err(db_error)?;
        self.day(day).await
    }

    /// Writes the demand of `from` onto every given day; returns how many days
    /// were written.
    pub async fn copy(&self, from: NaiveDate, days: &[NaiveDate]) -> Result<usize, DomainError> {
        if days.is_empty() {
            return Ok(0);
        }
        let source = self.day(from).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        for day in days {
            sqlx::query(DELETE_DAY)
                .bind(day)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            for entry in &source.entries {
                sqlx::query(INSERT_ENTRY)
                    .bind(day)
                    .bind(entry.shift_id)
                    .bind(&entry.position)
                    .bind(&entry.location)
                    .bind(entry.needed)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
        }
        tx.commit().await.map_err(db_error)?;
        Ok(days.len())
    }
}
