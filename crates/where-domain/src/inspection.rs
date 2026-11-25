//! Inspection reports: PDF reports bound to one device (`"Seriennummern"`).
//!
//! A protocol belongs to the piece, not to the article — the paper names the serial
//! and inventory number. Both are copied into the row at upload time so a protocol
//! keeps naming the device even if a number is corrected later.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// One uploaded protocol of a device.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct InspectionReport {
    pub id: i32,
    pub device_id: i32,
    pub article_id: i32,
    /// Serial number of the device when the file was uploaded.
    pub serial_no: Option<String>,
    /// Inventory number of the device when the file was uploaded.
    pub inventory_no: Option<String>,
    pub title: String,
    pub mime: Option<String>,
    pub size: Option<i64>,
    pub sha256: String,
    pub uploaded_by: Option<String>,
    pub uploaded_at: Option<DateTime<Utc>>,
}

/// The article image: the uploaded picture (an entry of `"article_file"` in the
/// folder `bild`) and whether the animated placeholder stands in for it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArticleImage {
    #[serde(default)]
    pub file: Option<ArticleImageFile>,
    #[serde(default)]
    pub placeholder: bool,
}

/// The part of the file entry the image needs; enough to fetch its content.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArticleImageFile {
    pub id: i32,
    pub name: String,
    pub mime: Option<String>,
    pub size: Option<i64>,
    pub uploaded_at: Option<DateTime<Utc>>,
}

impl InspectionReport {
    /// A protocol needs a title; the file itself is checked by [`is_pdf`].
    pub fn validate(&self) -> Result<(), crate::DomainError> {
        if self.title.trim().is_empty() {
            return Err(crate::DomainError::Validation(
                "Der Titel des Prüfprotokolls darf nicht leer sein".into(),
            ));
        }
        Ok(())
    }
}

/// A protocol is a PDF — recognised by its signature, not by the MIME type a
/// browser guessed or the ending of a file name.
pub fn is_pdf(content: &[u8]) -> bool {
    content.starts_with(b"%PDF-")
}
