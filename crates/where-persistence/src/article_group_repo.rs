use sqlx::PgPool;
use where_domain::DomainError;

use crate::error::db_error;

/// Article group including usage count and parent node.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ArticleGroupEntry {
    pub id: i32,
    pub name: Option<String>,
    pub parent_id: Option<i32>,
    pub article_count: i64,
}

const LIST_SQL: &str = r#"
SELECT w."Primary_Key" AS id,
       w."Index" AS name,
       CASE WHEN w."Parent_ID" = 0 THEN NULL ELSE w."Parent_ID" END AS parent_id,
       (SELECT count(*) FROM "Artikel_Warengruppe" aw
         WHERE aw."Storage_Primkey" = w."Primary_Key") AS article_count
  FROM "Storage_Warengruppe" w
 WHERE ($1::text IS NULL OR w."Index" ILIKE '%' || $1 || '%')
 ORDER BY w."Index""#;

const GET_SQL: &str = r#"
SELECT w."Primary_Key" AS id,
       w."Index" AS name,
       CASE WHEN w."Parent_ID" = 0 THEN NULL ELSE w."Parent_ID" END AS parent_id,
       (SELECT count(*) FROM "Artikel_Warengruppe" aw
         WHERE aw."Storage_Primkey" = w."Primary_Key") AS article_count
  FROM "Storage_Warengruppe" w
 WHERE w."Primary_Key" = $1"#;

const INSERT_SQL: &str = r#"
INSERT INTO "Storage_Warengruppe" ("Index", "Parent_ID", "Child_ID")
VALUES ($1, COALESCE($2, 0), 0)
RETURNING "Primary_Key" AS id"#;

const UPDATE_SQL: &str = r#"
UPDATE "Storage_Warengruppe"
   SET "Index" = $2, "Parent_ID" = COALESCE($3, 0)
 WHERE "Primary_Key" = $1
RETURNING "Primary_Key" AS id"#;

const DELETE_SQL: &str = r#"DELETE FROM "Storage_Warengruppe" WHERE "Primary_Key" = $1"#;

const EXISTS_SQL: &str = r#"SELECT 1 FROM "Storage_Warengruppe" WHERE "Primary_Key" = $1"#;

/// Same name, ignoring case and surrounding spaces — the comparison the duplicate
/// check uses, so the import finds exactly the group the admin screen would reject.
const FIND_BY_NAME_SQL: &str = r#"
SELECT w."Primary_Key" AS id,
       w."Index" AS name,
       CASE WHEN w."Parent_ID" = 0 THEN NULL ELSE w."Parent_ID" END AS parent_id,
       (SELECT count(*) FROM "Artikel_Warengruppe" aw
         WHERE aw."Storage_Primkey" = w."Primary_Key") AS article_count
  FROM "Storage_Warengruppe" w
 WHERE lower(btrim(w."Index")) = lower(btrim($1))
 ORDER BY w."Primary_Key"
 LIMIT 1"#;

/// Same name (ignoring case and surrounding spaces), different id.
const DUPLICATE_SQL: &str = r#"
SELECT 1 FROM "Storage_Warengruppe"
 WHERE lower(btrim("Index")) = lower(btrim($1))
   AND "Primary_Key" <> $2
 LIMIT 1"#;

const USAGE_SQL: &str =
    r#"SELECT count(*) FROM "Artikel_Warengruppe" WHERE "Storage_Primkey" = $1"#;

const CHILDREN_SQL: &str = r#"SELECT count(*) FROM "Storage_Warengruppe" WHERE "Parent_ID" = $1"#;

/// Is `$2` a descendant of `$1`? Prevents cycles when re-parenting.
const IS_DESCENDANT_SQL: &str = r#"
WITH RECURSIVE d AS (
    SELECT "Primary_Key" FROM "Storage_Warengruppe" WHERE "Primary_Key" = $1
    UNION ALL
    SELECT w."Primary_Key"
      FROM "Storage_Warengruppe" w
      JOIN d ON w."Parent_ID" = d."Primary_Key"
)
SELECT count(*) FROM d WHERE "Primary_Key" = $2"#;

/// Article group maintenance (admin area).
#[derive(Clone)]
pub struct ArticleGroupRepository {
    pool: PgPool,
}

