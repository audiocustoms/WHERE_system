use crate::state::State;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use where_domain::{Article, DomainError};
use where_modules::ModuleId;
use where_persistence::rows::SavedSearch;
use where_persistence::{DeviceRow, StockRow};

use crate::dto::{
    ArticleListParams, ArticlePage, ArticleSearchInput, DeviceDto, DeviceInput, GoodsReceiptInput,
    GoodsReceiptResponse, ListParams, SavedSearchDto, SearchSaveInput, StockLevelDto, StockRowDto,
};
use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::AppState;

/// `true` when the article carries rental/sale data or prices.
///
/// `WHERE - is my stuff?` alone has no rental or sale category and no pricing
/// block; both belong to `WHERE - does my stuff go?`.
fn uses_pricing(article: &Article) -> bool {
    article.rental || article.sale || !article.pricing.is_empty()
}

fn date(value: Option<chrono::NaiveDate>) -> Option<String> {
    value.map(|d| d.format("%Y-%m-%d").to_string())
}

fn parse_date(value: &Option<String>) -> Result<Option<chrono::NaiveDate>, ApiError> {
    match value.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(None),
        Some(text) => chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| {
                ApiError(where_domain::DomainError::Validation(format!(
                    "Date '{text}' is not a valid date (YYYY-MM-DD)"
                )))
            }),
    }
}

impl From<StockRow> for StockRowDto {
    fn from(z: StockRow) -> Self {
        let available = z.available().map(|d| d.to_string());
        Self {
            stock_id: z.stock_id,
            stock_name: z.stock_name,
            is_default: z.is_default,
            total: z.total.map(|d| d.to_string()),
            rental: z.rental.map(|d| d.to_string()),
            sale: z.sale.map(|d| d.to_string()),
            inventory: z.inventory.map(|d| d.to_string()),
            used_sale: z.used_sale.map(|d| d.to_string()),
            reserved: z.reserved.map(|d| d.to_string()),
            withdrawal: z.withdrawal.map(|d| d.to_string()),
            available,
        }
    }
}

impl From<DeviceRow> for DeviceDto {
    fn from(g: DeviceRow) -> Self {
        let status = g.status().to_string();
        let usage_text = g.usage_text();
        Self {
            id: g.id,
            serial_no: g.serial_no,
            inventory_no: g.inventory_no,
            inventory_no2: g.inventory_no2,
            quantity: g.quantity.map(|d| d.to_string()),
            inventory: g.inventory,
            rental: g.rental,
            sale: g.sale,
            used_sale: g.used_sale,
            usage_text,
            stock_id: g.stock_id,
            stock_name: g.stock_name,
            bin_location: g.bin_location,
            condition: g.condition,
            discarded: g.discarded,
            status,
            purchase_price: g.purchase_price.map(|d| d.to_string()),
            purchase_date: date(g.purchase_date),
            supplier_no: g.supplier_no,
            owner: g.owner,
            warranty_months: g.warranty_months,
            ext_barcode: g.ext_barcode,
            year_built: g.year_built,
            note: g.note,
            without_serial: g.without_serial,
            recorded_at: date(g.recorded_at),
        }
    }
}

/// Article list with search over name, short name, match code, manufacturer, inventory no. and EAN,
/// optionally narrowed to one article group of the left menu (including its subgroups).
#[utoipa::path(
    get,
    path = "/api/v1/articles",
    tag = "articles",
    params(ArticleListParams),
    responses(
        (status = 200, description = "One page of article rows", body = ArticlePage),
        (status = 422, description = "Unknown active filter"),
        (status = 500, description = "Interner Fehler")
    )
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<ArticleListParams>,
) -> Result<Json<ArticlePage>, ApiError> {
    let (limit, offset) = params.window();
    let page = state
        .article
        .list(
            limit,
            offset,
            params.search(),
            params.group(),
            params.active()?,
        )
        .await?;
    Ok(Json(ArticlePage {
        items: page.items.into_iter().map(Into::into).collect(),
        total: page.total,
        limit: page.limit,
        offset: page.offset,
    }))
}

