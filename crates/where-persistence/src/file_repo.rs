use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;
use where_domain::{ArticleFile, ArticleImage, ArticleImageFile, DomainError};

use crate::error::db_error;
use crate::rows::FileRow;

const LIST_SQL: &str = r#"
SELECT "id", "article_id", "folder", "name", "is_link", "link_target", "mime",
       "size", "sha256", "version", "is_current", "uploaded_by", "uploaded_at"
  FROM "article_file"
 WHERE "article_id" = $1 AND NOT "deleted" AND "is_current"
 ORDER BY "folder", "name""#;

const LIST_ALL_SQL: &str = r#"
SELECT "id", "article_id", "folder", "name", "is_link", "link_target", "mime",
       "size", "sha256", "version", "is_current", "uploaded_by", "uploaded_at"
  FROM "article_file"
 WHERE "article_id" = $1 AND NOT "deleted"
 ORDER BY "folder", "name", "version""#;

const GET_SQL: &str = r#"
SELECT "id", "article_id", "folder", "name", "is_link", "link_target", "mime",
       "size", "sha256", "version", "is_current", "uploaded_by", "uploaded_at"
  FROM "article_file"
 WHERE "id" = $1 AND NOT "deleted""#;

const MAX_VERSION_SQL: &str = r#"
SELECT COALESCE(MAX("version"), 0) FROM "article_file"
 WHERE "article_id" = $1 AND "folder" = $2 AND "name" = $3"#;

const CLEAR_CURRENT_SQL: &str = r#"
UPDATE "article_file" SET "is_current" = false
 WHERE "article_id" = $1 AND "folder" = $2 AND "name" = $3 AND "is_current""#;

const INSERT_SQL: &str = r#"
INSERT INTO "article_file" (
    "article_id", "folder", "name", "is_link", "link_target", "mime",
    "size", "sha256", "version", "is_current", "uploaded_by"
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true, $10)
RETURNING "id", "article_id", "folder", "name", "is_link", "link_target", "mime",
          "size", "sha256", "version", "is_current", "uploaded_by", "uploaded_at""#;

const SOFT_DELETE_SQL: &str =
    r#"UPDATE "article_file" SET "deleted" = true, "is_current" = false WHERE "id" = $1"#;

/// Folder of the article image inside `"article_file"`; the fixed name keeps every
/// new photo as the next version of the same entry.
pub const IMAGE_FOLDER: &str = "bild";
pub const IMAGE_NAME: &str = "artikelbild";

const IMAGE_SQL: &str = r#"
SELECT "id", "name", "mime", "size", "uploaded_at"
  FROM "article_file"
 WHERE "article_id" = $1 AND "folder" = $2 AND "name" = $3 AND NOT "deleted" AND "is_current""#;

const IMAGE_PLACEHOLDER_SQL: &str = r#"
SELECT "placeholder" FROM "where_article_image" WHERE "article_id" = $1"#;

const IMAGE_PLACEHOLDER_SET_SQL: &str = r#"
INSERT INTO "where_article_image" ("article_id", "placeholder") VALUES ($1, $2)
ON CONFLICT ("article_id") DO UPDATE SET "placeholder" = excluded."placeholder", "updated_at" = now()"#;

const IMAGE_DELETE_SQL: &str = r#"
WITH betroffen AS (
    UPDATE "article_file" SET "deleted" = true, "is_current" = false
     WHERE "article_id" = $1 AND "folder" = $2 AND "name" = $3
       AND NOT "deleted" AND "is_current"
    RETURNING "id"
)
SELECT count(*) FROM betroffen"#;

const HISTORY_SQL: &str = r#"
INSERT INTO "article_file_history" ("file_id", "aktion", "detail", "benutzer")
VALUES ($1, $2, $3, $4)"#;

