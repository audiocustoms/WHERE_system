//! Stocktaking (`Inventur`): counting one warehouse against the book.
//!
//! The mask is `/stocktaking`: pick a warehouse, start counting, enter what is on the
//! shelf, finish — the differences go into the stock history as corrections.

use axum::Json;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::SessionUser;
use where_persistence::{StocktakeLineRow, StocktakeRow};

use crate::error::ApiError;
use crate::state::{AppState, State};

/// One stocktake with its counters.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StocktakeDto {
    pub id: i32,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub name: String,
    pub note: Option<String>,
    /// `open`, `closed` or `cancelled`.
    pub state: String,
    pub started_at: String,
    pub started_by: String,
    pub finished_at: Option<String>,
    pub finished_by: Option<String>,
    /// Lines in the book.
    pub lines: i64,
    /// Lines that were counted.
    pub counted: i64,
    /// Counted lines with a difference.
    pub differences: i64,
}

/// One line: a device/batch as the book had it, plus what was counted.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StocktakeLineDto {
    pub id: i32,
    pub device_id: i32,
    pub article_id: i32,
    pub article_no: Option<i32>,
    pub article_name: Option<String>,
    pub serial_no: Option<String>,
    pub bin_location: Option<String>,
    /// Amount the book said when the stocktake was started.
    pub book_quantity: String,
    /// What was counted; `null` while the line is open.
    pub counted_quantity: Option<String>,
    /// `counted − book`; `null` while the line is open.
    pub difference: Option<String>,
    pub counted_at: Option<String>,
    pub counted_by: Option<String>,
    pub note: Option<String>,
    /// The device was removed while the stocktake was open.
    pub device_gone: bool,
}

/// Stocktake with all its lines.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StocktakeDetailDto {
    pub stocktake: StocktakeDto,
    pub lines: Vec<StocktakeLineDto>,
}

/// Starts a stocktake of one warehouse.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StocktakeInput {
    pub warehouse_id: i32,
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// Counts one line. `counted: null` clears the count again.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct StocktakeCountInput {
    #[serde(default)]
    pub counted: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub note: Option<String>,
}

fn at(value: Option<chrono::DateTime<chrono::Utc>>) -> Option<String> {
    value.map(|d| {
        d.with_timezone(&chrono::Local)
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string()
    })
}

impl From<StocktakeRow> for StocktakeDto {
    fn from(s: StocktakeRow) -> Self {
        Self {
            id: s.id,
            warehouse_id: s.warehouse_id,
            warehouse_name: s.warehouse_name,
            name: s.name,
            note: s.note,
            state: s.state,
            started_at: s
                .started_at
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            started_by: s.started_by,
            finished_at: at(s.finished_at),
            finished_by: s.finished_by,
            lines: s.lines,
            counted: s.counted,
            differences: s.differences,
        }
    }
}

impl From<StocktakeLineRow> for StocktakeLineDto {
    fn from(l: StocktakeLineRow) -> Self {
        let difference = l.difference();
        Self {
            id: l.id,
            device_id: l.device_id,
            article_id: l.article_id,
            article_no: l.article_no,
            article_name: l.article_name,
            serial_no: l.serial_no,
            bin_location: l.bin_location,
            book_quantity: l.book_quantity.to_string(),
            counted_quantity: l.counted_quantity.map(|d| d.to_string()),
            difference: difference.map(|d| d.to_string()),
            counted_at: at(l.counted_at),
            counted_by: l.counted_by,
            note: l.note,
            device_gone: l.device_gone,
        }
    }
}

/// All stocktakes, newest first.
#[utoipa::path(
    get,
    path = "/api/v1/stocktaking",
    tag = "articles",
    responses((status = 200, description = "Stocktakes", body = Vec<StocktakeDto>))
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<StocktakeDto>>, ApiError> {
    let rows = state.stocktakes.list().await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// One stocktake with its lines.
#[utoipa::path(
    get,
    path = "/api/v1/stocktaking/{id}",
    tag = "articles",
    params(("id" = i32, Path, description = "Stocktake")),
    responses(
        (status = 200, description = "Stocktake and lines", body = StocktakeDetailDto),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<StocktakeDetailDto>, ApiError> {
    let row = state.stocktakes.get(id).await?;
    let lines = state.stocktakes.lines(id).await?;
    Ok(Json(StocktakeDetailDto {
        stocktake: row.into(),
        lines: lines.into_iter().map(Into::into).collect(),
    }))
}

/// Starts a stocktake: the lines are the devices and batches the book has for this
/// warehouse.
#[utoipa::path(
    post,
    path = "/api/v1/stocktaking",
    tag = "articles",
    request_body = StocktakeInput,
    responses(
        (status = 201, description = "Gestartet", body = StocktakeDto),
        (status = 404, description = "Warehouse not found"),
        (status = 409, description = "Für dieses Lager läuft schon eine Inventur"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn start(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Json(body): Json<StocktakeInput>,
) -> Result<(StatusCode, Json<StocktakeDto>), ApiError> {
    let row = state
        .stocktakes
        .start(
            body.warehouse_id,
            &body.name,
            body.note.as_deref(),
            &user.user_name,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Counts one line of a stocktake.
#[utoipa::path(
    put,
    path = "/api/v1/stocktaking/{id}/lines/{line_id}",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Stocktake"),
        ("line_id" = i32, Path, description = "Line")
    ),
    request_body = StocktakeCountInput,
    responses(
        (status = 204, description = "Gezählt"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Diese Inventur ist abgeschlossen"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn count(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path((id, line_id)): Path<(i32, i32)>,
    Json(body): Json<StocktakeCountInput>,
) -> Result<StatusCode, ApiError> {
    state
        .stocktakes
        .count(
            id,
            line_id,
            body.counted,
            body.note.as_deref(),
            &user.user_name,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Closes the stocktake and books every counted difference into the stock history.
#[utoipa::path(
    post,
    path = "/api/v1/stocktaking/{id}/finish",
    tag = "articles",
    params(("id" = i32, Path, description = "Stocktake")),
    responses(
        (status = 200, description = "Abgeschlossen", body = StocktakeDto),
        (status = 404, description = "Not found"),
        (status = 409, description = "Bereits abgeschlossen")
    )
)]
pub async fn finish(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path(id): Path<i32>,
) -> Result<Json<StocktakeDto>, ApiError> {
    let row = state.stocktakes.finish(id, &user.user_name).await?;
    Ok(Json(row.into()))
}

/// Drops the stocktake without booking anything.
#[utoipa::path(
    post,
    path = "/api/v1/stocktaking/{id}/cancel",
    tag = "articles",
    params(("id" = i32, Path, description = "Stocktake")),
    responses(
        (status = 200, description = "Abgebrochen", body = StocktakeDto),
        (status = 404, description = "Not found"),
        (status = 409, description = "Bereits abgeschlossen")
    )
)]
pub async fn cancel(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path(id): Path<i32>,
) -> Result<Json<StocktakeDto>, ApiError> {
    let row = state.stocktakes.cancel(id, &user.user_name).await?;
    Ok(Json(row.into()))
}
