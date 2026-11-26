//! Purchase orders (`Bestellungen`) of the warehouse.
//!
//! The page is `/purchase-orders`: build an order from the reorder suggestions or by hand,
//! receive the delivery through the goods receipt of the article (which names the line),
//! and close what is not coming.

use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use where_domain::{DomainError, SessionUser};
use where_persistence::{
    NewOrder, NewOrderLine, OpenOrderLineRow, OrderHeaderUpdate, PurchaseOrderLineRow,
    PurchaseOrderRow,
};

use crate::error::ApiError;
use crate::state::{AppState, State};

/// One purchase order with its counters.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PurchaseOrderDto {
    pub id: i32,
    /// Number from the series `Bestellnummer`.
    pub number: String,
    pub supplier_no: Option<i32>,
    pub supplier_name: Option<String>,
    /// `YYYY-MM-DD`.
    pub ordered_at: String,
    pub expected_at: Option<String>,
    pub note: Option<String>,
    /// `open`, `done` or `cancelled`.
    pub state: String,
    pub created_at: String,
    pub created_by: String,
    pub lines: i64,
    /// Lines that are not delivered completely.
    pub open_lines: i64,
    pub ordered: String,
    pub received: String,
    /// `ordered − received`, never negative.
    pub missing: String,
}

/// One line: what was ordered and what arrived.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PurchaseOrderLineDto {
    pub id: i32,
    pub order_id: i32,
    pub line_no: i32,
    pub article_id: i32,
    pub article_no: Option<i32>,
    pub article_name: Option<String>,
    pub quantity: String,
    /// Sum of the devices that arrived on this line.
    pub received: String,
    /// `quantity − received`, never negative.
    pub open: String,
    pub unit_price: Option<String>,
    pub note: Option<String>,
}

/// Order with all its lines.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PurchaseOrderDetailDto {
    pub order: PurchaseOrderDto,
    pub lines: Vec<PurchaseOrderLineDto>,
}

/// A line that still expects material — offered by the goods receipt.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OpenOrderLineDto {
    pub line_id: i32,
    pub order_id: i32,
    pub number: String,
    pub supplier_name: Option<String>,
    pub expected_at: Option<String>,
    pub line_no: i32,
    pub article_id: i32,
    pub quantity: String,
    pub received: String,
    pub open: String,
}

/// One line of a new order.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewOrderLineInput {
    pub article_id: i32,
    pub quantity: rust_decimal::Decimal,
    #[serde(default)]
    pub unit_price: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub note: Option<String>,
}

/// A new order with at least one line.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NewOrderInput {
    #[serde(default)]
    pub supplier_no: Option<i32>,
    /// `YYYY-MM-DD`, today when omitted.
    #[serde(default)]
    pub ordered_at: Option<String>,
    /// `YYYY-MM-DD`.
    #[serde(default)]
    pub expected_at: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub lines: Vec<NewOrderLineInput>,
}

/// Amount, price or note of a line.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OrderLineInput {
    pub quantity: rust_decimal::Decimal,
    #[serde(default)]
    pub unit_price: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Supplier, delivery date and note of an open order.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct OrderHeaderInput {
    #[serde(default)]
    pub supplier_no: Option<i32>,
    /// `YYYY-MM-DD`; `null` clears the date.
    #[serde(default)]
    pub expected_at: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Filter of the list.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct OrderParams {
    /// `open`, `done` or `cancelled`; omitted = all.
    pub state: Option<String>,
    /// Only the open lines of this article (for the goods receipt).
    pub article_id: Option<i32>,
}

fn parse_date(value: Option<&str>) -> Result<Option<chrono::NaiveDate>, ApiError> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(None),
        Some(text) => chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| DomainError::Validation(format!("'{text}' is not a date")).into()),
    }
}

impl From<PurchaseOrderRow> for PurchaseOrderDto {
    fn from(o: PurchaseOrderRow) -> Self {
        let missing = o.missing();
        Self {
            id: o.id,
            number: o.number,
            supplier_no: o.supplier_no,
            supplier_name: o.supplier_name,
            ordered_at: o.ordered_at.format("%Y-%m-%d").to_string(),
            expected_at: o.expected_at.map(|d| d.format("%Y-%m-%d").to_string()),
            note: o.note,
            state: o.state,
            created_at: o
                .created_at
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            created_by: o.created_by,
            lines: o.lines,
            open_lines: o.open_lines,
            ordered: o.ordered.to_string(),
            received: o.received.to_string(),
            missing: missing.to_string(),
        }
    }
}

