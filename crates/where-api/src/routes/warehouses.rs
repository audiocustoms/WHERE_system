//! Warehouses and bin locations (`img/Admin Menü/Lager und Lagerplätze`).
//!
//! The mask has three tabs: the warehouses of one tenant, the location based
//! bin locations (`Lagerplatz (Symetric)`) and the freely named ones (`Lagerplatz (Self
//! defined)`). A warehouse belongs to a tenant through `"Mandanten_ID"` — without a
//! chosen tenant there is no warehouse list (`422`).

use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use where_persistence::{WarehouseFlags, WarehouseRepository};

use crate::error::ApiError;
use crate::state::{AppState, State};

/// One warehouse of a client, with the switches of the mask.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WarehouseDto {
    pub id: i32,
    /// `"Mandanten_ID"` — the client the warehouse belongs to.
    pub mandant_id: Option<i32>,
    pub name: Option<String>,
    /// `__DefaultLager`: the warehouse the forms preselect.
    pub is_default: bool,
    /// Zugriff
    pub zugriff: bool,
    /// Priorität
    pub prioritaet: bool,
    /// Such-Std.
    pub such_std: bool,
    /// Entnahme
    pub entnahme: bool,
    /// Anmelden
    pub anmelden: bool,
    /// Fremd
    pub fremd: bool,
    /// `Konzerninternes Lager oder ohne Dispo` — in-house warehouse or without
    /// disposition.
    pub konzern: bool,
}

impl From<where_persistence::WarehouseAdminRow> for WarehouseDto {
    fn from(w: where_persistence::WarehouseAdminRow) -> Self {
        Self {
            id: w.id,
            mandant_id: w.mandant_id,
            name: w.name,
            is_default: w.is_default,
            zugriff: w.zugriff,
            prioritaet: w.prioritaet,
            such_std: w.such_std,
            entnahme: w.entnahme,
            anmelden: w.anmelden,
            fremd: w.fremd,
            konzern: w.konzern,
        }
    }
}

/// A "symmetric" bin location: name, short code and range of values.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BinDto {
    pub id: i32,
    pub warehouse_id: i32,
    pub name: Option<String>,
    /// Kürzel
    pub prefix: Option<String>,
    /// Werte
    pub values: Option<String>,
    pub position: Option<i32>,
}

impl From<where_persistence::BinRow> for BinDto {
    fn from(b: where_persistence::BinRow) -> Self {
        Self {
            id: b.id,
            warehouse_id: b.warehouse_id,
            name: b.name,
            prefix: b.prefix,
            values: b.values,
            position: b.position,
        }
    }
}

/// A "self defined" bin location.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SubBinDto {
    pub id: i32,
    pub warehouse_id: i32,
    pub label: Option<String>,
    pub path: Option<String>,
    pub parent_id: Option<i32>,
}

impl From<where_persistence::SubBinRow> for SubBinDto {
    fn from(b: where_persistence::SubBinRow) -> Self {
        Self {
            id: b.id,
            warehouse_id: b.warehouse_id,
            label: b.label,
            path: b.path,
            parent_id: b.parent_id,
        }
    }
}

/// Query of the warehouse list — the client is required.
#[derive(Debug, Deserialize, IntoParams)]
pub struct WarehouseListParams {
    /// `"Mandanten"."Primary_Key"`; without it there is no list.
    pub mandant: i64,
}

/// Input of a new warehouse.
#[derive(Debug, Deserialize, ToSchema)]
pub struct WarehouseInput {
    pub mandant: i64,
    pub name: String,
}

/// Input of the changes of one warehouse.
#[derive(Debug, Deserialize, ToSchema)]
pub struct WarehouseUpdateInput {
    pub name: String,
    #[serde(default)]
    pub zugriff: bool,
    #[serde(default)]
    pub prioritaet: bool,
    #[serde(default)]
    pub such_std: bool,
    #[serde(default)]
    pub entnahme: bool,
    #[serde(default)]
    pub anmelden: bool,
    #[serde(default)]
    pub fremd: bool,
    #[serde(default)]
    pub konzern: bool,
}

/// Assignment of a warehouse to a client.
#[derive(Debug, Deserialize, ToSchema)]
pub struct WarehouseAssignInput {
    pub mandant: i64,
}

/// Input of a bin location.
#[derive(Debug, Deserialize, ToSchema)]
pub struct BinInput {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub values: String,
}

/// Input of a self defined bin location.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SubBinInput {
    #[serde(default)]
    pub label: String,
}

fn repository(state: &AppState) -> WarehouseRepository {
    WarehouseRepository::new(state.pool.clone())
}

