//! Settings of the order mask (`img/Admin Menü/Aufträge`).
//!
//! Three small tables carry the lists of the window; the switches and numbers of the
//! `Regeln` and `Zeiten` tabs are options and live in `"where_settings"` (see
//! [`crate::settings_repo`]). WHERE has no order engine yet — a later module reads
//! these rows through the endpoints that already exist.

use sqlx::{PgPool, Row};
use where_domain::DomainError;

use crate::order_settings::{
    HeadingEntry, HeadingInput, MacroInput, OrderMacro, TimelineEntry, TimelineInput,
};

use where_persistence::db_error;

const TIMELINE_SQL: &str = "SELECT \"key\", \"label\", \"position\", \"shown\" \
     FROM \"where_order_timeline\" ORDER BY \"position\"";

const HEADINGS_SQL: &str =
    "SELECT \"number\", \"shown\", \"label\" FROM \"where_order_headings\" ORDER BY \"number\"";

const MACROS_SQL: &str = "SELECT \"id\", \"label\", \"for_sale\", \"active\", \"created_by\", \
     \"body\", \"changed_at\" FROM \"where_order_macros\" ORDER BY \"label\"";

/// Storage of the "Auftragseinstellungen" window.
#[derive(Clone)]
pub struct OrderSettingsRepository {
    pool: PgPool,
}

impl OrderSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The default time plan of a new order, in the order of the window.
    pub async fn timeline(&self) -> Result<Vec<TimelineEntry>, DomainError> {
        let rows = sqlx::query(TIMELINE_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .into_iter()
            .map(|row| TimelineEntry {
                key: row.get("key"),
                label: row.get("label"),
                position: row.get("position"),
                shown: row.get("shown"),
            })
            .collect())
    }

    /// Changes name and visibility of one step.
    pub async fn save_timeline(
        &self,
        key: &str,
        input: TimelineInput,
    ) -> Result<TimelineEntry, DomainError> {
        let (label, shown) = input.prepare()?;
        let changed = sqlx::query(
            "UPDATE \"where_order_timeline\" \
             SET \"label\" = COALESCE($2, \"label\"), \"shown\" = $3 WHERE \"key\" = $1",
        )
        .bind(key)
        .bind(label)
        .bind(shown)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFoundKey {
                entity: "Zeitplan",
                key: key.to_string(),
            });
        }
        self.timeline()
            .await?
            .into_iter()
            .find(|row| row.key == key)
            .ok_or_else(|| DomainError::NotFoundKey {
                entity: "Zeitplan",
                key: key.to_string(),
            })
    }

    /// The nine headings of the order form.
    pub async fn headings(&self) -> Result<Vec<HeadingEntry>, DomainError> {
        let rows = sqlx::query(HEADINGS_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .into_iter()
            .map(|row| HeadingEntry {
                number: row.get("number"),
                shown: row.get("shown"),
                label: row.get("label"),
            })
            .collect())
    }

    /// Changes visibility and text of one heading.
    pub async fn save_heading(
        &self,
        number: i32,
        input: HeadingInput,
    ) -> Result<HeadingEntry, DomainError> {
        let (shown, label) = input.prepare()?;
        let changed = sqlx::query(
            "UPDATE \"where_order_headings\" SET \"shown\" = $2, \"label\" = $3 \
             WHERE \"number\" = $1",
        )
        .bind(number)
        .bind(shown)
        .bind(label)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Überschrift",
                id: number,
            });
        }
        self.headings()
            .await?
            .into_iter()
            .find(|row| row.number == number)
            .ok_or(DomainError::NotFound {
                entity: "Überschrift",
                id: number,
            })
    }

    /// The macro list of the order mask.
    pub async fn macros(&self) -> Result<Vec<OrderMacro>, DomainError> {
        let rows = sqlx::query(MACROS_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows
            .into_iter()
            .map(|row| OrderMacro {
                id: row.get("id"),
                label: row.get("label"),
                for_sale: row.get("for_sale"),
                active: row.get("active"),
                created_by: row.get("created_by"),
                body: row.get("body"),
                changed_at: row.get("changed_at"),
            })
            .collect())
    }

    /// Adds a macro; `user` is booked as its author until WHERE knows logins.
    pub async fn create_macro(
        &self,
        input: MacroInput,
        user: &str,
    ) -> Result<OrderMacro, DomainError> {
        let (label, for_sale, active, body) = input.prepare()?;
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO \"where_order_macros\" \
             (\"label\", \"for_sale\", \"active\", \"created_by\", \"body\") \
             VALUES ($1, $2, $3, $4, $5) RETURNING \"id\"",
        )
        .bind(&label)
        .bind(for_sale)
        .bind(active)
        .bind(user)
        .bind(body)
        .fetch_one(&self.pool)
        .await
        .map_err(db_error)?;
        self.macro_by_id(id).await
    }

    pub async fn update_macro(
        &self,
        id: i64,
        input: MacroInput,
    ) -> Result<OrderMacro, DomainError> {
        let (label, for_sale, active, body) = input.prepare()?;
        let changed = sqlx::query(
            "UPDATE \"where_order_macros\" SET \"label\" = $2, \"for_sale\" = $3, \
             \"active\" = $4, \"body\" = $5, \"changed_at\" = now() WHERE \"id\" = $1",
        )
        .bind(id)
        .bind(&label)
        .bind(for_sale)
        .bind(active)
        .bind(body)
        .execute(&self.pool)
        .await
        .map_err(db_error)?
        .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Makro",
                id: id as i32,
            });
        }
        self.macro_by_id(id).await
    }

    pub async fn delete_macro(&self, id: i64) -> Result<(), DomainError> {
        let changed = sqlx::query("DELETE FROM \"where_order_macros\" WHERE \"id\" = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if changed == 0 {
            return Err(DomainError::NotFound {
                entity: "Makro",
                id: id as i32,
            });
        }
        Ok(())
    }

    async fn macro_by_id(&self, id: i64) -> Result<OrderMacro, DomainError> {
        self.macros()
            .await?
            .into_iter()
            .find(|row| row.id == id)
            .ok_or(DomainError::NotFound {
                entity: "Makro",
                id: id as i32,
            })
    }
}

/// The order module of a later version reads these rows; until then the API is the
/// only consumer. The test keeps the SQL strings to quoted identifiers.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statements_use_quoted_identifiers() {
        for sql in [TIMELINE_SQL, HEADINGS_SQL, MACROS_SQL] {
            assert!(sql.starts_with("SELECT"));
            assert!(sql.contains('"'), "{sql}");
            assert!(!sql.contains('\''), "{sql}");
        }
    }
}
