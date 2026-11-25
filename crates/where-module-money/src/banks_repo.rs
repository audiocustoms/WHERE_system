//! Bank list (`img/Admin Menü/Bank - Swift BIC Liste`).
//!
//! The legacy table `"Storage_Banken"` keeps one row per BIC: the code, the name, the
//! country, the sort code, the town and an alternative name. The original list holds
//! some 43 000 rows, so the search filters in SQL and the page loads in slices.

use std::collections::HashMap;

use sqlx::{PgPool, Row};
use where_domain::DomainError;
use where_persistence::db_error;

use crate::bank::{Bank, BankImportRow, BankInput, BankSyncReport};

const COLUMNS: &str = "\"Primary_Key\"::bigint AS id, \"BIC\" AS bic, \"Name\" AS name, \
     \"Land_ISO3\" AS country, \"BLZ\" AS blz, \"Ort\" AS city, \"AltName\" AS alt_name";

/// A page of the list with the number of matching rows.
#[derive(Debug, Clone)]
pub struct BankPage {
    pub rows: Vec<Bank>,
    pub total: i64,
}

/// Where the next search starts: `where_sql` is the filter without the leading `WHERE`.
fn filter(q: Option<&str>) -> (Option<String>, &'static str) {
    match q.map(str::trim).filter(|text| !text.is_empty()) {
        Some(text) => {
            // The pattern is bound, never concatenated — only the shape is fixed.
            (
                Some(format!("%{}%", escape_like(text))),
                "(\"BIC\" ILIKE $1 OR \"Name\" ILIKE $1 OR \"Ort\" ILIKE $1 \
                 OR \"BLZ\" ILIKE $1 OR \"AltName\" ILIKE $1 OR \"Land_ISO3\" ILIKE $1)",
            )
        }
        None => (None, "TRUE"),
    }
}