impl ArticleGroupRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(&self, search: Option<&str>) -> Result<Vec<ArticleGroupEntry>, DomainError> {
        sqlx::query_as::<_, ArticleGroupEntry>(LIST_SQL)
            .bind(search)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn get(&self, id: i32) -> Result<ArticleGroupEntry, DomainError> {
        sqlx::query_as::<_, ArticleGroupEntry>(GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Article group",
                id,
            })
    }

    /// The group with this name, if there is one — the import needs it before it
    /// decides whether to create the group.
    pub async fn find_by_name(&self, name: &str) -> Result<Option<ArticleGroupEntry>, DomainError> {
        sqlx::query_as::<_, ArticleGroupEntry>(FIND_BY_NAME_SQL)
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)
    }

    pub async fn create(
        &self,
        name: &str,
        parent_id: Option<i32>,
    ) -> Result<ArticleGroupEntry, DomainError> {
        let name = Self::validate_name(name)?;
        self.validate_unique(&name, 0).await?;
        if let Some(p) = parent_id {
            self.validate_exists(p).await?;
        }

        let (id,): (i32,) = sqlx::query_as(INSERT_SQL)
            .bind(&name)
            .bind(parent_id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        self.get(id).await
    }

    pub async fn update(
        &self,
        id: i32,
        name: &str,
        parent_id: Option<i32>,
    ) -> Result<ArticleGroupEntry, DomainError> {
        let name = Self::validate_name(name)?;
        self.validate_exists(id).await?;
        self.validate_unique(&name, id).await?;

        if let Some(p) = parent_id {
            if p == id {
                return Err(DomainError::Validation(
                    "An article group cannot be its own parent".into(),
                ));
            }
            self.validate_exists(p).await?;
            let (is_descendant,): (i64,) = sqlx::query_as(IS_DESCENDANT_SQL)
                .bind(id)
                .bind(p)
                .fetch_one(&self.pool)
                .await
                .map_err(db_error)?;
            if is_descendant > 0 {
                return Err(DomainError::Validation(
                    "The parent must not be a subgroup of the group (cycle)".into(),
                ));
            }
        }

        sqlx::query_as::<_, (i32,)>(UPDATE_SQL)
            .bind(id)
            .bind(&name)
            .bind(parent_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Article group",
                id,
            })?;

        self.get(id).await
    }

    /// Deletes only when the group is unused and has no subgroups.
    /// Both counts are named so the UI can show a meaningful message.
    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        self.validate_exists(id).await?;

        let (in_use,): (i64,) = sqlx::query_as(USAGE_SQL)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        if in_use > 0 {
            return Err(DomainError::Conflict(format!(
                "Article group is assigned to {in_use} article(s) and cannot be deleted"
            )));
        }

        let (subgroups,): (i64,) = sqlx::query_as(CHILDREN_SQL)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        if subgroups > 0 {
            return Err(DomainError::Conflict(format!(
                "Article group has {subgroups} subgroup(s) and cannot be deleted"
            )));
        }

        let result = sqlx::query(DELETE_SQL)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "Article group",
                id,
            });
        }
        Ok(())
    }

    fn validate_name(name: &str) -> Result<String, DomainError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(DomainError::Validation("Name must not be empty".into()));
        }
        if name.chars().count() > 255 {
            return Err(DomainError::Validation(
                "Name must not exceed 255 characters".into(),
            ));
        }
        Ok(name.to_string())
    }

    async fn validate_exists(&self, id: i32) -> Result<(), DomainError> {
        let found: Option<(i32,)> = sqlx::query_as(EXISTS_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if found.is_none() {
            return Err(DomainError::NotFound {
                entity: "Article group",
                id,
            });
        }
        Ok(())
    }

    async fn validate_unique(&self, name: &str, ausser_id: i32) -> Result<(), DomainError> {
        let exists: Option<(i32,)> = sqlx::query_as(DUPLICATE_SQL)
            .bind(name)
            .bind(ausser_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;
        if exists.is_some() {
            return Err(DomainError::Conflict(format!(
                "Warengruppe '{name}' existiert bereits"
            )));
        }
        Ok(())
    }
}
