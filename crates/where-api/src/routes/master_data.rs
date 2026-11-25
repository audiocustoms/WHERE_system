use crate::state::State;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use where_persistence::{ArticleGroupEntry, NumberSeriesEntry};

use crate::dto::{
    ArticleGroupDto, ArticleGroupInput, LanguageDto, ListParams, NumberSeriesDto,
    NumberSeriesInput, SearchIndexNodeDto, WarehouseDto,
};
use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::AppState;

impl From<ArticleGroupEntry> for ArticleGroupDto {
    fn from(w: ArticleGroupEntry) -> Self {
        Self {
            id: w.id,
            name: w.name,
            parent_id: w.parent_id,
            article_count: w.article_count,
        }
    }
}

/// Article groups with their parent group and the number of assigned articles.
#[utoipa::path(
    get,
    path = "/api/v1/master-data/article-groups",
    tag = "master-data",
    params(ListParams),
    responses((status = 200, description = "Article groups", body = Vec<ArticleGroupDto>))
)]
pub async fn article_groups(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<ArticleGroupDto>>, ApiError> {
    let rows = state.article_groups.list(params.search()).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Creates an article group.
#[utoipa::path(
    post,
    path = "/api/v1/master-data/article-groups",
    tag = "master-data",
    request_body = ArticleGroupInput,
    responses(
        (status = 201, description = "Created", body = ArticleGroupDto),
        (status = 409, description = "Name already taken"),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn article_group_create(
    State(state): State<AppState>,
    Json(body): Json<ArticleGroupInput>,
) -> Result<(StatusCode, Json<ArticleGroupDto>), ApiError> {
    let created = state
        .article_groups
        .create(&body.name, body.parent_id)
        .await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

/// Changes the name and/or the parent group.
#[utoipa::path(
    put,
    path = "/api/v1/master-data/article-groups/{id}",
    tag = "master-data",
    params(("id" = i32, Path, description = "Article group id")),
    request_body = ArticleGroupInput,
    responses(
        (status = 200, description = "Updated", body = ArticleGroupDto),
        (status = 404, description = "Not found"),
        (status = 409, description = "Name already taken"),
        (status = 422, description = "Validation failed (for example a cycle)")
    )
)]
pub async fn article_group_update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<ArticleGroupInput>,
) -> Result<Json<ArticleGroupDto>, ApiError> {
    let updated = state
        .article_groups
        .update(id, &body.name, body.parent_id)
        .await?;
    Ok(Json(updated.into()))
}

/// Deletes an article group. Refused while articles use it or subgroups exist.
#[utoipa::path(
    delete,
    path = "/api/v1/master-data/article-groups/{id}",
    tag = "master-data",
    params(("id" = i32, Path, description = "Article group id")),
    responses(
        (status = 200, description = "Deleted", body = DeletedResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "In use or has subgroups")
    )
)]
pub async fn article_group_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.article_groups.delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

impl From<NumberSeriesEntry> for NumberSeriesDto {
    fn from(k: NumberSeriesEntry) -> Self {
        let (next_value, next_number) = match k.preview_today() {
            Some((value, text)) => (Some(value), Some(text)),
            None => (None, None),
        };
        Self {
            key: k.key,
            name: k.name,
            prefix: k.prefix,
            format: k.format,
            digits: k.digits,
            start_value: k.start_value,
            step: k.step,
            current_value: k.current_value,
            yearly_reset: k.yearly_reset,
            year: k.year,
            active: k.active,
            legacy_source: k.legacy_source,
            changed_at: k.changed_at.to_rfc3339(),
            next_value,
            next_number,
        }
    }
}

