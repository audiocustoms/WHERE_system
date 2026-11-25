//! Storage of the disposition: the locations and their entries.

use chrono::{NaiveDate, NaiveTime};
use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::dispo::{CleanDispoEntry, DispoEntry, DispoImportReport, DispoScan};

const SELECT_RANGE: &str = "SELECT e.\"id\", e.\"location_id\", e.\"day\", e.\"starts_at\", \
     e.\"title\", f.\"code\" AS \"flag_code\", e.\"sort_order\", e.\"ignored\" \
     FROM \"where_people_dispo_entries\" e \
     JOIN \"where_people_locations\" l ON l.\"id\" = e.\"location_id\" \
     LEFT JOIN \"where_people_flags\" f ON f.\"id\" = e.\"flag_id\" \
     WHERE e.\"day\" BETWEEN $1 AND $2 \
     ORDER BY l.\"sort_order\", l.\"id\", e.\"day\", e.\"sort_order\", e.\"id\"";

const SELECT_DAY_ENTRIES: &str = "SELECT e.\"id\", e.\"location_id\", e.\"day\", \
     e.\"starts_at\", e.\"title\", f.\"code\" AS \"flag_code\", e.\"sort_order\", e.\"ignored\" \
     FROM \"where_people_dispo_entries\" e \
     LEFT JOIN \"where_people_flags\" f ON f.\"id\" = e.\"flag_id\" \
     WHERE e.\"location_id\" = $1 AND e.\"day\" = $2 ORDER BY e.\"sort_order\", e.\"id\"";

const INSERT_ENTRY: &str = "INSERT INTO \"where_people_dispo_entries\" \
     (\"location_id\", \"day\", \"starts_at\", \"title\", \"flag_id\", \"sort_order\", \"ignored\") \
     VALUES ($1, $2, $3, $4, $5, $6, $7)";

const DELETE_DAY: &str = "DELETE FROM \"where_people_dispo_entries\" \
     WHERE \"location_id\" = $1 AND \"day\" = $2";

const DELETE_RANGE: &str =
    "DELETE FROM \"where_people_dispo_entries\" WHERE \"day\" BETWEEN $1 AND $2";

const SELECT_ONE: &str = "SELECT e.\"id\", e.\"location_id\", e.\"day\", e.\"starts_at\", \
     e.\"title\", f.\"code\" AS \"flag_code\", e.\"sort_order\", e.\"ignored\" \
     FROM \"where_people_dispo_entries\" e \
     LEFT JOIN \"where_people_flags\" f ON f.\"id\" = e.\"flag_id\" \
     WHERE e.\"id\" = $1";

const SET_IGNORED: &str = "UPDATE \"where_people_dispo_entries\" \
     SET \"ignored\" = $2, \"changed_at\" = now() WHERE \"id\" = $1";

const SELECT_LOCATION: &str = "SELECT \"id\" FROM \"where_people_locations\" \
     WHERE lower(btrim(\"name\")) = lower(btrim($1)) AND \"is_text\" = false";

const INSERT_LOCATION: &str = "INSERT INTO \"where_people_locations\" (\"name\", \"sort_order\") \
     SELECT $1, coalesce(max(\"sort_order\") + 1, 0) FROM \"where_people_locations\" \
     RETURNING \"id\"";

const SELECT_LOCATION_EXISTS: &str = "SELECT 1 FROM \"where_people_locations\" WHERE \"id\" = $1";

/// The entries of a period with the names the duplicate check compares.
const KEYS_RANGE: &str = "SELECT l.\"name\" AS \"location\", e.\"day\", e.\"starts_at\", \
     e.\"title\", f.\"code\" AS \"flag_code\" \
     FROM \"where_people_dispo_entries\" e \
     JOIN \"where_people_locations\" l ON l.\"id\" = e.\"location_id\" \
     LEFT JOIN \"where_people_flags\" f ON f.\"id\" = e.\"flag_id\" \
     WHERE e.\"day\" BETWEEN $1 AND $2";

const COUNT_RANGE: &str = "SELECT count(*) AS \"n\" FROM \"where_people_dispo_entries\" \
     WHERE \"day\" BETWEEN $1 AND $2";

const SELECT_IMPORTED: &str = "SELECT 1 FROM \"where_people_dispo_imports\" \
     WHERE \"period_start\" = $1 AND \"fingerprint\" = $2";

const UPSERT_IMPORT: &str = "INSERT INTO \"where_people_dispo_imports\" \
     (\"period_start\", \"period_end\", \"fingerprint\", \"locations\", \"entries\") \
     VALUES ($1, $2, $3, $4, $5) \
     ON CONFLICT (\"period_start\", \"fingerprint\") DO UPDATE \
     SET \"imported_at\" = now(), \"locations\" = excluded.\"locations\", \
     \"entries\" = excluded.\"entries\"";

