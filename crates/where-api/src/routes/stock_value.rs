//! Stock value (`Bestandswert`): what the shelf is worth, per warehouse and article group.
//!
//! The page is `/stock-value`. Nothing is stored — the figure is `"Menge" × "Kaufpreis"`
//! over the stock rows, so it follows every booking and every correction of a price.

use axum::Json;
use axum::extract::Query;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use where_persistence::StockValueRow;

use crate::error::ApiError;
use crate::state::{AppState, State};

/// One line of the report.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct StockValueDto {
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    /// Empty when the article belongs to no group.
    pub group_name: String,
    pub articles: i64,
    pub amount: String,
    /// `"Menge" × "Kaufpreis"`, in the currency of the purchase prices.
    pub value: String,
}

/// Filter of the report.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct StockValueParams {
    /// Only this warehouse; omitted = all.
    pub warehouse: Option<i32>,
}

impl From<StockValueRow> for StockValueDto {
    fn from(r: StockValueRow) -> Self {
        Self {
            warehouse_id: r.warehouse_id,
            warehouse_name: r.warehouse_name,
            group_name: r.group_name,
            articles: r.articles,
            amount: r.amount.to_string(),
            value: r.value.to_string(),
        }
    }
}

/// Value of the stock per warehouse and article group, largest first.
#[utoipa::path(
    get,
    path = "/api/v1/stock-value",
    tag = "articles",
    params(StockValueParams),
    responses((status = 200, description = "Stock value", body = Vec<StockValueDto>))
)]
pub async fn report(
    State(state): State<AppState>,
    Query(params): Query<StockValueParams>,
) -> Result<Json<Vec<StockValueDto>>, ApiError> {
    let rows = state.stock_value.report(params.warehouse).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}