/// Extended article search — the form from `img/article-search.PNG`.
///
/// The criteria that are set are combined with AND; empty fields do not restrict.
/// `with_bundle` extends the hits by articles that share a bundle with a hit
/// (as material or as the main article).
#[utoipa::path(
    post,
    path = "/api/v1/articles/search",
    tag = "articles",
    request_body = ArticleSearchInput,
    responses(
        (status = 200, description = "Search hits", body = ArticlePage),
        (status = 422, description = "Unknown article type, operator or inverted range")
    )
)]
pub async fn search(
    State(state): State<AppState>,
    Json(body): Json<ArticleSearchInput>,
) -> Result<Json<ArticlePage>, ApiError> {
    let (limit, offset) = body.window();
    let criteria = body.criteria()?;
    let (items, total) = state
        .article_search
        .search(&criteria, limit, offset)
        .await?;
    Ok(Json(ArticlePage {
        items: items.into_iter().map(Into::into).collect(),
        total,
        limit,
        offset,
    }))
}

fn saved_dto(s: SavedSearch) -> SavedSearchDto {
    let criteria = serde_json::from_str(&s.criteria).unwrap_or(serde_json::Value::Null);
    SavedSearchDto {
        id: s.id,
        name: s.name,
        criteria,
        created_at: s.created_at.to_rfc3339(),
        changed_at: s.changed_at.to_rfc3339(),
    }
}

/// The saved searches of the form (the "process" selector).
#[utoipa::path(
    get,
    path = "/api/v1/articles/searches",
    tag = "articles",
    responses((status = 200, description = "Saved searches", body = Vec<SavedSearchDto>))
)]
pub async fn saved_searches(
    State(state): State<AppState>,
) -> Result<Json<Vec<SavedSearchDto>>, ApiError> {
    let rows = state.article_search.saved().await?;
    Ok(Json(rows.into_iter().map(saved_dto).collect()))
}

/// Saves a search under a name; an existing name is overwritten.
#[utoipa::path(
    post,
    path = "/api/v1/articles/searches",
    tag = "articles",
    request_body = SearchSaveInput,
    responses(
        (status = 201, description = "Gespeichert", body = SavedSearchDto),
        (status = 422, description = "Name missing or criteria invalid")
    )
)]
pub async fn search_save(
    State(state): State<AppState>,
    Json(body): Json<SearchSaveInput>,
) -> Result<(StatusCode, Json<SavedSearchDto>), ApiError> {
    let name = body.name.trim();
    if name.is_empty() {
        return Err(ApiError(where_domain::DomainError::Validation(
            "The name of the search must not be empty".into(),
        )));
    }
    body.criteria.criteria()?;
    let criteria = serde_json::to_string(&body.criteria).map_err(|e| {
        ApiError(where_domain::DomainError::Internal(format!(
            "Criteria cannot be serialized: {e}"
        )))
    })?;
    let saved = state.article_search.save(name, &criteria).await?;
    Ok((StatusCode::CREATED, Json(saved_dto(saved))))
}

