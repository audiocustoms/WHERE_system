//! Reorder suggestions (`Beschaffung`): what stands below its reorder level.

use axum::Json;
use axum::extract::Query;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use where_persistence::SuggestionRow;

use crate::error::ApiError;
use crate::state::{AppState, State};

/// One line of the reorder list.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SuggestionDto {
    pub article_id: i32,
    pub article_no: Option<i32>,
    pub article_name: Option<String>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    /// `Gesamt − Festblockade − Entnahme` of that warehouse.
    pub available: String,
    /// `"VerkaufMindBestand"` — the field `Meldebestand` of the article form.
    pub reorder_level: String,
    /// What has to be ordered: `reorder_level − available`.
    pub missing: String,
    /// Consumed within the window (all warehouses of the article).
    pub used: String,
    pub lead_time_days: Option<i32>,
    pub supplier_no: Option<i32>,
    pub supplier_name: Option<String>,
    pub supplier_article_no: Option<String>,
}

/// Window of the consumption figures.
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct SuggestionParams {
    /// Days back for the consumption column; default 90, at most 3650.
    pub days: Option<i32>,
}

impl From<SuggestionRow> for SuggestionDto {
    fn from(r: SuggestionRow) -> Self {
        Self {
            article_id: r.article_id,
            article_no: r.article_no,
            article_name: r.article_name,
            warehouse_id: r.warehouse_id,
            warehouse_name: r.warehouse_name,
            available: r.available.to_string(),
            reorder_level: r.reorder_level.to_string(),
            missing: r.missing.to_string(),
            used: r.used.to_string(),
            lead_time_days: r.lead_time_days,
            supplier_no: r.supplier_no,
            supplier_name: r.supplier_name,
            supplier_article_no: r.supplier_article_no,
        }
    }
}

/// Articles below their reorder level, largest gap first.
///
/// Everything is derived: the level from `"Artikel"."VerkaufMindBestand"`, the amount from
/// `"Artikel_Env"`, the consumption from the stock history and the lead time from the
/// article's suppliers. No order is placed here.
#[utoipa::path(
    get,
    path = "/api/v1/procurement/suggestions",
    tag = "articles",
    params(SuggestionParams),
    responses((status = 200, description = "Reorder suggestions", body = Vec<SuggestionDto>))
)]
pub async fn suggestions(
    State(state): State<AppState>,
    Query(params): Query<SuggestionParams>,
) -> Result<Json<Vec<SuggestionDto>>, ApiError> {
    let rows = state
        .procurement
        .suggestions(params.days.unwrap_or(90))
        .await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}
