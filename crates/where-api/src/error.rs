use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;
use where_domain::DomainError;

/// RFC 9457 problem document, matching the shape the Angular client already
/// expects from the original REST API (`rest_get_stdsuccess`/`stderr`).
#[derive(Debug, Serialize, ToSchema)]
pub struct Problem {
    /// Short, stable machine-readable title, e.g. `not-found`.
    pub title: String,
    /// HTTP status code, duplicated for clients that cannot read headers.
    pub status: u16,
    /// Human-readable explanation, safe to display.
    pub detail: String,
}

#[derive(Debug)]
pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(err: DomainError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title, detail) = match &self.0 {
            DomainError::Validation(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation", msg.clone())
            }
            DomainError::NotFound { entity, id } => (
                StatusCode::NOT_FOUND,
                "not-found",
                format!("{entity} with id {id} does not exist"),
            ),
            DomainError::NotFoundKey { entity, key } => (
                StatusCode::NOT_FOUND,
                "not-found",
                format!("{entity} '{key}' does not exist"),
            ),
            DomainError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            DomainError::Unauthorized(msg) => {
                (StatusCode::UNAUTHORIZED, "unauthorized", msg.clone())
            }
            DomainError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
            DomainError::Upstream(msg) => {
                tracing::warn!(error = %msg, "upstream request failed");
                (StatusCode::BAD_GATEWAY, "upstream", msg.clone())
            }
            DomainError::Internal(detail_internal) => {
                // Never leak driver output to the client; log it instead.
                tracing::error!(error = %detail_internal, "internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "an internal error occurred".to_string(),
                )
            }
        };

        let body = Problem {
            title: title.to_string(),
            status: status.as_u16(),
            detail,
        };
        (status, Json(body)).into_response()
    }
}
