use thiserror::Error;

/// Errors raised by the domain layer. The API layer maps these onto HTTP
/// status codes; the persistence layer maps database errors onto them.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("{entity} with id {id} not found")]
    NotFound { entity: &'static str, id: i32 },

    /// Like [`DomainError::NotFound`], for entities keyed by a name instead of a number.
    #[error("{entity} '{key}' not found")]
    NotFoundKey { entity: &'static str, key: String },

    #[error("conflict: {0}")]
    Conflict(String),

    /// The caller is not signed in, or the sign-in attempt was rejected. The message
    /// is written for the login mask.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// The caller is signed in but does not hold the right the route asks for.
    /// The message names the right and is written for the user.
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// An external source (an online list) could not be reached or answered with
    /// something unusable. The message is written for the user.
    #[error("upstream: {0}")]
    Upstream(String),

    /// Unexpected infrastructure failure. Never contains credentials or SQL;
    /// the API layer logs the detail and returns a generic problem document.
    #[error("internal error: {0}")]
    Internal(String),
}
