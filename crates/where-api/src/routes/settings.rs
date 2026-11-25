//! Settings lists of the admin menu (`img/Admin Menü`).
//!
//! One endpoint family serves every list: the catalogue lives in
//! `where_persistence::settings_repo`, so a caller can only reach a table that
//! WHERE defines.

use crate::state::State;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use serde::Deserialize;
use where_domain::{PrefInput, Prefs, SettingsArea, SettingsInput, SettingsList, SettingsRow};

use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::AppState;

/// All lists of the admin menu.
#[utoipa::path(
    get,
    path = "/api/v1/settings",
    tag = "einstellungen",
    responses((status = 200, body = Vec<SettingsArea>))
)]
pub async fn areas(State(state): State<AppState>) -> Json<Vec<SettingsArea>> {
    Json(state.settings.areas())
}

/// One list with its rows.
#[utoipa::path(
    get,
    path = "/api/v1/settings/{key}",
    tag = "einstellungen",
    params(("key" = String, Path, description = "Key of the list, e.g. `anreden`")),
    responses(
        (status = 200, body = SettingsList),
        (status = 422, description = "Unknown list", body = crate::error::Problem)
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<SettingsList>, ApiError> {
    Ok(Json(state.settings.list(&key).await?))
}

/// Adds an entry.
#[utoipa::path(
    post,
    path = "/api/v1/settings/{key}",
    tag = "einstellungen",
    params(("key" = String, Path, description = "Key of the list")),
    request_body = SettingsInput,
    responses(
        (status = 201, body = SettingsRow),
        (status = 422, description = "Invalid value or unknown list", body = crate::error::Problem)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(input): Json<SettingsInput>,
) -> Result<(StatusCode, Json<SettingsRow>), ApiError> {
    let row = state.settings.create(&key, input).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

/// Changes an entry.
#[utoipa::path(
    put,
    path = "/api/v1/settings/{key}/{id}",
    tag = "einstellungen",
    params(
        ("key" = String, Path, description = "Key of the list"),
        ("id" = i64, Path, description = "Key of the entry")
    ),
    request_body = SettingsInput,
    responses(
        (status = 200, body = SettingsRow),
        (status = 404, description = "Unknown entry", body = crate::error::Problem),
        (status = 422, description = "Invalid value", body = crate::error::Problem)
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path((key, id)): Path<(String, i64)>,
    Json(input): Json<SettingsInput>,
) -> Result<Json<SettingsRow>, ApiError> {
    Ok(Json(state.settings.update(&key, id, input).await?))
}

/// Removes an entry. A number that is still used somewhere keeps its meaning in the
/// referencing record — the legacy schema has no foreign keys.
#[utoipa::path(
    delete,
    path = "/api/v1/settings/{key}/{id}",
    tag = "einstellungen",
    params(
        ("key" = String, Path, description = "Key of the list"),
        ("id" = i64, Path, description = "Key of the entry")
    ),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown entry", body = crate::error::Problem)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path((key, id)): Path<(String, i64)>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.settings.delete(&key, id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Until WHERE has an authentication, every change is booked on this name.
fn current_user() -> &'static str {
    "where"
}

/// Range of options that belong to one form.
#[derive(Debug, Deserialize)]
pub struct PrefParams {
    /// Key prefix, e.g. `address.` — without it every option is returned.
    pub prefix: Option<String>,
}

/// Options of the WHERE masks (checkboxes and small entries of a form).
#[utoipa::path(
    get,
    path = "/api/v1/prefs",
    tag = "einstellungen",
    params(("prefix" = Option<String>, Query, description = "Key prefix, e.g. `address.`")),
    responses((status = 200, body = Prefs))
)]
pub async fn prefs(
    State(state): State<AppState>,
    Query(params): Query<PrefParams>,
) -> Result<Json<Prefs>, ApiError> {
    let prefix = params
        .prefix
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty());
    Ok(Json(state.settings.prefs(prefix).await?))
}

/// Stores one option; an empty value removes it.
#[utoipa::path(
    put,
    path = "/api/v1/prefs/{key}",
    tag = "einstellungen",
    params(("key" = String, Path, description = "Option, e.g. `address.brief.anrede`")),
    request_body = PrefInput,
    responses(
        (status = 204, description = "Saved"),
        (status = 422, description = "Invalid key", body = crate::error::Problem)
    )
)]
pub async fn pref_save(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(input): Json<PrefInput>,
) -> Result<StatusCode, ApiError> {
    state
        .settings
        .save_pref(&key, input.value.as_deref(), current_user())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
