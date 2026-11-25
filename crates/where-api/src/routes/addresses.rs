use crate::state::State;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use utoipa::ToSchema;

use crate::dto::{
    AddressDto, AddressInputDto, AddressListParams, AddressPage, AddressSearchInput, ListParams,
};
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, serde::Serialize, ToSchema)]
pub struct DeletedResponse {
    pub deleted: bool,
}

/// List addresses with paging, quick search, active filter and sorting.
#[utoipa::path(
    get,
    path = "/api/v1/addresses",
    tag = "addresses",
    params(AddressListParams),
    responses(
        (status = 200, description = "One page of addresses", body = AddressPage),
        (status = 422, description = "Unknown filter or sort value"),
        (status = 500, description = "Internal error")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<AddressListParams>,
) -> Result<Json<AddressPage>, ApiError> {
    let page = state.addresses.list(&params.into_list()?).await?;
    Ok(Json(AddressPage {
        items: page.items.into_iter().map(Into::into).collect(),
        total: page.total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// Duplicate check of the "Dublettenverhinderer" window (`img/adressen/Dublettenverhinderer.PNG`).
///
/// One search term, matched broadly; an empty term lists the first addresses.
#[utoipa::path(
    get,
    path = "/api/v1/addresses/duplicates",
    tag = "addresses",
    params(
        ("q" = Option<String>, Query, description = "Search term of the new address"),
        ("limit" = Option<i64>, Query, description = "Maximum number of hits (default 25)")
    ),
    responses(
        (status = 200, description = "Possible duplicates", body = AddressPage),
        (status = 500, description = "Internal error")
    )
)]
pub async fn duplicates(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<AddressPage>, ApiError> {
    let limit = params.limit.unwrap_or(25).clamp(1, 100);
    let page = state.addresses.duplicates(params.search(), limit).await?;
    Ok(Json(AddressPage {
        items: page.items.into_iter().map(Into::into).collect(),
        total: page.total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// Search addresses with the criteria of the search form.
#[utoipa::path(
    post,
    path = "/api/v1/addresses/search",
    tag = "addresses",
    request_body = AddressSearchInput,
    responses(
        (status = 200, description = "One page of hits", body = AddressPage),
        (status = 422, description = "Unknown criteria value"),
        (status = 500, description = "Internal error")
    )
)]
pub async fn search(
    State(state): State<AppState>,
    Json(body): Json<AddressSearchInput>,
) -> Result<Json<AddressPage>, ApiError> {
    let criteria = body.criteria()?;
    let (limit, offset) = body.window();
    let page = state.addresses.search(&criteria, limit, offset).await?;
    Ok(Json(AddressPage {
        items: page.items.into_iter().map(Into::into).collect(),
        total: page.total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// Fetch a single address.
#[utoipa::path(
    get,
    path = "/api/v1/addresses/{id}",
    tag = "addresses",
    params(("id" = i32, Path, description = "Adresse primary key")),
    responses(
        (status = 200, description = "The address", body = AddressDto),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AddressDto>, ApiError> {
    Ok(Json(state.addresses.get(id).await?.into()))
}

/// Create an address.
#[utoipa::path(
    post,
    path = "/api/v1/addresses",
    tag = "addresses",
    request_body = AddressInputDto,
    responses(
        (status = 201, description = "Created", body = AddressDto),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<AddressInputDto>,
) -> Result<(StatusCode, Json<AddressDto>), ApiError> {
    let created = state.addresses.create(&body.into()).await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

/// Replace an address (full PUT semantics).
#[utoipa::path(
    put,
    path = "/api/v1/addresses/{id}",
    tag = "addresses",
    params(("id" = i32, Path, description = "Adresse primary key")),
    request_body = AddressInputDto,
    responses(
        (status = 200, description = "Replaced", body = AddressDto),
        (status = 404, description = "Not found"),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn replace(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<AddressInputDto>,
) -> Result<Json<AddressDto>, ApiError> {
    Ok(Json(state.addresses.update(id, &body.into()).await?.into()))
}

/// Delete an address.
#[utoipa::path(
    delete,
    path = "/api/v1/addresses/{id}",
    tag = "addresses",
    params(("id" = i32, Path, description = "Adresse primary key")),
    responses(
        (status = 200, description = "Deleted", body = DeletedResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.addresses.delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}