/// Deletes a saved search.
#[utoipa::path(
    delete,
    path = "/api/v1/articles/searches/{id}",
    tag = "articles",
    params(("id" = i32, Path, description = "Saved search id")),
    responses(
        (status = 200, description = "Deleted", body = DeletedResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn search_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.article_search.delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Complete article including text blocks and article groups.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}",
    tag = "articles",
    params(("id" = i32, Path, description = "Article primary key")),
    responses(
        (status = 200, description = "The article", body = Article),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Article>, ApiError> {
    Ok(Json(state.article.get(id).await?))
}

/// Creates an article. `article_no = 0` assigns the next free number.
#[utoipa::path(
    post,
    path = "/api/v1/articles",
    tag = "articles",
    request_body = Article,
    responses(
        (status = 201, description = "Angelegt", body = Article),
        (status = 409, description = "Artikelnummer bereits vergeben"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<Article>,
) -> Result<(StatusCode, Json<Article>), ApiError> {
    // Without `WHERE - does my stuff go?` there are neither rental/sale articles nor prices.
    if uses_pricing(&body) && !state.modules.is_installed(ModuleId::Go) {
        return Err(DomainError::Conflict(
            "Miete, Verkauf und Preise stehen erst mit dem Modul „WHERE - does my stuff go?“ bereit."
                .to_string(),
        )
        .into());
    }
    let created = state.article.create(&body).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

/// Replaces an article completely (PUT semantics).
#[utoipa::path(
    put,
    path = "/api/v1/articles/{id}",
    tag = "articles",
    params(("id" = i32, Path, description = "Article primary key")),
    request_body = Article,
    responses(
        (status = 200, description = "Ersetzt", body = Article),
        (status = 404, description = "Not found"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn replace(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(mut body): Json<Article>,
) -> Result<Json<Article>, ApiError> {
    // Without the order module the existing rental/sale data is kept, even when an
    // older mask does not send it.
    if !state.modules.is_installed(ModuleId::Go) {
        let existing = state.article.get(id).await?;
        body.rental = existing.rental;
        body.sale = existing.sale;
        body.pricing = existing.pricing;
    }
    Ok(Json(state.article.update(id, &body).await?))
}

/// Deletes an article.
#[utoipa::path(
    delete,
    path = "/api/v1/articles/{id}",
    tag = "articles",
    params(("id" = i32, Path, description = "Article primary key")),
    responses(
        (status = 200, description = "Deleted", body = DeletedResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.article.delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Form "new": returns the next free article number so the form shows
/// the same number that will be assigned on save.
#[utoipa::path(
    get,
    path = "/api/v1/articles/new",
    tag = "articles",
    responses((status = 200, description = "Template for a new article", body = Article))
)]
pub async fn create_template(State(state): State<AppState>) -> Result<Json<Article>, ApiError> {
    let mut template = Article {
        active: true,
        stock_tracking: true,
        ..Default::default()
    };
    template.article_no = state.article.next_number().await?;
    Ok(Json(template))
}

/// Stock of an article: totals per warehouse and the devices/batches.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/stock",
    tag = "articles",
    params(("id" = i32, Path, description = "Article primary key")),
    responses(
        (status = 200, description = "Warehouses and devices/batches", body = StockLevelDto),
        (status = 404, description = "Article not found")
    )
)]
pub async fn stock(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<ListParams>,
) -> Result<Json<StockLevelDto>, ApiError> {
    let stock = state.stock.stock(id).await?;
    let devices = state.stock.devices(id, params.search()).await?;
    Ok(Json(StockLevelDto {
        stock: stock.into_iter().map(Into::into).collect(),
        devices: devices.into_iter().map(Into::into).collect(),
    }))
}

/// Goods receipt: books in the devices of one delivery.
///
/// With serial numbers `quantity` is the number of devices — each gets its own row and
/// its serial number continues the one entered. Without serial numbers (a batch) one
/// row carries the quantity.
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/devices",
    tag = "articles",
    params(("id" = i32, Path, description = "Article primary key")),
    request_body = GoodsReceiptInput,
    responses(
        (status = 201, description = "Erfasst", body = GoodsReceiptResponse),
        (status = 404, description = "Article or warehouse not found"),
        (status = 409, description = "Seriennummer bereits erfasst"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn goods_receipt(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<GoodsReceiptInput>,
) -> Result<(StatusCode, Json<GoodsReceiptResponse>), ApiError> {
    let eingang = where_persistence::GoodsReceipt {
        serial_no: body.serial_no,
        without_serial: body.without_serial,
        stock_id: body.stock_id,
        bin_location: body.bin_location,
        condition: body.condition,
        purchase_price: body.purchase_price,
        purchase_date: parse_date(&body.purchase_date)?,
        supplier_no: body.supplier_no,
        inventory_no: body.inventory_no,
        reason: body.reason,
        inventory: body.inventory,
        rental: body.rental,
        sale: body.sale,
        used_sale: body.used_sale,
        quantity: body.quantity,
    };
    let geraete = state.stock.goods_receipt(id, &eingang).await?;
    let menge: rust_decimal::Decimal = geraete
        .iter()
        .map(|g| g.quantity.unwrap_or(rust_decimal::Decimal::ONE))
        .sum();
    Ok((
        StatusCode::CREATED,
        Json(GoodsReceiptResponse {
            devices: geraete.into_iter().map(Into::into).collect(),
            quantity: menge.to_string(),
        }),
    ))
}

/// A single device.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/devices/{device_id}",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device id")
    ),
    responses(
        (status = 200, description = "Device", body = DeviceDto),
        (status = 404, description = "Not found")
    )
)]
pub async fn device(
    State(state): State<AppState>,
    Path((id, device_id)): Path<(i32, i32)>,
) -> Result<Json<DeviceDto>, ApiError> {
    let device = state.stock.device(id, device_id).await?;
    Ok(Json(device.into()))
}

/// Changes device data (right-hand side of the form). Stock totals are recomputed.
#[utoipa::path(
    put,
    path = "/api/v1/articles/{id}/devices/{device_id}",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device id")
    ),
    request_body = DeviceInput,
    responses(
        (status = 200, description = "Gespeichert", body = DeviceDto),
        (status = 404, description = "Device or warehouse not found"),
        (status = 409, description = "Seriennummer bereits erfasst"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn device_update(
    State(state): State<AppState>,
    Path((id, device_id)): Path<(i32, i32)>,
    Json(body): Json<DeviceInput>,
) -> Result<Json<DeviceDto>, ApiError> {
    let update = where_persistence::DeviceUpdate {
        serial_no: body.serial_no,
        inventory_no: body.inventory_no,
        inventory_no2: body.inventory_no2,
        stock_id: body.stock_id,
        bin_location: body.bin_location,
        condition: body.condition,
        inventory: body.inventory,
        rental: body.rental,
        sale: body.sale,
        used_sale: body.used_sale,
        discarded: body.discarded,
        purchase_price: body.purchase_price,
        purchase_date: parse_date(&body.purchase_date)?,
        supplier_no: body.supplier_no,
        owner: body.owner,
        warranty_months: body.warranty_months,
        ext_barcode: body.ext_barcode,
        year_built: body.year_built,
        note: body.note,
    };
    let device = state.stock.update_device(id, device_id, &update).await?;
    Ok(Json(device.into()))
}

/// Removes a device for good (form button "−"). To retire it instead,
/// set `ausgesondert`.
#[utoipa::path(
    delete,
    path = "/api/v1/articles/{id}/devices/{device_id}",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device id")
    ),
    responses(
        (status = 200, description = "Entfernt", body = DeletedResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn device_delete(
    State(state): State<AppState>,
    Path((id, device_id)): Path<(i32, i32)>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.stock.delete_device(id, device_id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn article() -> Article {
        Article {
            name: "Scheinwerfer".into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_plain_article_needs_no_pricing_module() {
        assert!(!uses_pricing(&article()));
    }

    #[test]
    fn rental_or_sale_flags_need_the_module() {
        let mut a = article();
        a.rental = true;
        assert!(uses_pricing(&a));
        a.rental = false;
        a.sale = true;
        assert!(uses_pricing(&a));
    }

    #[test]
    fn a_single_price_needs_the_module() {
        let mut a = article();
        a.pricing.rental_price_per_day = Some(Decimal::from(12));
        assert!(uses_pricing(&a));
    }
}
