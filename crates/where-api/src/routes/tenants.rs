//! Tenants of the installation (`img/Admin Menü/Mandanten`).
//!
//! The window `Einstellungen Mandanten` lists the tenants and edits one of them on
//! two tabs. The two blocks of options below the list belong to the installation and
//! not to a tenant; they are read and written through `/api/v1/prefs` with the prefix
//! `mandanten.` like the options of the other masks.

use crate::state::State;
use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use where_domain::{Tenant, TenantInput, TenantReference};

use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::AppState;

/// All tenants, by company name.
#[utoipa::path(
    get,
    path = "/api/v1/tenants",
    tag = "mandanten",
    responses((status = 200, body = Vec<Tenant>))
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Tenant>>, ApiError> {
    Ok(Json(state.tenants.list().await?))
}

/// The countries and print languages of the mask.
#[utoipa::path(
    get,
    path = "/api/v1/tenants/reference",
    tag = "mandanten",
    responses((status = 200, body = TenantReference))
)]
pub async fn reference() -> Json<TenantReference> {
    Json(TenantReference::new())
}

/// One tenant with all columns of the two tabs.
#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}",
    tag = "mandanten",
    params(("id" = i64, Path, description = "Key of the tenant")),
    responses(
        (status = 200, body = Tenant),
        (status = 404, description = "Unknown tenant", body = crate::error::Problem)
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Tenant>, ApiError> {
    Ok(Json(state.tenants.get(id).await?))
}

/// Adds a tenant.
#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    tag = "mandanten",
    request_body = TenantInput,
    responses(
        (status = 201, body = Tenant),
        (status = 422, description = "Missing company name or negative Ident.-Nr.", body = crate::error::Problem)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<TenantInput>,
) -> Result<(StatusCode, Json<Tenant>), ApiError> {
    let tenant = state.tenants.create(input).await?;
    Ok((StatusCode::CREATED, Json(tenant)))
}

/// Changes a tenant.
#[utoipa::path(
    put,
    path = "/api/v1/tenants/{id}",
    tag = "mandanten",
    params(("id" = i64, Path, description = "Key of the tenant")),
    request_body = TenantInput,
    responses(
        (status = 200, body = Tenant),
        (status = 404, description = "Unknown tenant", body = crate::error::Problem),
        (status = 422, description = "Missing company name or negative Ident.-Nr.", body = crate::error::Problem)
    )
)]
pub async fn replace(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<TenantInput>,
) -> Result<Json<Tenant>, ApiError> {
    Ok(Json(state.tenants.update(id, input).await?))
}

/// Removes a tenant.
#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{id}",
    tag = "mandanten",
    params(("id" = i64, Path, description = "Key of the tenant")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown tenant", body = crate::error::Problem)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.tenants.delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}
