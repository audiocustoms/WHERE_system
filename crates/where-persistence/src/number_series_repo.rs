//! Number series: central allocation of running numbers (articles, customers, documents).
//!
//! One row per series in `where_number_series` (migration `1100_number_series.sql`).
//! The legacy application spreads the same task over twelve tables and a format string;
//! the mapping is recorded in `legacy_source`.

use chrono::{Datelike, NaiveDate, Utc};
use sqlx::{PgConnection, PgPool};
use where_domain::DomainError;

use crate::error::db_error;

/// Allowed placeholders of the format. Order = order in the error message.
pub const FORMAT_PLACEHOLDERS: [&str; 4] = ["{number}", "{year}", "{month}", "{day}"];

/// A number series with its counter and a preview of the next number.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NumberSeriesEntry {
    pub key: String,
    pub name: String,
    pub prefix: String,
    pub format: String,
    pub digits: i16,
    pub start_value: i64,
    pub step: i64,
    pub current_value: i64,
    pub yearly_reset: bool,
    pub year: Option<i32>,
    pub active: bool,
    pub legacy_source: Option<String>,
    pub changed_at: chrono::DateTime<Utc>,
}

impl NumberSeriesEntry {
    /// Next value without consuming the counter.
    pub fn next_value(&self, today: NaiveDate) -> Option<i64> {
        if !self.active {
            return None;
        }
        Some(
            if self.current_value == 0 || (self.yearly_reset && self.year != Some(today.year())) {
                self.start_value
            } else {
                self.current_value + self.step
            },
        )
    }

    /// Next number as text without consuming the counter.
    pub fn preview(&self, today: NaiveDate) -> Option<String> {
        let value = self.next_value(today)?;
        Some(format(
            &self.prefix,
            &self.format,
            self.digits,
            value,
            today,
        ))
    }

    /// Preview using today's date: (value, formatted number).
    pub fn preview_today(&self) -> Option<(i64, String)> {
        let today = Utc::now().date_naive();
        let value = self.next_value(today)?;
        Some((
            value,
            format(&self.prefix, &self.format, self.digits, value, today),
        ))
    }
}

/// Substitutes the placeholders. Unknown placeholders are left alone —
/// storage does not allow them anyway.
pub fn format(prefix: &str, format: &str, digits: i16, value: i64, today: NaiveDate) -> String {
    let number = if digits > 0 {
        format!("{value:0width$}", width = digits as usize)
    } else {
        value.to_string()
    };
    let text = format
        .replace("{number}", &number)
        .replace("{year}", &today.year().to_string())
        .replace("{month}", &format!("{:02}", today.month()))
        .replace("{day}", &format!("{:02}", today.day()));
    format!("{prefix}{text}")
}

const LIST_SQL: &str = r#"
SELECT "key", "name", "prefix", "format", "digits",
       "start_value", "step", "current_value", "yearly_reset", "year",
       "active", "legacy_source", "changed_at"
  FROM "where_number_series"
 ORDER BY "name""#;

const GET_SQL: &str = r#"
SELECT "key", "name", "prefix", "format", "digits",
       "start_value", "step", "current_value", "yearly_reset", "year",
       "active", "legacy_source", "changed_at"
  FROM "where_number_series"
 WHERE "key" = $1"#;

const UPDATE_SQL: &str = r#"
UPDATE "where_number_series"
   SET "name" = $2,
       "prefix" = $3,
       "format" = $4,
       "digits" = $5,
       "start_value" = $6,
       "step" = $7,
       "yearly_reset" = $8,
       "active" = $9,
       "current_value" = $10,
       "changed_at" = now()
 WHERE "key" = $1
RETURNING "key""#;

const RESET_SQL: &str = r#"
UPDATE "where_number_series"
   SET "current_value" = 0,
       "year" = NULL,
       "changed_at" = now()
 WHERE "key" = $1
RETURNING "key""#;

/// Consumes the next number. `$2` is the current year; with
/// `yearly_reset` the series restarts at `start_value` in a new year.
const RESERVE_SQL: &str = r#"
UPDATE "where_number_series"
   SET "current_value" = CASE
         WHEN "yearly_reset" AND "year" IS DISTINCT FROM $2 THEN "start_value"
         WHEN "current_value" = 0 THEN "start_value"
         ELSE "current_value" + "step"
       END,
       "year" = $2,
       "changed_at" = now()
 WHERE "key" = $1 AND "active"
