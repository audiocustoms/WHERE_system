//! Storage of the holidays (`where_people_holidays`,
//! `where_people_school_holidays`).

use std::collections::HashSet;

use chrono::NaiveDate;
use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::holidays::{Holiday, HolidayCalendar, HolidayCounts, SOURCE, SchoolHoliday};

const DELETE_HOLIDAYS: &str =
    "DELETE FROM \"where_people_holidays\" WHERE \"day\" BETWEEN $1 AND $2";

const DELETE_SCHOOL_HOLIDAYS: &str = "DELETE FROM \"where_people_school_holidays\" \
     WHERE \"ends_on\" >= $1 AND \"starts_on\" <= $2";

const INSERT_HOLIDAY: &str = "INSERT INTO \"where_people_holidays\" \
     (\"day\", \"name\", \"scope\", \"state\", \"source\") \
     VALUES ($1, $2, $3, $4, $5) \
     ON CONFLICT (\"day\", \"name\") DO UPDATE SET \
        \"scope\" = excluded.\"scope\", \"state\" = excluded.\"state\", \
        \"source\" = excluded.\"source\", \"fetched_at\" = now()";

const INSERT_SCHOOL_HOLIDAY: &str = "INSERT INTO \"where_people_school_holidays\" \
     (\"starts_on\", \"ends_on\", \"name\", \"state\", \"source\") \
     VALUES ($1, $2, $3, $4, $5) \
     ON CONFLICT (\"starts_on\", \"ends_on\", \"state\") DO UPDATE SET \
        \"name\" = excluded.\"name\", \"source\" = excluded.\"source\", \
        \"fetched_at\" = now()";

const SELECT_HOLIDAYS: &str = "SELECT \"day\", \"name\", \"scope\", \"state\" \
     FROM \"where_people_holidays\" WHERE \"day\" BETWEEN $1 AND $2 \
     ORDER BY \"day\", \"name\"";

const SELECT_SCHOOL_HOLIDAYS: &str = "SELECT \"starts_on\", \"ends_on\", \"name\", \"state\" \
     FROM \"where_people_school_holidays\" WHERE \"ends_on\" >= $1 AND \"starts_on\" <= $2 \
     ORDER BY \"starts_on\", \"name\"";

const COUNT_ALL: &str = "SELECT \
     (SELECT count(*) FROM \"where_people_holidays\") AS \"holidays\", \
     (SELECT count(*) FROM \"where_people_school_holidays\") AS \"school_holidays\"";

/// The days of a year as its first and its last day.
fn year_span(year: i32) -> (NaiveDate, NaiveDate) {
    let first = NaiveDate::from_ymd_opt(year, 1, 1).unwrap_or_default();
    let last = NaiveDate::from_ymd_opt(year, 12, 31).unwrap_or(first);
    (first, last)
}

/// Reads and writes the holiday calendar of the tenant.
#[derive(Clone)]
pub struct HolidayRepository {
    pool: PgPool,
}

impl HolidayRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Holidays and school holidays that touch `from`..`to`.
    pub async fn range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<HolidayCalendar, DomainError> {
        let holiday_rows = sqlx::query(SELECT_HOLIDAYS)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let school_rows = sqlx::query(SELECT_SCHOOL_HOLIDAYS)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(HolidayCalendar {
            holidays: holiday_rows
                .iter()
                .map(|row| Holiday {
                    day: row.get("day"),
                    name: row.get("name"),
                    scope: row.get("scope"),
                    state: row.get("state"),
                })
                .collect(),
            school_holidays: school_rows
                .iter()
                .map(|row| SchoolHoliday {
                    starts_on: row.get("starts_on"),
                    ends_on: row.get("ends_on"),
                    name: row.get("name"),
                    state: row.get("state"),
                })
                .collect(),
        })
    }

    /// The holiday days of a period — what the rule check needs.
    pub async fn days(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<HashSet<NaiveDate>, DomainError> {
        let rows = sqlx::query(SELECT_HOLIDAYS)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .iter()
            .map(|row| row.get::<NaiveDate, &str>("day"))
            .collect())
    }

    /// How many rows are stored — the settings only offer the display switches
    /// for a calendar that was imported.
    pub async fn counts(&self) -> Result<HolidayCounts, DomainError> {
        let row = sqlx::query(COUNT_ALL)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(HolidayCounts {
            holidays: row.get("holidays"),
            school_holidays: row.get("school_holidays"),
        })
    }

    /// Replaces what one import run brought: `holidays` are the public holidays
    /// of `year` that should exist, `school` the school holidays. `None` leaves
    /// the rows of that kind untouched, so a failed source does not wipe them.
    pub async fn replace_year(
        &self,
        year: i32,
        holidays: Option<&[Holiday]>,
        school: Option<&[SchoolHoliday]>,
    ) -> Result<(), DomainError> {
        let (first, last) = year_span(year);
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if let Some(holidays) = holidays {
            sqlx::query(DELETE_HOLIDAYS)
                .bind(first)
                .bind(last)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            for holiday in holidays {
                sqlx::query(INSERT_HOLIDAY)
                    .bind(holiday.day)
                    .bind(&holiday.name)
                    .bind(&holiday.scope)
                    .bind(holiday.state.as_deref())
                    .bind(SOURCE)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
        }
        if let Some(school) = school {
            sqlx::query(DELETE_SCHOOL_HOLIDAYS)
                .bind(first)
                .bind(last)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
            for holiday in school {
                sqlx::query(INSERT_SCHOOL_HOLIDAY)
                    .bind(holiday.starts_on)
                    .bind(holiday.ends_on)
                    .bind(&holiday.name)
                    .bind(&holiday.state)
                    .bind(SOURCE)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
            }
        }
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_year_spans_its_first_and_last_day() {
        let (first, last) = year_span(2026);
        assert_eq!(first.to_string(), "2026-01-01");
        assert_eq!(last.to_string(), "2026-12-31");
    }
}
