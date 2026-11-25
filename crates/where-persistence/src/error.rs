use sqlx::Error as SqlxError;
use where_domain::DomainError;

/// Maps a database error onto a domain error.
///
/// PostgreSQL SQLSTATE codes are used rather than message matching:
/// `23505` unique violation -> conflict, `23502` not-null violation ->
/// validation. Everything else is reported as internal; the API layer logs the
/// detail and returns a generic problem document.
pub fn db_error(err: SqlxError) -> DomainError {
    if let SqlxError::Database(ref db) = err {
        match db.code().as_deref() {
            Some("23505") => {
                return DomainError::Conflict(format!(
                    "duplicate value for constraint {}",
                    db.constraint().unwrap_or("unknown")
                ));
            }
            Some("23502") => {
                return DomainError::Validation("a required value is missing".to_string());
            }
            Some("23503") => {
                return DomainError::Validation("referenced record does not exist".into());
            }
            _ => {}
        }
    }
    DomainError::Internal(err.to_string())
}
