use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An entry of the file storage (dialog `img/ablage_01.PNG`).
///
/// The content itself is not stored in the database but under a path
/// addressed by SHA-256 in the file store; only the metadata lives here.
#[derive(Debug, Clone, Default, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ArticleFile {
    pub id: i32,
    pub article_id: i32,
    /// Empty for the root, otherwise a folder path such as `datasheets`.
    pub folder: String,
    pub name: String,
    /// `true` for "add link": then `link_target` points outside.
    pub is_link: bool,
    pub link_target: Option<String>,
    pub mime: Option<String>,
    pub size: Option<i64>,
    pub sha256: Option<String>,
    /// Starts at 1; every update of the same file increases it.
    pub version: i32,
    pub is_current: bool,
    pub uploaded_by: Option<String>,
    pub uploaded_at: Option<DateTime<Utc>>,
}

impl ArticleFile {
    /// A link needs a target, a file needs content.
    pub fn validate(&self) -> Result<(), crate::DomainError> {
        if self.name.trim().is_empty() {
            return Err(crate::DomainError::Validation(
                "File name must not be empty".into(),
            ));
        }
        if self.name.contains('/') || self.name.contains('\\') {
            return Err(crate::DomainError::Validation(
                "Dateiname darf keine Pfadtrenner enthalten".into(),
            ));
        }
        if self.is_link && self.link_target.as_deref().unwrap_or("").trim().is_empty() {
            return Err(crate::DomainError::Validation(
                "A link needs a target URL".into(),
            ));
        }
        Ok(())
    }
}