impl From<PurchaseOrderLineRow> for PurchaseOrderLineDto {
    fn from(l: PurchaseOrderLineRow) -> Self {
        Self {
            id: l.id,
            order_id: l.order_id,
            line_no: l.line_no,
            article_id: l.article_id,
            article_no: l.article_no,
            article_name: l.article_name,
            quantity: l.quantity.to_string(),
            received: l.received.to_string(),
            open: l.open.to_string(),
            unit_price: l.unit_price.map(|p| p.to_string()),
            note: l.note,
        }
    }
}

impl From<OpenOrderLineRow> for OpenOrderLineDto {
    fn from(l: OpenOrderLineRow) -> Self {
        Self {
            line_id: l.line_id,
            order_id: l.order_id,
            number: l.number,
            supplier_name: l.supplier_name,
            expected_at: l.expected_at.map(|d| d.format("%Y-%m-%d").to_string()),
            line_no: l.line_no,
            article_id: l.article_id,
            quantity: l.quantity.to_string(),
            received: l.received.to_string(),
            open: l.open.to_string(),
        }
    }
}

/// Orders, newest first.
///
/// `?state=open` filters, `?article_id=` returns the open lines of one article instead —
/// that is what the goods receipt offers in its `Bestellung` field.
#[utoipa::path(
    get,
    path = "/api/v1/purchase-orders",
    tag = "articles",
    params(OrderParams),
    responses((status = 200, description = "Orders", body = Vec<PurchaseOrderDto>))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<OrderParams>,
) -> Result<Json<Vec<PurchaseOrderDto>>, ApiError> {
    let rows = state.orders.list(params.state.as_deref()).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Open lines of one article: what a delivery could still belong to.
#[utoipa::path(
    get,
    path = "/api/v1/purchase-orders/open-lines",
    tag = "articles",
    params(("article_id" = i32, Query, description = "Article")),
    responses((status = 200, description = "Open lines", body = Vec<OpenOrderLineDto>))
)]
pub async fn open_lines(
    State(state): State<AppState>,
    Query(params): Query<OrderParams>,
) -> Result<Json<Vec<OpenOrderLineDto>>, ApiError> {
    let article_id = params
        .article_id
        .ok_or_else(|| DomainError::Validation("article_id is required".to_string()))?;
    let rows = state.orders.open_lines_of_article(article_id).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// One order with its lines.
#[utoipa::path(
    get,
    path = "/api/v1/purchase-orders/{id}",
    tag = "articles",
    params(("id" = i32, Path, description = "Order")),
    responses(
        (status = 200, description = "Order and lines", body = PurchaseOrderDetailDto),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<PurchaseOrderDetailDto>, ApiError> {
    let order = state.orders.get(id).await?;
    let lines = state.orders.lines(id).await?;
    Ok(Json(PurchaseOrderDetailDto {
        order: order.into(),
        lines: lines.into_iter().map(Into::into).collect(),
    }))
}

/// Creates an order; the number comes from the series `Bestellnummer`.
#[utoipa::path(
    post,
    path = "/api/v1/purchase-orders",
    tag = "articles",
    request_body = NewOrderInput,
    responses(
        (status = 201, description = "Angelegt", body = PurchaseOrderDto),
        (status = 404, description = "Article or supplier not found"),
        (status = 409, description = "Der Nummernkreis fehlt oder ist inaktiv"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Json(body): Json<NewOrderInput>,
) -> Result<(StatusCode, Json<PurchaseOrderDto>), ApiError> {
    let order = NewOrder {
        supplier_no: body.supplier_no,
        ordered_at: parse_date(body.ordered_at.as_deref())?,
        expected_at: parse_date(body.expected_at.as_deref())?,
        note: body.note,
        lines: body
            .lines
            .into_iter()
            .map(|l| NewOrderLine {
                article_id: l.article_id,
                quantity: l.quantity,
                unit_price: l.unit_price,
                note: l.note,
            })
            .collect(),
    };
    let row = state.orders.create(&order, &user.user_name).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Changes supplier, delivery date or note of an open order.
#[utoipa::path(
    put,
    path = "/api/v1/purchase-orders/{id}",
    tag = "articles",
    params(("id" = i32, Path, description = "Order")),
    request_body = OrderHeaderInput,
    responses(
        (status = 200, description = "Geändert", body = PurchaseOrderDto),
        (status = 404, description = "Order or supplier not found"),
        (status = 409, description = "Diese Bestellung ist abgeschlossen"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn update_header(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<OrderHeaderInput>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let update = OrderHeaderUpdate {
        supplier_no: body.supplier_no,
        expected_at: parse_date(body.expected_at.as_deref())?,
        note: body.note,
    };
    let row = state.orders.update_header(id, &update).await?;
    Ok(Json(row.into()))
}

/// Adds a line to an open order.
#[utoipa::path(
    post,
    path = "/api/v1/purchase-orders/{id}/lines",
    tag = "articles",
    params(("id" = i32, Path, description = "Order")),
    request_body = OrderLineInput,
    responses(
        (status = 200, description = "Position angelegt", body = PurchaseOrderDto),
        (status = 404, description = "Order or article not found"),
        (status = 409, description = "Diese Bestellung ist abgeschlossen"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn add_line(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<AddLineInput>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let row = state
        .orders
        .add_line(
            id,
            body.article_id,
            body.quantity,
            body.unit_price,
            body.note.as_deref(),
        )
        .await?;
    Ok(Json(row.into()))
}

/// A line of an order: the article is only sent when the line is created.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct AddLineInput {
    pub article_id: i32,
    pub quantity: rust_decimal::Decimal,
    #[serde(default)]
    pub unit_price: Option<rust_decimal::Decimal>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Changes amount, price or note of a line.
#[utoipa::path(
    put,
    path = "/api/v1/purchase-orders/{id}/lines/{line_id}",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Order"),
        ("line_id" = i32, Path, description = "Line")
    ),
    request_body = OrderLineInput,
    responses(
        (status = 200, description = "Geändert", body = PurchaseOrderDto),
        (status = 404, description = "Not found"),
        (status = 409, description = "Diese Bestellung ist abgeschlossen"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn update_line(
    State(state): State<AppState>,
    Path((id, line_id)): Path<(i32, i32)>,
    Json(body): Json<OrderLineInput>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let row = state
        .orders
        .update_line(
            id,
            line_id,
            body.quantity,
            body.unit_price,
            body.note.as_deref(),
        )
        .await?;
    Ok(Json(row.into()))
}

/// Removes a line that has not delivered anything yet.
#[utoipa::path(
    delete,
    path = "/api/v1/purchase-orders/{id}/lines/{line_id}",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Order"),
        ("line_id" = i32, Path, description = "Line")
    ),
    responses(
        (status = 200, description = "Position entfernt", body = PurchaseOrderDto),
        (status = 404, description = "Not found"),
        (status = 409, description = "Auf diese Position ist schon Ware eingegangen")
    )
)]
pub async fn remove_line(
    State(state): State<AppState>,
    Path((id, line_id)): Path<(i32, i32)>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let row = state.orders.remove_line(id, line_id).await?;
    Ok(Json(row.into()))
}

/// Closes the order: the rest is not coming.
#[utoipa::path(
    post,
    path = "/api/v1/purchase-orders/{id}/close",
    tag = "articles",
    params(("id" = i32, Path, description = "Order")),
    responses(
        (status = 200, description = "Abgeschlossen", body = PurchaseOrderDto),
        (status = 404, description = "Not found"),
        (status = 409, description = "Bereits abgeschlossen")
    )
)]
pub async fn close(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let row = state.orders.close(id).await?;
    Ok(Json(row.into()))
}

/// Drops the order — only while nothing has arrived.
#[utoipa::path(
    post,
    path = "/api/v1/purchase-orders/{id}/cancel",
    tag = "articles",
    params(("id" = i32, Path, description = "Order")),
    responses(
        (status = 200, description = "Storniert", body = PurchaseOrderDto),
        (status = 404, description = "Not found"),
        (status = 409, description = "Auf diese Bestellung ist schon Ware eingegangen")
    )
)]
pub async fn cancel(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<PurchaseOrderDto>, ApiError> {
    let row = state.orders.cancel(id).await?;
    Ok(Json(row.into()))
}