const SELECT_FLAG: &str = "SELECT \"id\" FROM \"where_people_flags\" \
     WHERE lower(btrim(\"code\")) = lower(btrim($1))";

const SELECT_ALL_FLAGS: &str = "SELECT \"id\", \"code\" FROM \"where_people_flags\"";

/// `09:00` → the time the database keeps.
fn to_time(text: &Option<String>) -> Option<NaiveTime> {
    text.as_deref()
        .and_then(|text| NaiveTime::parse_from_str(text, "%H:%M").ok())
}

/// The time of a row as `HH:MM`.
fn from_time(time: Option<NaiveTime>) -> Option<String> {
    time.map(|time| time.format("%H:%M").to_string())
}

fn row_to_entry(row: &sqlx::postgres::PgRow) -> DispoEntry {
    let starts_at: Option<NaiveTime> = row.get("starts_at");
    DispoEntry {
        id: row.get("id"),
        location_id: row.get("location_id"),
        day: row.get("day"),
        starts_at: from_time(starts_at),
        title: row.get("title"),
        flag: row.get("flag_code"),
        sort_order: row.get("sort_order"),
        ignored: row.get("ignored"),
    }
}

/// Reads and writes the entries of the disposition.
#[derive(Clone)]
pub struct DispoRepository {
    pool: PgPool,
}