/// The part of the image entry the mask needs.
#[derive(Debug, sqlx::FromRow)]
struct ImageRow {
    id: i32,
    name: String,
    mime: Option<String>,
    size: Option<i64>,
    uploaded_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// File content storage. Metadata lives in `article_file`, the bytes live
/// under `<root>/<sha256[0..2]>/<sha256>` — content addressed, so the same
/// file only takes space once across articles.
#[derive(Clone)]
pub struct FileRepository {
    pool: PgPool,
    root: PathBuf,
}

impl FileRepository {
    pub fn new(pool: PgPool) -> Self {
        let root = std::env::var("FILE_STORAGE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("data/files"));
        Self::with_root(pool, root)
    }

    /// Uses `root` as the storage directory instead of the environment default.
    ///
    /// Packaged installations pass an absolute path: the relative default would
    /// resolve against the working directory, which is read-only below
    /// `Program Files`.
    pub fn with_root(pool: PgPool, root: impl Into<PathBuf>) -> Self {
        Self {
            pool,
            root: root.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Current file storage entries of an article.
    pub async fn list(&self, article_id: i32) -> Result<Vec<ArticleFile>, DomainError> {
        let rows = sqlx::query_as::<_, FileRow>(LIST_SQL)
            .bind(article_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// All versions (for the "version overview").
    pub async fn history(&self, article_id: i32) -> Result<Vec<ArticleFile>, DomainError> {
        let rows = sqlx::query_as::<_, FileRow>(LIST_ALL_SQL)
            .bind(article_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, id: i32) -> Result<ArticleFile, DomainError> {
        let row = sqlx::query_as::<_, FileRow>(GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "article_file",
                id,
            })?;
        Ok(row.into())
    }

    /// Writes a file to the store and creates the metadata.
    /// An existing file with the same name becomes the new version.
    pub async fn upload(
        &self,
        article_id: i32,
        folder: &str,
        name: &str,
        mime: Option<&str>,
        content: &[u8],
        user: Option<&str>,
    ) -> Result<ArticleFile, DomainError> {
        let file = ArticleFile {
            article_id,
            folder: folder.to_string(),
            name: name.to_string(),
            is_link: false,
            ..Default::default()
        };
        file.validate()?;

        let sha = hex_sha256(content);
        let path = self.path_for(&sha);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
            }
            // Write to a temporary file first, then rename atomically: a crash
            // therefore leaves no half-written file under the hash.
            let tmp = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
            tokio::fs::write(&tmp, content)
                .await
                .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
            tokio::fs::rename(&tmp, &path)
                .await
                .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
        }

        let mut tx = self.pool.begin().await.map_err(db_error)?;

        let version: i32 = sqlx::query_scalar(MAX_VERSION_SQL)
            .bind(article_id)
            .bind(folder)
            .bind(name)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;
        let version = version + 1;

        sqlx::query(CLEAR_CURRENT_SQL)
            .bind(article_id)
            .bind(folder)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

        let row = sqlx::query_as::<_, FileRow>(INSERT_SQL)
            .bind(article_id)
            .bind(folder)
            .bind(name)
            .bind(false)
            .bind(None::<&str>)
            .bind(mime)
            .bind(content.len() as i64)
            .bind(&sha)
            .bind(version)
            .bind(user)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;

        Self::log(
            &mut tx,
            row.id,
            if version == 1 {
                "angelegt"
            } else {
                "aktualisiert"
            },
            Some(&format!("Version {version}, {sha}")),
            user,
        )
        .await?;

        tx.commit().await.map_err(db_error)?;
        Ok(row.into())
    }

    /// "Add link": points outside instead of storing bytes.
    pub async fn add_link(
        &self,
        article_id: i32,
        folder: &str,
        name: &str,
        target: &str,
        user: Option<&str>,
    ) -> Result<ArticleFile, DomainError> {
        let file = ArticleFile {
            article_id,
            folder: folder.to_string(),
            name: name.to_string(),
            is_link: true,
            link_target: Some(target.to_string()),
            ..Default::default()
        };
        file.validate()?;

        let mut tx = self.pool.begin().await.map_err(db_error)?;

        let version: i32 = sqlx::query_scalar(MAX_VERSION_SQL)
            .bind(article_id)
            .bind(folder)
            .bind(name)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;
        let version = version + 1;

        sqlx::query(CLEAR_CURRENT_SQL)
            .bind(article_id)
            .bind(folder)
            .bind(name)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;

        let row = sqlx::query_as::<_, FileRow>(INSERT_SQL)
            .bind(article_id)
            .bind(folder)
            .bind(name)
            .bind(true)
            .bind(Some(target))
            .bind(None::<&str>)
            .bind(None::<i64>)
            .bind(None::<&str>)
            .bind(version)
            .bind(user)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_error)?;

        Self::log(&mut tx, row.id, "verlinkt", Some(target), user).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(row.into())
    }

    /// Path of the content of a file. `None` for links.
    pub async fn content_path(&self, id: i32) -> Result<Option<PathBuf>, DomainError> {
        let file = self.get(id).await?;
        if file.is_link {
            return Ok(None);
        }
        match file.sha256 {
            Some(sha) => Ok(Some(self.path_for(&sha))),
            None => Err(DomainError::Internal("File without a content hash".into())),
        }
    }

    /// Reads the content for the download.
    pub async fn content(&self, id: i32) -> Result<(ArticleFile, Vec<u8>), DomainError> {
        let file = self.get(id).await?;
        if file.is_link {
            return Err(DomainError::Validation(
                "Link hat keinen gespeicherten Inhalt".into(),
            ));
        }
        let sha = file
            .sha256
            .clone()
            .ok_or_else(|| DomainError::Internal("File without a content hash".into()))?;
        let bytes = tokio::fs::read(self.path_for(&sha))
            .await
            .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
        Ok((file, bytes))
    }

    /// Soft delete: the metadata stays for the history.
    pub async fn delete(&self, id: i32, user: Option<&str>) -> Result<(), DomainError> {
        let file = self.get(id).await?;
        let mut tx = self.pool.begin().await.map_err(db_error)?;
        let affected = sqlx::query(SOFT_DELETE_SQL)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
        if affected.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "article_file",
                id,
            });
        }
        Self::log(&mut tx, id, "deleted", Some(&file.name), user).await?;
        tx.commit().await.map_err(db_error)?;
        Ok(())
    }

