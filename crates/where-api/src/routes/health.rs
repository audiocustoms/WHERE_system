use crate::state::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Always `ok`; liveness only proves the process can serve requests.
    pub status: String,
    pub version: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReadyResponse {
    pub status: String,
    /// Whether a pooled connection could be taken and validated.
    pub database: bool,
}

/// Liveness probe. Does not touch the database.
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses((status = 200, description = "Process is alive", body = HealthResponse))
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness probe. Fails with 503 when the database is unreachable.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "system",
    responses(
        (status = 200, description = "Ready to serve traffic", body = ReadyResponse),
        (status = 503, description = "Database unreachable", body = ReadyResponse)
    )
)]
pub async fn ready(State(state): State<AppState>) -> (axum::http::StatusCode, Json<ReadyResponse>) {
    let ok = sqlx_ping(&state).await;
    let status = if ok {
        axum::http::StatusCode::OK
    } else {
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(ReadyResponse {
            status: if ok {
                "ready".into()
            } else {
                "degraded".into()
            },
            database: ok,
        }),
    )
}

async fn sqlx_ping(state: &AppState) -> bool {
    where_persistence::ping(&state.pool).await
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
}
