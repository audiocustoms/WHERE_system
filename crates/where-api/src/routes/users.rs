//! Users and permission groups (`img/Admin Menü/Benutzerverwaltung`).
//!
//! The main window lists the accounts of one view (`Aktuelle Ansicht`) and the groups
//! of the selected account; the dialog behind `+` edits an account on four tabs, the
//! dialog behind the group buttons edits a group with the rights tree of the
//! catalogue.

use crate::state::State;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use serde::Deserialize;
use where_domain::{
    PermissionGroup, User, UserGroup, UserGroupInput, UserInput, UserSummary, UserView,
};

use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::AppState;

/// Which list of `Aktuelle Ansicht` is asked for.
#[derive(Debug, Deserialize)]
pub struct ViewParams {
    /// `local-active`, `mobile-active`, `local-inactive` or `mobile-inactive`.
    pub view: Option<String>,
}

/// The rights tree of the group dialog.
#[utoipa::path(
    get,
    path = "/api/v1/permissions",
    tag = "benutzer",
    responses((status = 200, body = Vec<PermissionGroup>))
)]
pub async fn permissions() -> Json<Vec<PermissionGroup>> {
    Json(where_domain::catalogue())
}

/// The users of one view, each with the groups it belongs to.
#[utoipa::path(
    get,
    path = "/api/v1/users",
    tag = "benutzer",
    params(("view" = Option<String>, Query, description = "local-active (default), mobile-active, local-inactive, mobile-inactive")),
    responses(
        (status = 200, body = Vec<UserSummary>),
        (status = 422, description = "Unknown view", body = crate::error::Problem)
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<ViewParams>,
) -> Result<Json<Vec<UserSummary>>, ApiError> {
    let text = params.view.as_deref().unwrap_or("");
    let view = UserView::parse(text).ok_or_else(|| {
        ApiError::from(where_domain::DomainError::Validation(format!(
            "„{text}\u{201c} ist keine Ansicht (local-active, mobile-active, local-inactive, mobile-inactive)."
        )))
    })?;
    Ok(Json(state.users.list(view).await?))
}

/// One account with all columns of the four tabs.
#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    tag = "benutzer",
    params(("id" = i64, Path, description = "Key of the account")),
    responses(
        (status = 200, body = User),
        (status = 404, description = "Unknown account", body = crate::error::Problem)
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(state.users.get(id).await?))
}

/// Adds an account.
#[utoipa::path(
    post,
    path = "/api/v1/users",
    tag = "benutzer",
    request_body = UserInput,
    responses(
        (status = 201, body = User),
        (status = 422, description = "Invalid login name, kind of login, short name or colour", body = crate::error::Problem)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<UserInput>,
) -> Result<(StatusCode, Json<User>), ApiError> {
    let user = state.users.create(input).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

/// Changes an account.
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    tag = "benutzer",
    params(("id" = i64, Path, description = "Key of the account")),
    request_body = UserInput,
    responses(
        (status = 200, body = User),
        (status = 404, description = "Unknown account", body = crate::error::Problem),
        (status = 422, description = "Invalid login name, kind of login, short name or colour", body = crate::error::Problem)
    )
)]
pub async fn replace(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UserInput>,
) -> Result<Json<User>, ApiError> {
    Ok(Json(state.users.update(id, input).await?))
}

/// Removes an account with its assignments and its contact data.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    tag = "benutzer",
    params(("id" = i64, Path, description = "Key of the account")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown account", body = crate::error::Problem)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.users.delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Puts an account into a group.
#[utoipa::path(
    put,
    path = "/api/v1/users/{id}/groups/{group}",
    tag = "benutzer",
    params(
        ("id" = i64, Path, description = "Key of the account"),
        ("group" = i64, Path, description = "Key of the group")
    ),
    responses(
        (status = 200, body = User),
        (status = 404, description = "Unknown account or group", body = crate::error::Problem)
    )
)]
pub async fn group_add(
    State(state): State<AppState>,
    Path((id, group)): Path<(i64, i64)>,
) -> Result<Json<User>, ApiError> {
    state.users.add_to_group(id, group).await?;
    Ok(Json(state.users.get(id).await?))
}

/// Takes an account out of a group.
#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}/groups/{group}",
    tag = "benutzer",
    params(
        ("id" = i64, Path, description = "Key of the account"),
        ("group" = i64, Path, description = "Key of the group")
    ),
    responses((status = 200, body = User))
)]
pub async fn group_unassign(
    State(state): State<AppState>,
    Path((id, group)): Path<(i64, i64)>,
) -> Result<Json<User>, ApiError> {
    state.users.remove_from_group(id, group).await?;
    Ok(Json(state.users.get(id).await?))
}

/// Every group with its rights, its scope and its accounts — the right-hand tree.
#[utoipa::path(
    get,
    path = "/api/v1/user-groups",
    tag = "benutzer",
    responses((status = 200, body = Vec<UserGroup>))
)]
pub async fn group_list(State(state): State<AppState>) -> Result<Json<Vec<UserGroup>>, ApiError> {
    Ok(Json(state.users.groups().await?))
}

/// One group with its rights and its scope.
#[utoipa::path(
    get,
    path = "/api/v1/user-groups/{id}",
    tag = "benutzer",
    params(("id" = i64, Path, description = "Key of the group")),
    responses(
        (status = 200, body = UserGroup),
        (status = 404, description = "Unknown group", body = crate::error::Problem)
    )
)]
pub async fn group_get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<UserGroup>, ApiError> {
    Ok(Json(state.users.group(id).await?))
}

/// Adds a group.
#[utoipa::path(
    post,
    path = "/api/v1/user-groups",
    tag = "benutzer",
    request_body = UserGroupInput,
    responses(
        (status = 201, body = UserGroup),
        (status = 422, description = "Missing group name", body = crate::error::Problem)
    )
)]
pub async fn group_create(
    State(state): State<AppState>,
    Json(input): Json<UserGroupInput>,
) -> Result<(StatusCode, Json<UserGroup>), ApiError> {
    let group = state.users.create_group(input).await?;
    Ok((StatusCode::CREATED, Json(group)))
}

/// Changes a group with its rights and its scope.
#[utoipa::path(
    put,
    path = "/api/v1/user-groups/{id}",
    tag = "benutzer",
    params(("id" = i64, Path, description = "Key of the group")),
    request_body = UserGroupInput,
    responses(
        (status = 200, body = UserGroup),
        (status = 404, description = "Unknown group", body = crate::error::Problem),
        (status = 422, description = "Missing group name", body = crate::error::Problem)
    )
)]
pub async fn group_replace(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<UserGroupInput>,
) -> Result<Json<UserGroup>, ApiError> {
    Ok(Json(state.users.update_group(id, input).await?))
}

/// Copies a group under a new name.
#[utoipa::path(
    post,
    path = "/api/v1/user-groups/{id}/copy",
    tag = "benutzer",
    params(("id" = i64, Path, description = "Key of the group")),
    responses(
        (status = 201, body = UserGroup),
        (status = 404, description = "Unknown group", body = crate::error::Problem)
    )
)]
pub async fn group_copy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Json<UserGroup>), ApiError> {
    let group = state.users.copy_group(id).await?;
    Ok((StatusCode::CREATED, Json(group)))
}

/// Removes a group with its rights, its scope and its memberships.
#[utoipa::path(
    delete,
    path = "/api/v1/user-groups/{id}",
    tag = "benutzer",
    params(("id" = i64, Path, description = "Key of the group")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown group", body = crate::error::Problem)
    )
)]
pub async fn group_remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.users.delete_group(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}