RETURNING "current_value" AS value, "prefix", "format", "digits""#;

/// Moves a series up to a number that was assigned by hand.
const ALIGN_SQL: &str = r#"
UPDATE "where_number_series"
   SET "current_value" = GREATEST("current_value", $2),
       "year" = CASE WHEN "yearly_reset" THEN $3 ELSE "year" END,
       "changed_at" = now()
 WHERE "key" = $1"#;

#[derive(Clone)]
pub struct NumberSeriesRepository {
    pool: PgPool,
}

/// Consumed number, already formatted.
#[derive(Debug, Clone)]
pub struct Reservation {
    pub value: i64,
    pub text: String,
}

impl NumberSeriesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<NumberSeriesEntry>, DomainError> {
        sqlx::query_as::<_, NumberSeriesEntry>(LIST_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn get(&self, key: &str) -> Result<NumberSeriesEntry, DomainError> {
        sqlx::query_as::<_, NumberSeriesEntry>(GET_SQL)
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or_else(|| DomainError::NotFoundKey {
                entity: "Number series",
                key: key.to_string(),
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        key: &str,
        name: &str,
        prefix: &str,
        format: &str,
        digits: i16,
        start_value: i64,
        step: i64,
        yearly_reset: bool,
        active: bool,
        current_value: i64,
    ) -> Result<NumberSeriesEntry, DomainError> {
        Self::validate_input(
            name,
            prefix,
            format,
            digits,
            start_value,
            step,
            current_value,
        )?;

        let exists: Option<(String,)> = sqlx::query_as(UPDATE_SQL)
            .bind(key)
            .bind(name.trim())
            .bind(prefix)
            .bind(format)
            .bind(digits)
            .bind(start_value)
            .bind(step)
            .bind(yearly_reset)
            .bind(active)
            .bind(current_value)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if exists.is_none() {
            return Err(DomainError::NotFoundKey {
                entity: "Number series",
                key: key.to_string(),
            });
        }
        self.get(key).await
    }

    /// Resets the counter; the next issued number is `start_value` again.
    pub async fn reset(&self, key: &str) -> Result<NumberSeriesEntry, DomainError> {
        let exists: Option<(String,)> = sqlx::query_as(RESET_SQL)
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if exists.is_none() {
            return Err(DomainError::NotFoundKey {
                entity: "Number series",
                key: key.to_string(),
            });
        }
        self.get(key).await
    }

    /// Consumes the next number of the series.
    pub async fn reserve(&self, key: &str) -> Result<Reservation, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let today = Utc::now().date_naive();
        let raw = reserve_on(&mut tx, key, today.year())
            .await
            .map_err(db_error)?
            .ok_or_else(|| {
                DomainError::Conflict(format!(
                    "Number series '{key}' does not exist or is inactive"
                ))
            })?;
        let text = format(&raw.prefix, &raw.format, raw.digits, raw.value, today);
        tx.commit().await.map_err(db_error)?;
        Ok(Reservation {
            value: raw.value,
            text,
        })
    }

    fn validate_input(
        name: &str,
        prefix: &str,
        format: &str,
        digits: i16,
        start_value: i64,
        step: i64,
        current_value: i64,
    ) -> Result<(), DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation("Name must not be empty".into()));
        }
        if name.chars().count() > 120 {
            return Err(DomainError::Validation(
                "Name must not exceed 120 characters".into(),
            ));
        }
        if prefix.chars().count() > 16 {
            return Err(DomainError::Validation(
                "Prefix must not exceed 16 characters".into(),
            ));
        }
        if !(0..=12).contains(&digits) {
            return Err(DomainError::Validation(
                "Digits must be between 0 and 12".into(),
            ));
        }
        if step < 1 {
            return Err(DomainError::Validation("Step must be at least 1".into()));
        }
        if start_value < 0 || current_value < 0 {
            return Err(DomainError::Validation(
                "Start value and current value must not be negative".into(),
            ));
        }

