use crate::state::State;
use axum::Json;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use where_domain::{Article, DomainError, SessionUser};
use where_modules::ModuleId;
use where_persistence::rows::SavedSearch;
use where_persistence::{DeviceRow, MovementRow, StockRow};

use crate::dto::{
    ArticleListParams, ArticlePage, ArticleSearchInput, BulkTransferDto, BulkTransferFailureDto,
    BulkTransferInput, ConsumeInput, DeviceDto, DeviceInput, GoodsReceiptInput,
    GoodsReceiptResponse, IssueInput, ListParams, MovementDto, MovementParams, ReorderLevelInput,
    ReturnInput, SavedSearchDto, SearchSaveInput, StockLevelDto, StockRowDto, TransferInput,
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

/// Guards a booking against the warehouse limit of the account (`"User_Lager"`).
///
/// An account without entries may work in every warehouse; one with entries only in
/// the warehouses it was given. `stock_id` is the warehouse the booking touches —
/// the target of a receipt or a transfer, the current one of a withdrawal.
async fn require_warehouse(
    state: &AppState,
    user: &SessionUser,
    stock_id: Option<i32>,
) -> Result<(), ApiError> {
    let Some(stock_id) = stock_id else {
        return Ok(());
    };
    let allowed = state.users.warehouses(user.id).await?;
    if allowed.is_empty() || allowed.contains(&stock_id) {
        return Ok(());
    }
    Err(
        DomainError::Forbidden("Dieses Lager ist für dieses Konto nicht freigegeben.".into())
            .into(),
    )
}

impl From<StockRow> for StockRowDto {
    fn from(z: StockRow) -> Self {
        let available = z.available().map(|d| d.to_string());
        Self {
            stock_id: z.stock_id,
            stock_name: z.stock_name,
            is_default: z.is_default,
            zugriff: z.zugriff,
            entnahme: z.entnahme,
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
            place_id: g.place_id,
            place_name: g.place_name,
            issued_from: date(g.issued_from),
            issued_until: date(g.issued_until),
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

impl From<MovementRow> for MovementDto {
    fn from(m: MovementRow) -> Self {
        Self {
            id: m.id,
            device_id: m.device_id,
            serial_no: m.serial_no,
            kind: m.kind,
            quantity: m.quantity.map(|d| d.to_string()),
            warehouse_from: m.warehouse_from,
            warehouse_from_name: m.warehouse_from_name,
            warehouse_to: m.warehouse_to,
            warehouse_to_name: m.warehouse_to_name,
            bin_from: m.bin_from,
            bin_to: m.bin_to,
            place_from_name: m.place_from_name,
            place_to_name: m.place_to_name,
            condition_from: m.condition_from,
            condition_to: m.condition_to,
            discarded_from: m.discarded_from,
            discarded_to: m.discarded_to,
            note: m.note,
            at: m
                .created_at
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string(),
            by: m.created_by,
        }
    }
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
    let reorder_level = state.stock.reorder_level(id).await?;
    Ok(Json(StockLevelDto {
        stock: stock.into_iter().map(Into::into).collect(),
        devices: devices.into_iter().map(Into::into).collect(),
        reorder_level: reorder_level.map(|d| d.to_string()),
    }))
}

/// Sets the `Meldebestand` of an article — the level the reorder suggestion compares with.
///
/// It rides on `"VerkaufMindBestand"`, so it also feeds the pricing tab when
/// `WHERE - does my stuff go?` is installed.
#[utoipa::path(
    put,
    path = "/api/v1/articles/{id}/reorder-level",
    tag = "articles",
    params(("id" = i32, Path, description = "Article primary key")),
    request_body = ReorderLevelInput,
    responses(
        (status = 204, description = "Gespeichert"),
        (status = 404, description = "Article not found"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn set_reorder_level(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<ReorderLevelInput>,
) -> Result<StatusCode, ApiError> {
    let level = body.level.filter(|d| *d > rust_decimal::Decimal::ZERO);
    state.stock.set_reorder_level(id, level).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Stock history of an article — every booking of its devices, newest first.
/// Written by the goods receipt, the device data, the move to another warehouse, the
/// discard and the delete, each inside the transaction of the booking itself.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/movements",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device" = Option<i32>, Query, description = "Only the lines of this device"),
        ("limit" = Option<i64>, Query, description = "Newest first, default 100, at most 1000")
    ),
    responses(
        (status = 200, description = "Movements", body = Vec<MovementDto>),
        (status = 404, description = "Article not found")
    )
)]
pub async fn movements(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<MovementParams>,
) -> Result<Json<Vec<MovementDto>>, ApiError> {
    let limit = params.limit.unwrap_or(100).clamp(1, 1000);
    let rows = state.stock.movements(id, params.device, limit).await?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
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
    Extension(user): Extension<SessionUser>,
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
        order_line: body.order_line,
        inventory: body.inventory,
        rental: body.rental,
        sale: body.sale,
        used_sale: body.used_sale,
        quantity: body.quantity,
    };
    require_warehouse(&state, &user, Some(eingang.stock_id)).await?;
    let geraete = state
        .stock
        .goods_receipt(id, &eingang, &user.user_name)
        .await?;
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
    Extension(user): Extension<SessionUser>,
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
    let geraet = state.stock.device(id, device_id).await?;
    require_warehouse(&state, &user, geraet.stock_id).await?;
    require_warehouse(&state, &user, update.stock_id).await?;
    let device = state
        .stock
        .update_device(id, device_id, &update, &user.user_name)
        .await?;
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
    Extension(user): Extension<SessionUser>,
    Path((id, device_id)): Path<(i32, i32)>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let geraet = state.stock.device(id, device_id).await?;
    require_warehouse(&state, &user, geraet.stock_id).await?;
    state
        .stock
        .delete_device(id, device_id, &user.user_name)
        .await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Hands material out to a place of the house — `Foyer` for a week, `Podium` for a
/// show. A device with a serial number keeps its row; a batch is split, so the part that
/// stays behind keeps its own line. What goes out counts as withdrawal: still our stock,
/// no longer on the shelf.
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/devices/{device_id}/issue",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device id")
    ),
    request_body = IssueInput,
    responses(
        (status = 204, description = "Ausgegeben"),
        (status = 404, description = "Device, place or warehouse not found"),
        (status = 409, description = "Das Material steht bereits an einem Ort"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn device_issue(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path((id, device_id)): Path<(i32, i32)>,
    Json(body): Json<IssueInput>,
) -> Result<StatusCode, ApiError> {
    let ausgabe = where_persistence::Issue {
        place_id: body.place_id,
        quantity: body.quantity,
        until: parse_date(&body.until)?,
        note: body.note,
    };
    let geraet = state.stock.device(id, device_id).await?;
    require_warehouse(&state, &user, geraet.stock_id).await?;
    state
        .stock
        .issue(id, device_id, &ausgabe, &user.user_name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Takes issued material back into the warehouse.
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/devices/{device_id}/return",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device id")
    ),
    request_body = ReturnInput,
    responses(
        (status = 204, description = "Zurückgenommen"),
        (status = 404, description = "Not found"),
        (status = 422, description = "Das Material steht nicht an einem Ort")
    )
)]
pub async fn device_return(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path((id, device_id)): Path<(i32, i32)>,
    Json(body): Json<ReturnInput>,
) -> Result<StatusCode, ApiError> {
    let geraet = state.stock.device(id, device_id).await?;
    require_warehouse(&state, &user, geraet.stock_id).await?;
    state
        .stock
        .give_back(
            id,
            device_id,
            body.condition.as_deref(),
            body.note.as_deref(),
            &user.user_name,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Moves material to another warehouse of the house.
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/devices/{device_id}/transfer",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device id")
    ),
    request_body = TransferInput,
    responses(
        (status = 204, description = "Umgelegt"),
        (status = 404, description = "Device or warehouse not found"),
        (status = 409, description = "Ausgegebenes Material zuerst zurücknehmen"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn device_transfer(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path((id, device_id)): Path<(i32, i32)>,
    Json(body): Json<TransferInput>,
) -> Result<StatusCode, ApiError> {
    let geraet = state.stock.device(id, device_id).await?;
    require_warehouse(&state, &user, geraet.stock_id).await?;
    require_warehouse(&state, &user, Some(body.stock_id)).await?;
    state
        .stock
        .transfer(
            id,
            device_id,
            body.stock_id,
            body.quantity,
            body.note.as_deref(),
            &user.user_name,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Moves several rows of one article into another warehouse (`Sammel-Umlagerung`).
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/stock-transfer",
    tag = "articles",
    params(("id" = i32, Path, description = "Article primary key")),
    request_body = BulkTransferInput,
    responses(
        (status = 200, description = "Umgelegt", body = BulkTransferDto),
        (status = 404, description = "Article or warehouse not found"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn device_transfer_many(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path(id): Path<i32>,
    Json(body): Json<BulkTransferInput>,
) -> Result<Json<BulkTransferDto>, ApiError> {
    require_warehouse(&state, &user, Some(body.stock_id)).await?;
    let (moved, failed) = state
        .stock
        .transfer_many(
            id,
            &body.devices,
            body.stock_id,
            body.note.as_deref(),
            &user.user_name,
        )
        .await?;
    Ok(Json(BulkTransferDto {
        moved,
        failed: failed
            .into_iter()
            .map(|(device_id, reason)| BulkTransferFailureDto { device_id, reason })
            .collect(),
    }))
}

/// Consumes material out of a batch (`Verbrauchen`) — used up, it goes nowhere and comes
/// back never. A device with a serial number is retired instead.
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/devices/{device_id}/consume",
    tag = "articles",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device id")
    ),
    request_body = ConsumeInput,
    responses(
        (status = 204, description = "Verbraucht"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Ausgegebenes Material zuerst zurücknehmen"),
        (status = 422, description = "Validierung fehlgeschlagen")
    )
)]
pub async fn device_consume(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Path((id, device_id)): Path<(i32, i32)>,
    Json(body): Json<ConsumeInput>,
) -> Result<StatusCode, ApiError> {
    let geraet = state.stock.device(id, device_id).await?;
    require_warehouse(&state, &user, geraet.stock_id).await?;
    state
        .stock
        .consume(
            id,
            device_id,
            body.quantity,
            body.note.as_deref(),
            &user.user_name,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
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
