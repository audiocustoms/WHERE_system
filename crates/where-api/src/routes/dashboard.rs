//! Start page aggregates (`Zentrale`).
//!
//! One read for the dashboard: counts, stock sums, the groups with the most
//! articles, what changed recently and where master data is still missing.

use crate::state::State;
use axum::Json;
use where_domain::Dashboard;

use crate::error::ApiError;
use crate::state::AppState;

/// Snapshot of the master data for the start page.
#[utoipa::path(
    get,
    path = "/api/v1/dashboard",
    tag = "zentrale",
    responses(
        (status = 200, description = "Counts, stock sums and recent changes", body = Dashboard),
        (status = 500, description = "Aggregation failed", body = crate::error::Problem)
    )
)]
pub async fn load(State(state): State<AppState>) -> Result<Json<Dashboard>, ApiError> {
    Ok(Json(state.dashboard.load().await?))
}