        let mut rest = format.to_string();
        for placeholder in FORMAT_PLACEHOLDERS {
            rest = rest.replace(placeholder, "");
        }
        if rest.contains('{') || rest.contains('}') {
            return Err(DomainError::Validation(format!(
                "Unknown placeholder in the format — allowed are {}",
                FORMAT_PLACEHOLDERS.join(", ")
            )));
        }
        if !format.contains("{number}") {
            return Err(DomainError::Validation(
                "Format must contain the placeholder {number}".into(),
            ));
        }
        Ok(())
    }
}

/// Raw values of a reservation as the database returns them.
pub struct RawNumber {
    pub value: i64,
    pub prefix: String,
    pub format: String,
    pub digits: i16,
}

/// Consumes a number on any connection so a caller
/// (for example creating an article or an employee) can use the same transaction.
pub async fn reserve_on(
    conn: &mut PgConnection,
    key: &str,
    year: i32,
) -> sqlx::Result<Option<RawNumber>> {
    let row: Option<(i64, String, String, i16)> = sqlx::query_as(RESERVE_SQL)
        .bind(key)
        .bind(year)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(row.map(|(value, prefix, format, digits)| RawNumber {
        value,
        prefix,
        format,
        digits,
    }))
}

/// Moves a series up to a number that was assigned by hand.
///
/// A mask always sends the number it was offered; without this the series stays behind
/// and offers the same — now taken — number again on the next form.
pub async fn align_on(
    conn: &mut PgConnection,
    key: &str,
    value: i32,
    year: i32,
) -> sqlx::Result<()> {
    sqlx::query(ALIGN_SQL)
        .bind(key)
        .bind(i64::from(value))
        .bind(year)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 3, 7).unwrap()
    }

    #[test]
    fn format_without_digits() {
        assert_eq!(format("", "{number}", 0, 42, tag()), "42");
    }

    #[test]
    fn format_with_prefix_and_digits() {
        assert_eq!(format("RE-", "{number}", 5, 42, tag()), "RE-00042");
    }

    #[test]
    fn format_with_date() {
        assert_eq!(
            format("", "{year}-{month}-{day}/{number}", 0, 7, tag()),
            "2026-03-07/7"
        );
    }

    #[test]
    fn preview_first_number_is_start_value() {
        let entry = NumberSeriesEntry {
            key: "artikel".into(),
            name: "Artikelnummer".into(),
            prefix: String::new(),
            format: "{number}".into(),
            digits: 0,
            start_value: 1000,
            step: 1,
            current_value: 0,
            yearly_reset: false,
            year: None,
            active: true,
            legacy_source: None,
            changed_at: Utc::now(),
        };
        assert_eq!(entry.preview(tag()).as_deref(), Some("1000"));
    }

    #[test]
    fn preview_continues_counting() {
        let entry = NumberSeriesEntry {
            current_value: 1001,
            start_value: 1000,
            ..entry()
        };
        assert_eq!(entry.preview(tag()).as_deref(), Some("1002"));
    }

    #[test]
    fn preview_with_yearly_reset() {
        let entry = NumberSeriesEntry {
            yearly_reset: true,
            year: Some(2025),
            current_value: 900,
            start_value: 1,
            format: "{year}-{number}".into(),
            ..entry()
        };
        assert_eq!(entry.preview(tag()).as_deref(), Some("2026-1"));
    }

    #[test]
    fn preview_continues_within_the_same_year() {
        let entry = NumberSeriesEntry {
            yearly_reset: true,
            year: Some(2026),
            current_value: 12,
            start_value: 1,
            format: "{year}-{number}".into(),
            ..entry()
        };
        assert_eq!(entry.preview(tag()).as_deref(), Some("2026-13"));
    }

    #[test]
    fn inactive_series_has_no_preview() {
        let entry = NumberSeriesEntry {
            active: false,
            ..entry()
        };
        assert_eq!(entry.preview(tag()), None);
    }

    fn entry() -> NumberSeriesEntry {
        NumberSeriesEntry {
            key: "artikel".into(),
            name: "Artikelnummer".into(),
            prefix: String::new(),
            format: "{number}".into(),
            digits: 0,
            start_value: 1000,
            step: 1,
            current_value: 0,
            yearly_reset: false,
            year: None,
            active: true,
            legacy_source: None,
            changed_at: Utc::now(),
        }
    }
}