    fn path_for(&self, sha: &str) -> PathBuf {
        self.root.join(&sha[..2]).join(sha)
    }

    // ------------------------------------------------------- Article image

    /// The article image and whether the placeholder stands in for it.
    pub async fn image(&self, article_id: i32) -> Result<ArticleImage, DomainError> {
        let placeholder: Option<bool> = sqlx::query_scalar(IMAGE_PLACEHOLDER_SQL)
            .bind(article_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;

        let file: Option<ImageRow> = sqlx::query_as::<_, ImageRow>(IMAGE_SQL)
            .bind(article_id)
            .bind(IMAGE_FOLDER)
            .bind(IMAGE_NAME)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?;

        Ok(ArticleImage {
            file: file.map(|row| ArticleImageFile {
                id: row.id,
                name: row.name,
                mime: row.mime,
                size: row.size,
                uploaded_at: row.uploaded_at,
            }),
            placeholder: placeholder.unwrap_or(false),
        })
    }

    /// Takes a new photo. It replaces the previous one (new version of the same
    /// entry) and switches the placeholder off.
    pub async fn image_upload(
        &self,
        article_id: i32,
        name: &str,
        mime: Option<&str>,
        content: &[u8],
        user: Option<&str>,
    ) -> Result<ArticleImage, DomainError> {
        self.upload(article_id, IMAGE_FOLDER, IMAGE_NAME, mime, content, user)
            .await?;
        sqlx::query(IMAGE_PLACEHOLDER_SET_SQL)
            .bind(article_id)
            .bind(false)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        self.log_image(article_id, "bild", Some(name), user).await?;
        self.image(article_id).await
    }

    /// Switches between the uploaded photo and the animated placeholder.
    pub async fn image_placeholder(
        &self,
        article_id: i32,
        placeholder: bool,
    ) -> Result<ArticleImage, DomainError> {
        sqlx::query(IMAGE_PLACEHOLDER_SET_SQL)
            .bind(article_id)
            .bind(placeholder)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        self.image(article_id).await
    }

    /// Removes the uploaded photo; the placeholder stays off as well.
    pub async fn image_delete(
        &self,
        article_id: i32,
        user: Option<&str>,
    ) -> Result<ArticleImage, DomainError> {
        let (_removed,): (i64,) = sqlx::query_as(IMAGE_DELETE_SQL)
            .bind(article_id)
            .bind(IMAGE_FOLDER)
            .bind(IMAGE_NAME)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        sqlx::query(IMAGE_PLACEHOLDER_SET_SQL)
            .bind(article_id)
            .bind(false)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        self.log_image(article_id, "bild entfernt", None, user)
            .await?;
        self.image(article_id).await
    }

    /// The change of the image ends up in the history of the article — it points at
    /// the current image entry, so the entry keeps its trail.
    async fn log_image(
        &self,
        article_id: i32,
        aktion: &str,
        detail: Option<&str>,
        user: Option<&str>,
    ) -> Result<(), DomainError> {
        let id: Option<i32> = sqlx::query_scalar(
            r#"SELECT "id" FROM "article_file"
                WHERE "article_id" = $1 AND "folder" = $2 AND NOT "deleted"
                ORDER BY "id" DESC LIMIT 1"#,
        )
        .bind(article_id)
        .bind(IMAGE_FOLDER)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_error)?;
        if let Some(file_id) = id {
            sqlx::query(HISTORY_SQL)
                .bind(file_id)
                .bind(aktion)
                .bind(detail)
                .bind(user)
                .execute(&self.pool)
                .await
                .map_err(db_error)?;
        }
        Ok(())
    }

    async fn log(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        file_id: i32,
        aktion: &str,
        detail: Option<&str>,
        user: Option<&str>,
    ) -> Result<(), DomainError> {
        sqlx::query(HISTORY_SQL)
            .bind(file_id)
            .bind(aktion)
            .bind(detail)
            .bind(user)
            .execute(&mut **tx)
            .await
            .map_err(db_error)?;
        Ok(())
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}