/// Warehouses of one client. Without `?mandant=` the answer is `422` — the mask only
/// offers the assignment after a client was chosen.
#[utoipa::path(
    get,
    path = "/api/v1/warehouses",
    tag = "einstellungen",
    params(WarehouseListParams),
    responses(
        (status = 200, description = "Warehouses of the client", body = Vec<WarehouseDto>),
        (status = 422, description = "Client missing")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<WarehouseListParams>,
) -> Result<Json<Vec<WarehouseDto>>, ApiError> {
    if params.mandant <= 0 {
        return Err(ApiError(where_domain::DomainError::Validation(
            "Bitte zuerst einen Mandanten wählen".into(),
        )));
    }
    let rows = repository(&state).list(params.mandant).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Creates a warehouse for the client. The first one becomes its default.
#[utoipa::path(
    post,
    path = "/api/v1/warehouses",
    tag = "einstellungen",
    request_body = WarehouseInput,
    responses(
        (status = 201, description = "Angelegt", body = WarehouseDto),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<WarehouseInput>,
) -> Result<(StatusCode, Json<WarehouseDto>), ApiError> {
    if body.mandant <= 0 {
        return Err(ApiError(where_domain::DomainError::Validation(
            "Bitte zuerst einen Mandanten wählen".into(),
        )));
    }
    let row = repository(&state).create(body.mandant, &body.name).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Changes name and switches of a warehouse.
#[utoipa::path(
    put,
    path = "/api/v1/warehouses/{id}",
    tag = "einstellungen",
    params(("id" = i32, Path, description = "Warehouse")),
    request_body = WarehouseUpdateInput,
    responses(
        (status = 200, description = "Gespeichert", body = WarehouseDto),
        (status = 404, description = "Not found"),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<WarehouseUpdateInput>,
) -> Result<Json<WarehouseDto>, ApiError> {
    let flags = WarehouseFlags {
        zugriff: body.zugriff,
        prioritaet: body.prioritaet,
        such_std: body.such_std,
        entnahme: body.entnahme,
        anmelden: body.anmelden,
        fremd: body.fremd,
        konzern: body.konzern,
    };
    let row = repository(&state).update(id, &body.name, &flags).await?;
    Ok(Json(row.into()))
}

/// Assigns a warehouse to a client — the warehouses taken over from the legacy
/// installation carry no client yet.
#[utoipa::path(
    post,
    path = "/api/v1/warehouses/{id}/assign",
    tag = "einstellungen",
    params(("id" = i32, Path, description = "Warehouse")),
    request_body = WarehouseAssignInput,
    responses(
        (status = 200, description = "Zugeordnet", body = WarehouseDto),
        (status = 404, description = "Not found")
    )
)]
pub async fn assign(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<WarehouseAssignInput>,
) -> Result<Json<WarehouseDto>, ApiError> {
    if body.mandant <= 0 {
        return Err(ApiError(where_domain::DomainError::Validation(
            "Bitte zuerst einen Mandanten wählen".into(),
        )));
    }
    let row = repository(&state).assign(id, body.mandant).await?;
    Ok(Json(row.into()))
}

/// Deletes a warehouse — refused while devices or bin locations hang on it.
#[utoipa::path(
    delete,
    path = "/api/v1/warehouses/{id}",
    tag = "einstellungen",
    params(("id" = i32, Path, description = "Warehouse")),
    responses(
        (status = 200, description = "Deleted", body = crate::routes::addresses::DeletedResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Still in use")
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<crate::routes::addresses::DeletedResponse>, ApiError> {
    repository(&state).delete(id).await?;
    Ok(Json(crate::routes::addresses::DeletedResponse {
        deleted: true,
    }))
}

// ------------------------------------------------------- Bin locations (Symetric)

/// Bin locations of a warehouse (`Lagerplatz (Symetric)`).
#[utoipa::path(
    get,
    path = "/api/v1/warehouses/{id}/locations",
    tag = "einstellungen",
    params(("id" = i32, Path, description = "Warehouse")),
    responses((status = 200, description = "Bin locations", body = Vec<BinDto>))
)]
pub async fn locations(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<BinDto>>, ApiError> {
    let rows = repository(&state).bins(id).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Adds a bin location to the warehouse.
#[utoipa::path(
    post,
    path = "/api/v1/warehouses/{id}/locations",
    tag = "einstellungen",
    params(("id" = i32, Path, description = "Warehouse")),
    request_body = BinInput,
    responses(
        (status = 201, description = "Angelegt", body = BinDto),
        (status = 404, description = "Warehouse not found")
    )
)]
pub async fn location_create(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<BinInput>,
) -> Result<(StatusCode, Json<BinDto>), ApiError> {
    let row = repository(&state).bin_create(id, &body.name).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Changes a bin location.
#[utoipa::path(
    put,
    path = "/api/v1/warehouses/{id}/locations/{location_id}",
    tag = "einstellungen",
    params(
        ("id" = i32, Path, description = "Warehouse"),
        ("location_id" = i32, Path, description = "Bin location")
    ),
    request_body = BinInput,
    responses(
        (status = 200, description = "Gespeichert", body = BinDto),
        (status = 404, description = "Not found")
    )
)]
pub async fn location_update(
    State(state): State<AppState>,
    Path((_warehouse_id, location_id)): Path<(i32, i32)>,
    Json(body): Json<BinInput>,
) -> Result<Json<BinDto>, ApiError> {
    let row = repository(&state)
        .bin_update(location_id, &body.name, &body.prefix, &body.values)
        .await?;
    Ok(Json(row.into()))
}