/// `%` and `_` of a search text are literals, not wildcards.
fn escape_like(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn row_to_bank(row: &sqlx::postgres::PgRow) -> Bank {
    let bic: Option<String> = row.get("bic");
    let name: Option<String> = row.get("name");
    Bank {
        id: row.get("id"),
        bic: bic.unwrap_or_default(),
        name: name.unwrap_or_default(),
        country: row.get("country"),
        blz: row.get("blz"),
        city: row.get("city"),
        alt_name: row.get("alt_name"),
    }
}

/// Reads and writes the bank list.
#[derive(Clone)]
pub struct BankRepository {
    pool: PgPool,
}

impl BankRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// One page of the list; `q` searches code, name, town, sort code and country.
    pub async fn list(
        &self,
        q: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<BankPage, DomainError> {
        let (pattern, where_sql) = filter(q);
        let rows_sql = format!(
            "SELECT {COLUMNS} FROM \"Storage_Banken\" WHERE {where_sql} \
             ORDER BY \"BIC\" NULLS LAST, \"Name\" NULLS LAST LIMIT $2 OFFSET $3"
        );
        let count_sql =
            format!("SELECT count(*)::bigint AS n FROM \"Storage_Banken\" WHERE {where_sql}");

        let query = sqlx::query(sqlx::AssertSqlSafe(rows_sql)).bind(pattern.clone());
        let rows = query
            .bind(limit.clamp(1, 20_000))
            .bind(offset.max(0))
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(count_sql))
            .bind(pattern)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(BankPage {
            rows: rows.iter().map(row_to_bank).collect(),
            total,
        })
    }

    /// One row by its key.
    pub async fn get(&self, id: i64) -> Result<Bank, DomainError> {
        let sql = format!("SELECT {COLUMNS} FROM \"Storage_Banken\" WHERE \"Primary_Key\" = $1");
        let row = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        row.as_ref().map(row_to_bank).ok_or(DomainError::NotFound {
            entity: "Bank",
            id: id as i32,
        })
    }

    pub async fn create(&self, input: BankInput) -> Result<Bank, DomainError> {
        let bank = input.prepare()?;
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO \"Storage_Banken\" (\"BIC\", \"Name\", \"Land_ISO3\", \"BLZ\", \"Ort\", \"AltName\") \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING \"Primary_Key\"::bigint",
        )
        .bind(&bank.bic)
        .bind(&bank.name)
        .bind(&bank.country)
        .bind(&bank.blz)
        .bind(&bank.city)
        .bind(&bank.alt_name)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(&self, id: i64, input: BankInput) -> Result<Bank, DomainError> {
        let bank = input.prepare()?;
        let changed = sqlx::query(
            "UPDATE \"Storage_Banken\" SET \"BIC\" = $2, \"Name\" = $3, \"Land_ISO3\" = $4, \
             \"BLZ\" = $5, \"Ort\" = $6, \"AltName\" = $7 WHERE \"Primary_Key\" = $1",
        )
        .bind(id)
        .bind(&bank.bic)
        .bind(&bank.name)
        .bind(&bank.country)
        .bind(&bank.blz)
        .bind(&bank.city)
        .bind(&bank.alt_name)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Bank",
                id: id as i32,
            });
        }
        self.get(id).await
    }

    pub async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let changed = sqlx::query("DELETE FROM \"Storage_Banken\" WHERE \"Primary_Key\" = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Bank",
                id: id as i32,
            });
        }
        Ok(())
    }

    /// Writes a parsed list. A row is identified by its BIC and sort code: an existing
    /// row is updated, a new one added, an identical one only counted.
    ///
    /// The existing list is read once and the new rows are inserted in blocks — a
    /// complete bank file has some 15 000 lines, and one round trip per line would keep
    /// the window waiting for minutes.
    pub async fn upsert(&self, rows: Vec<BankImportRow>) -> Result<BankSyncReport, DomainError> {
        // In the source a bank appears once per branch (same BIC, same sort code,
        // different post code). The first entry is the head office and wins.
        let mut einmalig: HashMap<(String, String), BankImportRow> = HashMap::new();
        for row in rows.iter() {
            einmalig
                .entry((row.bic.clone(), row.blz.clone().unwrap_or_default()))
                .or_insert_with(|| row.clone());
        }
        let mut report = BankSyncReport {
            read: rows.len() as u32,
            unchanged: (rows.len() - einmalig.len()) as u32,
            ..BankSyncReport::default()
        };
        type Known = (
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let mut known: HashMap<(String, String), Known> = HashMap::new();
        let existing = sqlx::query(
            "SELECT \"Primary_Key\"::bigint AS id, \"BIC\" AS bic, COALESCE(\"BLZ\", '') AS blz, \
             \"Name\" AS name, \"Land_ISO3\" AS land, \"Ort\" AS ort, \"AltName\" AS alt \
             FROM \"Storage_Banken\"",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db_error)?;
        for row in existing {
            let id: i64 = row.get("id");
            let bic: Option<String> = row.get("bic");
            let blz: String = row.get("blz");
            known.insert(
                (bic.unwrap_or_default(), blz),
                (
                    id,
                    row.get("name"),
                    row.get("land"),
                    row.get("ort"),
                    row.get("alt"),
                ),
            );
        }

        let mut transaction = self.pool.begin().await.map_err(db_error)?;
        let mut inserts: Vec<BankImportRow> = Vec::new();
        for row in einmalig.into_values() {
            let key = (row.bic.clone(), row.blz.clone().unwrap_or_default());
            match known.get(&key) {
                Some((_id, name, country, city, alt_name))
                    if name.as_deref() == Some(row.name.as_str())
                        && country == &row.country
                        && city == &row.city
                        && alt_name == &row.alt_name =>
                {
                    report.unchanged += 1;
                }
                Some((id, ..)) => {
                    sqlx::query(
                        "UPDATE \"Storage_Banken\" SET \"Name\" = $2, \"Land_ISO3\" = $3, \
                         \"Ort\" = $4, \"AltName\" = $5 WHERE \"Primary_Key\" = $1",
                    )
                    .bind(id)
                    .bind(&row.name)
                    .bind(&row.country)
                    .bind(&row.city)
                    .bind(&row.alt_name)
                    .execute(&mut *transaction)
                    .await
                    .map_err(db_error)?;
                    known.insert(
                        key,
                        (
                            *id,
                            Some(row.name.clone()),
                            row.country.clone(),
                            row.city.clone(),
                            row.alt_name.clone(),
                        ),
                    );
                    report.updated += 1;
                }
                None => {
                    known.insert(
                        key,
                        (
                            0,
                            Some(row.name.clone()),
                            row.country.clone(),
                            row.city.clone(),
                            row.alt_name.clone(),
                        ),
                    );
                    inserts.push(row);
                }
            }
        }

        for block in inserts.chunks(500) {
            let mut builder = sqlx::QueryBuilder::new(
                "INSERT INTO \"Storage_Banken\" (\"BIC\", \"Name\", \"Land_ISO3\", \"BLZ\", \"Ort\", \"AltName\") ",
            );
            builder.push_values(block.iter(), |mut values, row| {
                values
                    .push_bind(&row.bic)
                    .push_bind(&row.name)
                    .push_bind(&row.country)
                    .push_bind(&row.blz)
                    .push_bind(&row.city)
                    .push_bind(&row.alt_name);
            });
            builder
                .build()
                .execute(&mut *transaction)
                .await
                .map_err(db_error)?;
            report.inserted += block.len() as u32;
        }

        transaction.commit().await.map_err(db_error)?;
        Ok(report)
    }

    /// Number of rows in the list.
    pub async fn count(&self) -> Result<i64, DomainError> {
        sqlx::query_scalar("SELECT count(*)::bigint FROM \"Storage_Banken\"")
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_search_text_is_escaped() {
        assert_eq!(escape_like("50%"), "50\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        let (pattern, _) = filter(Some("  Aareal  "));
        assert_eq!(pattern.as_deref(), Some("%Aareal%"));
        let (pattern, where_sql) = filter(Some("   "));
        assert_eq!(pattern, None);
        assert_eq!(where_sql, "TRUE");
    }
}