impl DispoRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The entries of a period, by location order and then by day.
    pub async fn range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<DispoEntry>, DomainError> {
        let rows = sqlx::query(SELECT_RANGE)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.iter().map(row_to_entry).collect())
    }

    /// Sets an entry aside — or puts it back into the plan — and returns it.
    ///
    /// An entry that is set aside stays in the disposition, but it asks for nobody
    /// and no longer marks its day as unclear.
    pub async fn set_ignored(
        &self,
        id: i64,
        ignored: bool,
    ) -> Result<Option<DispoEntry>, DomainError> {
        let changed = sqlx::query(SET_IGNORED)
            .bind(id)
            .bind(ignored)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        if changed.rows_affected() == 0 {
            return Ok(None);
        }
        let row = sqlx::query(SELECT_ONE)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(Some(row_to_entry(&row)))
    }

    /// How many entries a period already holds.
    pub async fn count_range(&self, from: NaiveDate, to: NaiveDate) -> Result<usize, DomainError> {
        let count: i64 = sqlx::query_scalar(COUNT_RANGE)
            .bind(from)
            .bind(to)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(count.max(0) as usize)
    }

    /// Removes every entry of a period and reports how many rows that were.
    pub async fn clear_range(&self, from: NaiveDate, to: NaiveDate) -> Result<usize, DomainError> {
        let affected = sqlx::query(DELETE_RANGE)
            .bind(from)
            .bind(to)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(affected.rows_affected() as usize)
    }

    /// `true` when a disposition with this content was imported for the month.
    pub async fn fingerprint_seen(
        &self,
        from: NaiveDate,
        fingerprint: &str,
    ) -> Result<bool, DomainError> {
        let row = sqlx::query(SELECT_IMPORTED)
            .bind(from)
            .bind(fingerprint)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row.is_some())
    }

    /// `true` when the location exists.
    pub async fn has_location(&self, location_id: i64) -> Result<bool, DomainError> {
        let row = sqlx::query(SELECT_LOCATION_EXISTS)
            .bind(location_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row.is_some())
    }

    /// Replaces the entries of one location on one day and returns them.
    pub async fn replace_day(
        &self,
        location_id: i64,
        day: NaiveDate,
        entries: &[CleanDispoEntry],
    ) -> Result<Vec<DispoEntry>, DomainError> {
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        sqlx::query(DELETE_DAY)
            .bind(location_id)
            .bind(day)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        for (position, entry) in entries.iter().enumerate() {
            let flag_id = match &entry.flag {
                Some(code) => {
                    let known: Option<i64> = sqlx::query_scalar(SELECT_FLAG)
                        .bind(code)
                        .fetch_optional(&mut *tx)
                        .await
                        .map_err(db_error)?;
                    Some(known.ok_or_else(|| {
                        DomainError::Validation(format!("Unknown flag “{code}”."))
                    })?)
                }
                None => None,
            };
            sqlx::query(INSERT_ENTRY)
                .bind(location_id)
                .bind(day)
                .bind(to_time(&entry.starts_at))
                .bind(&entry.title)
                .bind(flag_id)
                .bind(position as i32)
                .bind(entry.ignored)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }
        tx.commit().await.map_err(db_error)?;
        let rows = sqlx::query(SELECT_DAY_ENTRIES)
            .bind(location_id)
            .bind(day)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.iter().map(row_to_entry).collect())
    }

    /// The entries of a period as the duplicate check compares them.
    pub async fn keys_range(
        &self,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<crate::dispo::DispoKey>, DomainError> {
        let rows = sqlx::query(KEYS_RANGE)
            .bind(from)
            .bind(to)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .iter()
            .map(|row| crate::dispo::DispoKey {
                location: row.get("location"),
                day: row.get("day"),
                starts_at: row
                    .get::<Option<chrono::NaiveTime>, _>("starts_at")
                    .map(|time| time.format("%H:%M").to_string()),
                title: row.get("title"),
                flag: row.get("flag_code"),
            })
            .collect())
    }

    /// Writes a scanned export: a location per block (created when unknown) and
    /// one entry per event. `replace` empties the month first.
    ///
    /// The same disposition is imported once: a repeat without `replace` is a
    /// conflict, because it would double every entry of the month. A month that
    /// no longer holds entries (they were deleted) takes the file again.
    pub async fn import(
        &self,
        scan: &DispoScan,
        from: NaiveDate,
        to: NaiveDate,
        replace: bool,
        fingerprint: &str,
    ) -> Result<DispoImportReport, DomainError> {
        // Read before the transaction opens: the pool hands the checks their own
        // connection, and one held open here would wait for it.
        let existing = if replace {
            Vec::new()
        } else {
            self.keys_range(from, to).await?
        };
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        if !replace {
            let written: i64 = sqlx::query_scalar(COUNT_RANGE)
                .bind(from)
                .bind(to)
                .fetch_one(&mut *tx)
                .await
                .map_err(db_error)?;
            let seen = sqlx::query(SELECT_IMPORTED)
                .bind(from)
                .bind(fingerprint)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_error)?;
            if written > 0 && seen.is_some() {
                return Err(DomainError::Conflict(format!(
                    "This disposition was already imported for {}. Set `replace` to import it again.",
                    from.format("%B %Y")
                )));
            }
            if written > 0 {
                // A sheet that stands in the month already would only double it. The
                // fingerprint above misses that when the month came from another import.
                if crate::dispo::adds_nothing(scan, &existing) {
                    return Err(DomainError::Conflict(format!(
                        "This disposition already stands in {}. Set `replace` to write it again.",
                        from.format("%B %Y")
                    )));
                }
            }
        }
        if replace {
            sqlx::query(DELETE_RANGE)
                .bind(from)
                .bind(to)
                .execute(&mut *tx)
                .await
                .map_err(db_error)?;
        }

        let mut locations_created = 0;
        let mut entries_created = 0;
        let mut flag_ids: std::collections::BTreeMap<String, i64> =
            std::collections::BTreeMap::new();
        for row in sqlx::query(SELECT_ALL_FLAGS)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_error)?
        {
            let code: String = row.get("code");
            flag_ids.insert(code.to_lowercase(), row.get("id"));
        }
        for location in &scan.locations {
            let known: Option<i64> = sqlx::query_scalar(SELECT_LOCATION)
                .bind(&location.name)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_error)?;
            let location_id = match known {
                Some(id) => id,
                None => {
                    let id: i64 = sqlx::query_scalar(INSERT_LOCATION)
                        .bind(&location.name)
                        .fetch_one(&mut *tx)
                        .await
                        .map_err(db_error)?;
                    locations_created += 1;
                    id
                }
            };
            let mut position = 0i32;
            let mut last_day: Option<NaiveDate> = None;
            for event in &location.events {
                if last_day != Some(event.day) {
                    position = 0;
                    last_day = Some(event.day);
                }
                let flag_id = event
                    .flag
                    .as_ref()
                    .and_then(|code| flag_ids.get(&code.to_lowercase()).copied());
                sqlx::query(INSERT_ENTRY)
                    .bind(location_id)
                    .bind(event.day)
                    .bind(to_time(&event.starts_at))
                    .bind(&event.title)
                    .bind(flag_id)
                    .bind(position)
                    .bind(event.ignored)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_error)?;
                position += 1;
                entries_created += 1;
            }
        }
        sqlx::query(UPSERT_IMPORT)
            .bind(from)
            .bind(to)
            .bind(fingerprint)
            .bind(scan.locations.len() as i32)
            .bind(entries_created as i32)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        tx.commit().await.map_err(db_error)?;

        Ok(DispoImportReport {
            locations: scan.locations.len(),
            locations_created,
            flags_created: 0,
            days: scan.days,
            entries_created,
            warnings: scan.warnings.clone(),
        })
    }
}
