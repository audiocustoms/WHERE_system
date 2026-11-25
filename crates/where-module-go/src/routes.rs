//! Settings of the order mask (`img/Admin Menü/Aufträge`).
//!
//! WHERE has no order engine yet; the endpoints exist so a later order module — and
//! the settings window itself — already have a stable interface. The switches and
//! numbers of the `Regeln` and `Zeiten` tabs are options and travel through
//! `/api/v1/prefs?prefix=order.`.

use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use serde::Deserialize;
use where_api::ApiError;
use where_api::routes::addresses::DeletedResponse;
use where_api::state::{AppState, State};
use where_domain::{ScaleCategory, ScaleStep};
use where_persistence::ArticleRepository;

use crate::order_settings::{
    HeadingEntry, HeadingInput, MacroInput, OrderMacro, TimelineEntry, TimelineInput,
};
use crate::order_settings_repo::OrderSettingsRepository;

fn order_settings(state: &AppState) -> OrderSettingsRepository {
    OrderSettingsRepository::new(state.pool.clone())
}

fn articles(state: &AppState) -> ArticleRepository {
    ArticleRepository::new(state.pool.clone())
}

/// Query of the scale list: `?for_sale=true` switches to the quantity scales.
#[derive(Debug, Default, Deserialize, utoipa::IntoParams)]
pub struct ScaleQuery {
    /// `true` = quantity scales of the sale, `false` (default) = factor scales of the rental.
    pub for_sale: Option<bool>,
}

impl ScaleQuery {
    fn for_sale(&self) -> bool {
        self.for_sale.unwrap_or(false)
    }
}

/// Scales of the "Staffel" dropdown: factor scales of the rental or quantity
/// scales of the sale. Both belong to this module, not to the core.
#[utoipa::path(
    get,
    path = "/api/v1/master-data/scales",
    tag = "auftraege",
    params(("for_sale" = Option<bool>, Query, description = "Sale quantity scales instead of rental factor scales")),
    responses((status = 200, description = "Scale categories", body = Vec<ScaleCategory>))
)]
pub async fn scales(
    State(state): State<AppState>,
    Query(params): Query<ScaleQuery>,
) -> Result<Json<Vec<ScaleCategory>>, ApiError> {
    Ok(Json(
        articles(&state).scale_categories(params.for_sale()).await?,
    ))
}

/// Steps of one scale — days of use and factor (rental) or quantity and discount (sale).
#[utoipa::path(
    get,
    path = "/api/v1/master-data/scales/{id}/steps",
    tag = "auftraege",
    params(("id" = i32, Path, description = "Scale category primary key")),
    responses((status = 200, description = "Scale steps", body = Vec<ScaleStep>))
)]
pub async fn scale_steps(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<ScaleStep>>, ApiError> {
    Ok(Json(articles(&state).scale_steps(id).await?))
}

/// Until WHERE has an authentication, every change is booked on this name.
fn current_user() -> &'static str {
    "where"
}

/// Default time plan of a new order ("Zeitplan").
#[utoipa::path(
    get,
    path = "/api/v1/orders/timeline",
    tag = "auftraege",
    responses((status = 200, body = Vec<TimelineEntry>))
)]
pub async fn timeline(State(state): State<AppState>) -> Result<Json<Vec<TimelineEntry>>, ApiError> {
    Ok(Json(order_settings(&state).timeline().await?))
}

/// Shows or hides one step of the time plan.
#[utoipa::path(
    put,
    path = "/api/v1/orders/timeline/{key}",
    tag = "auftraege",
    params(("key" = String, Path, description = "Step, e.g. `dispo`")),
    request_body = TimelineInput,
    responses(
        (status = 200, body = TimelineEntry),
        (status = 404, description = "Unknown step", body = where_api::error::Problem),
        (status = 422, description = "Invalid name", body = where_api::error::Problem)
    )
)]
pub async fn timeline_save(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(input): Json<TimelineInput>,
) -> Result<Json<TimelineEntry>, ApiError> {
    Ok(Json(
        order_settings(&state).save_timeline(&key, input).await?,
    ))
}

/// The nine headings of the order form ("Überschriften").
#[utoipa::path(
    get,
    path = "/api/v1/orders/headings",
    tag = "auftraege",
    responses((status = 200, body = Vec<HeadingEntry>))
)]
pub async fn headings(State(state): State<AppState>) -> Result<Json<Vec<HeadingEntry>>, ApiError> {
    Ok(Json(order_settings(&state).headings().await?))
}

/// Shows or hides a heading and stores its text.
#[utoipa::path(
    put,
    path = "/api/v1/orders/headings/{number}",
    tag = "auftraege",
    params(("number" = i32, Path, description = "Heading 1 to 9")),
    request_body = HeadingInput,
    responses(
        (status = 200, body = HeadingEntry),
        (status = 404, description = "Unknown heading", body = where_api::error::Problem),
        (status = 422, description = "Invalid text", body = where_api::error::Problem)
    )
)]
pub async fn heading_save(
    State(state): State<AppState>,
    Path(number): Path<i32>,
    Json(input): Json<HeadingInput>,
) -> Result<Json<HeadingEntry>, ApiError> {
    Ok(Json(
        order_settings(&state).save_heading(number, input).await?,
    ))
}

/// The macro list ("Makros").
#[utoipa::path(
    get,
    path = "/api/v1/orders/macros",
    tag = "auftraege",
    responses((status = 200, body = Vec<OrderMacro>))
)]
pub async fn macros(State(state): State<AppState>) -> Result<Json<Vec<OrderMacro>>, ApiError> {
    Ok(Json(order_settings(&state).macros().await?))
}

/// Adds a macro.
#[utoipa::path(
    post,
    path = "/api/v1/orders/macros",
    tag = "auftraege",
    request_body = MacroInput,
    responses(
        (status = 201, body = OrderMacro),
        (status = 422, description = "Invalid label", body = where_api::error::Problem)
    )
)]
pub async fn macro_create(
    State(state): State<AppState>,
    Json(input): Json<MacroInput>,
) -> Result<(StatusCode, Json<OrderMacro>), ApiError> {
    let macro_entry = order_settings(&state)
        .create_macro(input, current_user())
        .await?;
    Ok((StatusCode::CREATED, Json(macro_entry)))
}

/// Changes a macro.
#[utoipa::path(
    put,
    path = "/api/v1/orders/macros/{id}",
    tag = "auftraege",
    params(("id" = i64, Path, description = "Key of the macro")),
    request_body = MacroInput,
    responses(
        (status = 200, body = OrderMacro),
        (status = 404, description = "Unknown macro", body = where_api::error::Problem),
        (status = 422, description = "Invalid label", body = where_api::error::Problem)
    )
)]
pub async fn macro_update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<MacroInput>,
) -> Result<Json<OrderMacro>, ApiError> {
    Ok(Json(order_settings(&state).update_macro(id, input).await?))
}

/// Removes a macro.
#[utoipa::path(
    delete,
    path = "/api/v1/orders/macros/{id}",
    tag = "auftraege",
    params(("id" = i64, Path, description = "Key of the macro")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown macro", body = where_api::error::Problem)
    )
)]
pub async fn macro_delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    order_settings(&state).delete_macro(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}