/// All number series with their counter and a preview of the next number.
#[utoipa::path(
    get,
    path = "/api/v1/master-data/number-series",
    tag = "master-data",
    responses((status = 200, description = "Number series", body = Vec<NumberSeriesDto>))
)]
pub async fn number_series_list(
    State(state): State<AppState>,
) -> Result<Json<Vec<NumberSeriesDto>>, ApiError> {
    let rows = state.number_series.list().await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// A single number series.
#[utoipa::path(
    get,
    path = "/api/v1/master-data/number-series/{key}",
    tag = "master-data",
    params(("key" = String, Path, description = "For example article")),
    responses(
        (status = 200, description = "Number series", body = NumberSeriesDto),
        (status = 404, description = "Unknown key")
    )
)]
pub async fn number_series_get(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<NumberSeriesDto>, ApiError> {
    let row = state.number_series.get(&key).await?;
    Ok(Json(row.into()))
}

/// Changes prefix, format, digits, start value, step and counter.
#[utoipa::path(
    put,
    path = "/api/v1/master-data/number-series/{key}",
    tag = "master-data",
    params(("key" = String, Path, description = "For example article")),
    request_body = NumberSeriesInput,
    responses(
        (status = 200, description = "Updated", body = NumberSeriesDto),
        (status = 404, description = "Unknown key"),
        (status = 422, description = "Validation failed")
    )
)]
pub async fn number_series_update(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(body): Json<NumberSeriesInput>,
) -> Result<Json<NumberSeriesDto>, ApiError> {
    let row = state
        .number_series
        .update(
            &key,
            &body.name,
            &body.prefix,
            &body.format,
            body.digits,
            body.start_value,
            body.step,
            body.yearly_reset,
            body.active,
            body.current_value,
        )
        .await?;
    Ok(Json(row.into()))
}

/// Resets the counter — the next number is the start value again.
#[utoipa::path(
    post,
    path = "/api/v1/master-data/number-series/{key}/reset",
    tag = "master-data",
    params(("key" = String, Path, description = "For example article")),
    responses(
        (status = 200, description = "Reset", body = NumberSeriesDto),
        (status = 404, description = "Unknown key")
    )
)]
pub async fn number_series_reset(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<NumberSeriesDto>, ApiError> {
    let row = state.number_series.reset(&key).await?;
    Ok(Json(row.into()))
}

/// Languages for the language switcher of the article form.
#[utoipa::path(
    get,
    path = "/api/v1/master-data/languages",
    tag = "master-data",
    responses((status = 200, description = "Languages", body = Vec<LanguageDto>))
)]
pub async fn languages(State(state): State<AppState>) -> Result<Json<Vec<LanguageDto>>, ApiError> {
    let rows = state.article.languages().await?;
    Ok(Json(
        rows.into_iter()
            .map(|s| LanguageDto {
                id: s.id,
                language: s.language,
                native_language: s.native_language.unwrap_or(false),
                iso_639_1: s.iso_639_1,
            })
            .collect(),
    ))
}

/// Search index tree for the "select index" dialog of the article search.
#[utoipa::path(
    get,
    path = "/api/v1/master-data/search-index",
    tag = "master-data",
    responses((status = 200, description = "Search index nodes", body = Vec<SearchIndexNodeDto>))
)]
pub async fn search_index(
    State(state): State<AppState>,
) -> Result<Json<Vec<SearchIndexNodeDto>>, ApiError> {
    let rows = state.article_search.search_index().await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| SearchIndexNodeDto {
                id: r.id,
                name: r.name,
                parent_id: r.parent_id,
            })
            .collect(),
    ))
}

/// Warehouses of the "Lager" select (package slip and stock forms).
#[utoipa::path(
    get,
    path = "/api/v1/master-data/warehouses",
    tag = "master-data",
    responses((status = 200, description = "Warehouses", body = Vec<WarehouseDto>))
)]
pub async fn warehouses(
    State(state): State<AppState>,
) -> Result<Json<Vec<WarehouseDto>>, ApiError> {
    let rows = state.stock.warehouses().await?;
    Ok(Json(
        rows.into_iter()
            .map(|w| WarehouseDto {
                id: w.id,
                name: w.name,
                is_default: w.is_default,
            })
            .collect(),
    ))
}

/// Search index values of the addresses ("Index wählen" of the address search).
#[utoipa::path(
    get,
    path = "/api/v1/master-data/address-index",
    tag = "master-data",
    responses((status = 200, description = "Distinct index values", body = Vec<String>))
)]
pub async fn address_index(State(state): State<AppState>) -> Result<Json<Vec<String>>, ApiError> {
    Ok(Json(state.addresses.index_values().await?))
}