/// Removes a bin location.
#[utoipa::path(
    delete,
    path = "/api/v1/warehouses/{id}/locations/{location_id}",
    tag = "einstellungen",
    params(
        ("id" = i32, Path, description = "Warehouse"),
        ("location_id" = i32, Path, description = "Bin location")
    ),
    responses(
        (status = 200, description = "Deleted", body = crate::routes::addresses::DeletedResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn location_delete(
    State(state): State<AppState>,
    Path((_warehouse_id, location_id)): Path<(i32, i32)>,
) -> Result<Json<crate::routes::addresses::DeletedResponse>, ApiError> {
    repository(&state).bin_delete(location_id).await?;
    Ok(Json(crate::routes::addresses::DeletedResponse {
        deleted: true,
    }))
}

// ---------------------------------------------------- Bin locations (Self defined)

/// Self defined bin locations of a warehouse.
#[utoipa::path(
    get,
    path = "/api/v1/warehouses/{id}/bins",
    tag = "einstellungen",
    params(("id" = i32, Path, description = "Warehouse")),
    responses((status = 200, description = "Bin locations", body = Vec<SubBinDto>))
)]
pub async fn bins(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<SubBinDto>>, ApiError> {
    let rows = repository(&state).sub_bins(id).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Adds a self defined bin location.
#[utoipa::path(
    post,
    path = "/api/v1/warehouses/{id}/bins",
    tag = "einstellungen",
    params(("id" = i32, Path, description = "Warehouse")),
    request_body = SubBinInput,
    responses(
        (status = 201, description = "Angelegt", body = SubBinDto),
        (status = 404, description = "Warehouse not found")
    )
)]
pub async fn bin_create(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<SubBinInput>,
) -> Result<(StatusCode, Json<SubBinDto>), ApiError> {
    let row = repository(&state).sub_bin_create(id, &body.label).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Renames a self defined bin location.
#[utoipa::path(
    put,
    path = "/api/v1/warehouses/{id}/bins/{bin_id}",
    tag = "einstellungen",
    params(
        ("id" = i32, Path, description = "Warehouse"),
        ("bin_id" = i32, Path, description = "Bin location")
    ),
    request_body = SubBinInput,
    responses(
        (status = 200, description = "Gespeichert", body = SubBinDto),
        (status = 404, description = "Not found")
    )
)]
pub async fn bin_update(
    State(state): State<AppState>,
    Path((_warehouse_id, bin_id)): Path<(i32, i32)>,
    Json(body): Json<SubBinInput>,
) -> Result<Json<SubBinDto>, ApiError> {
    let row = repository(&state)
        .sub_bin_update(bin_id, &body.label)
        .await?;
    Ok(Json(row.into()))
}

/// Removes a self defined bin location.
#[utoipa::path(
    delete,
    path = "/api/v1/warehouses/{id}/bins/{bin_id}",
    tag = "einstellungen",
    params(
        ("id" = i32, Path, description = "Warehouse"),
        ("bin_id" = i32, Path, description = "Bin location")
    ),
    responses(
        (status = 200, description = "Deleted", body = crate::routes::addresses::DeletedResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Has sub locations")
    )
)]
pub async fn bin_delete(
    State(state): State<AppState>,
    Path((_warehouse_id, bin_id)): Path<(i32, i32)>,
) -> Result<Json<crate::routes::addresses::DeletedResponse>, ApiError> {
    repository(&state).sub_bin_delete(bin_id).await?;
    Ok(Json(crate::routes::addresses::DeletedResponse {
        deleted: true,
    }))
}
