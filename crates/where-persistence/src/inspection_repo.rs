//! Inspection reports: PDF files per device, content addressed like the article files.
//!
//! The bytes live under the same root as `article_file`
//! (`<root>/<sha256[0..2]>/<sha256>`), so a protocol that is uploaded twice takes
//! space once. The database row carries the binding to serial and inventory number.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;
use where_domain::{DomainError, InspectionReport};

use crate::error::db_error;

#[derive(Debug, FromRow)]
struct ReportRow {
    id: i32,
    device_id: i32,
    article_id: i32,
    serial_no: Option<String>,
    inventory_no: Option<String>,
    title: String,
    mime: Option<String>,
    size: Option<i64>,
    sha256: String,
    uploaded_by: Option<String>,
    uploaded_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<ReportRow> for InspectionReport {
    fn from(row: ReportRow) -> Self {
        Self {
            id: row.id,
            device_id: row.device_id,
            article_id: row.article_id,
            serial_no: row.serial_no,
            inventory_no: row.inventory_no,
            title: row.title,
            mime: row.mime,
            size: row.size,
            sha256: row.sha256,
            uploaded_by: row.uploaded_by,
            uploaded_at: row.uploaded_at,
        }
    }
}

const LIST_SQL: &str = r#"
SELECT "id", "device_id", "article_id", "serial_no", "inventory_no", "title",
       "mime", "size", "sha256", "uploaded_by", "uploaded_at"
  FROM "where_inspection_report"
 WHERE "article_id" = $1 AND NOT "deleted"
 ORDER BY "device_id", "uploaded_at" DESC, "id" DESC"#;

const LIST_DEVICE_SQL: &str = r#"
SELECT "id", "device_id", "article_id", "serial_no", "inventory_no", "title",
       "mime", "size", "sha256", "uploaded_by", "uploaded_at"
  FROM "where_inspection_report"
 WHERE "article_id" = $1 AND "device_id" = $2 AND NOT "deleted"
 ORDER BY "uploaded_at" DESC, "id" DESC"#;

const GET_SQL: &str = r#"
SELECT "id", "device_id", "article_id", "serial_no", "inventory_no", "title",
       "mime", "size", "sha256", "uploaded_by", "uploaded_at"
  FROM "where_inspection_report"
 WHERE "id" = $1 AND NOT "deleted""#;

const INSERT_SQL: &str = r#"
INSERT INTO "where_inspection_report" (
    "device_id", "article_id", "serial_no", "inventory_no", "title",
    "mime", "size", "sha256", "uploaded_by"
) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
RETURNING "id", "device_id", "article_id", "serial_no", "inventory_no", "title",
          "mime", "size", "sha256", "uploaded_by", "uploaded_at""#;

const SOFT_DELETE_SQL: &str = r#"
UPDATE "where_inspection_report" SET "deleted" = true WHERE "id" = $1 AND NOT "deleted""#;

/// The serial and inventory number a protocol is bound to.
#[derive(Debug, Clone, Default)]
pub struct DeviceBinding {
    pub serial_no: Option<String>,
    pub inventory_no: Option<String>,
}

/// Everything one upload carries.
pub struct NewReport<'a> {
    pub article_id: i32,
    pub device_id: i32,
    pub binding: DeviceBinding,
    pub title: &'a str,
    pub mime: Option<&'a str>,
    pub content: &'a [u8],
    pub user: Option<&'a str>,
}

#[derive(Clone)]
pub struct InspectionRepository {
    pool: PgPool,
    root: PathBuf,
}

impl InspectionRepository {
    pub fn with_root(pool: PgPool, root: impl Into<PathBuf>) -> Self {
        Self {
            pool,
            root: root.into(),
        }
    }

    /// All protocols of an article, newest first per device.
    pub async fn list(&self, article_id: i32) -> Result<Vec<InspectionReport>, DomainError> {
        let rows = sqlx::query_as::<_, ReportRow>(LIST_SQL)
            .bind(article_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// Protocols of one device.
    pub async fn list_for_device(
        &self,
        article_id: i32,
        device_id: i32,
    ) -> Result<Vec<InspectionReport>, DomainError> {
        let rows = sqlx::query_as::<_, ReportRow>(LIST_DEVICE_SQL)
            .bind(article_id)
            .bind(device_id)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, id: i32) -> Result<InspectionReport, DomainError> {
        let row = sqlx::query_as::<_, ReportRow>(GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "inspection_report",
                id,
            })?;
        Ok(row.into())
    }

    /// Stores the PDF and the binding to the device.
    pub async fn upload(&self, neu: NewReport<'_>) -> Result<InspectionReport, DomainError> {
        let report = InspectionReport {
            device_id: neu.device_id,
            article_id: neu.article_id,
            serial_no: neu.binding.serial_no,
            inventory_no: neu.binding.inventory_no,
            title: neu.title.to_string(),
            mime: neu.mime.map(str::to_string),
            size: Some(neu.content.len() as i64),
            ..Default::default()
        };
        report.validate()?;

        let sha = hex_sha256(neu.content);
        self.store(&sha, neu.content).await?;

        let row = sqlx::query_as::<_, ReportRow>(INSERT_SQL)
            .bind(neu.device_id)
            .bind(neu.article_id)
            .bind(report.serial_no.as_deref())
            .bind(report.inventory_no.as_deref())
            .bind(&report.title)
            .bind(report.mime.as_deref())
            .bind(report.size)
            .bind(&sha)
            .bind(neu.user)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(row.into())
    }

    /// Reads the PDF for the viewer.
    pub async fn content(&self, id: i32) -> Result<(InspectionReport, Vec<u8>), DomainError> {
        let report = self.get(id).await?;
        let bytes = tokio::fs::read(self.path_for(&report.sha256))
            .await
            .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
        Ok((report, bytes))
    }

    /// Soft delete: the row stays, the PDF is no longer listed.
    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        let affected = sqlx::query(SOFT_DELETE_SQL)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        if affected.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "inspection_report",
                id,
            });
        }
        Ok(())
    }

    /// Writes the bytes once per hash; a second upload of the same PDF is free.
    async fn store(&self, sha: &str, content: &[u8]) -> Result<(), DomainError> {
        let path = self.path_for(sha);
        if path.exists() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
        }
        let tmp = path.with_extension(format!("tmp-{}", Uuid::new_v4()));
        tokio::fs::write(&tmp, content)
            .await
            .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
        tokio::fs::rename(&tmp, &path)
            .await
            .map_err(|e| DomainError::Internal(format!("Dateispeicher: {e}")))?;
        Ok(())
    }

    fn path_for(&self, sha: &str) -> PathBuf {
        self.root.join(&sha[..2]).join(sha)
    }

    pub fn root(&self) -> &Path {
        &self.root
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
