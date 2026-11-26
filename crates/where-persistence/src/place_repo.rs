//! Places of the house where material stands while it is in use (`"where_place"`).
//!
//! A place is not a warehouse: `Foyer`, `Podium` and `Bühne` are rooms where material is
//! set up for a while, and a device that stands there is **issued** — still ours, but no
//! longer on the shelf. That is what `"Bestand_Entnahme*"` counts; see
//! `stock_repo::issue`.
//!
//! The list is master data: it is maintained next to the warehouses and stays when a device
//! leaves a place.

use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;

/// One place of the house.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PlaceRow {
    pub id: i32,
    pub name: String,
    pub note: Option<String>,
    pub active: bool,
}

const PLACE_LIST_SQL: &str = r#"
SELECT "id", "name", "note", COALESCE("active", true) AS active
  FROM "where_place"
 ORDER BY COALESCE("active", true) DESC, "name""#;

const PLACE_GET_SQL: &str = r#"
SELECT "id", "name", "note", COALESCE("active", true) AS active
  FROM "where_place" WHERE "id" = $1"#;

const PLACE_INSERT_SQL: &str = r#"
INSERT INTO "where_place" ("name", "note", "active")
VALUES ($1, $2, $3) RETURNING "id""#;

const PLACE_UPDATE_SQL: &str = r#"
UPDATE "where_place" SET "name" = $2, "note" = $3, "active" = $4
 WHERE "id" = $1"#;

const PLACE_DELETE_SQL: &str = r#"DELETE FROM "where_place" WHERE "id" = $1"#;

const PLACE_IN_USE_SQL: &str =
    r#"SELECT 1 FROM "Seriennummern" WHERE "where_place_id" = $1 LIMIT 1"#;

#[derive(Clone)]
pub struct PlaceRepository {
    pool: PgPool,
}

impl PlaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<PlaceRow>, DomainError> {
        sqlx::query_as::<_, PlaceRow>(PLACE_LIST_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn get(&self, id: i32) -> Result<PlaceRow, DomainError> {
        sqlx::query_as::<_, PlaceRow>(PLACE_GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Place",
                id,
            })
    }

    pub async fn create(
        &self,
        name: &str,
        note: Option<&str>,
        active: bool,
    ) -> Result<PlaceRow, DomainError> {
        let name = Self::validate_name(name)?;
        let (id,): (i32,) = sqlx::query_as(PLACE_INSERT_SQL)
            .bind(name)
            .bind(note)
            .bind(active)
            .fetch_one(&self.pool)
            .await
            .map_err(Self::unique_error)?;
        self.get(id).await
    }

    pub async fn update(
        &self,
        id: i32,
        name: &str,
        note: Option<&str>,
        active: bool,
    ) -> Result<PlaceRow, DomainError> {
        let name = Self::validate_name(name)?;
        let affected = sqlx::query(PLACE_UPDATE_SQL)
            .bind(id)
            .bind(name)
            .bind(note)
            .bind(active)
            .execute(&self.pool)
            .await
            .map_err(Self::unique_error)?
            .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "Place",
                id,
            });
        }
        self.get(id).await
    }

    /// Removes a place — refused while a device still stands there.
    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        let in_use: Option<(i32,)> = sqlx::query_as(PLACE_IN_USE_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if in_use.is_some() {
            return Err(DomainError::Conflict(
                "Material steht noch an diesem Ort".into(),
            ));
        }
        let affected = sqlx::query(PLACE_DELETE_SQL)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?
            .rows_affected();
        if affected == 0 {
            return Err(DomainError::NotFound {
                entity: "Place",
                id,
            });
        }
        Ok(())
    }

    fn validate_name(name: &str) -> Result<&str, DomainError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "Der Ort braucht einen Namen".into(),
            ));
        }
        Ok(name)
    }

    /// The unique index on the lower-cased name answers with `23505`.
    fn unique_error(error: sqlx::Error) -> DomainError {
        let unique = match &error {
            sqlx::Error::Database(db) => db.code().as_deref() == Some("23505"),
            _ => false,
        };
        if unique {
            return DomainError::Conflict("Diesen Ort gibt es bereits".into());
        }
        db_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_name_is_refused() {
        assert!(PlaceRepository::validate_name("   ").is_err());
        assert_eq!(PlaceRepository::validate_name(" Foyer ").unwrap(), "Foyer");
    }
}
