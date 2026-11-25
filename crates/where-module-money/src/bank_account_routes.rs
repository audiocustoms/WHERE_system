//! Bank accounts of the company (`img/Admin Menü/Bankverbindungen`).
//!
//! The window has three tabs over one row of `"Bankverbindungen"`: `Allgemein`, `SEPA` and
//! `Schweiz`. The `Kto & BLZ in IBAN & BIC umwandeln` button has an endpoint of its own —
//! it builds the German IBAN and reads BIC and bank name from the bank list.

use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use serde::Deserialize;
use utoipa::ToSchema;
use where_api::ApiError;
use where_api::routes::addresses::DeletedResponse;
use where_api::state::{AppState, State};

use crate::bank_account::{BankAccount, BankAccountInput, ConversionRequest, ConversionResult};
use crate::bank_account_repo::BankAccountRepository;

fn bank_accounts(state: &AppState) -> BankAccountRepository {
    BankAccountRepository::new(state.pool.clone())
}

/// All accounts of the company.
#[utoipa::path(
    get,
    path = "/api/v1/bank-accounts",
    tag = "bankverbindungen",
    responses((status = 200, body = Vec<BankAccount>))
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<BankAccount>>, ApiError> {
    Ok(Json(bank_accounts(&state).list().await?))
}

/// One account with all columns of the three tabs.
#[utoipa::path(
    get,
    path = "/api/v1/bank-accounts/{id}",
    tag = "bankverbindungen",
    params(("id" = i64, Path, description = "Key of the account")),
    responses(
        (status = 200, body = BankAccount),
        (status = 404, description = "Unknown account", body = where_api::error::Problem)
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<BankAccount>, ApiError> {
    Ok(Json(bank_accounts(&state).get(id).await?))
}

/// Adds an account.
#[utoipa::path(
    post,
    path = "/api/v1/bank-accounts",
    tag = "bankverbindungen",
    request_body = BankAccountInput,
    responses(
        (status = 201, body = BankAccount),
        (status = 422, description = "Invalid label, IBAN or BIC", body = where_api::error::Problem)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<BankAccountInput>,
) -> Result<(StatusCode, Json<BankAccount>), ApiError> {
    let account = bank_accounts(&state).create(input).await?;
    Ok((StatusCode::CREATED, Json(account)))
}

/// Changes an account.
#[utoipa::path(
    put,
    path = "/api/v1/bank-accounts/{id}",
    tag = "bankverbindungen",
    params(("id" = i64, Path, description = "Key of the account")),
    request_body = BankAccountInput,
    responses(
        (status = 200, body = BankAccount),
        (status = 404, description = "Unknown account", body = where_api::error::Problem),
        (status = 422, description = "Invalid label, IBAN or BIC", body = where_api::error::Problem)
    )
)]
pub async fn replace(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<BankAccountInput>,
) -> Result<Json<BankAccount>, ApiError> {
    Ok(Json(bank_accounts(&state).update(id, input).await?))
}

/// Removes an account.
#[utoipa::path(
    delete,
    path = "/api/v1/bank-accounts/{id}",
    tag = "bankverbindungen",
    params(("id" = i64, Path, description = "Key of the account")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown account", body = where_api::error::Problem)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    bank_accounts(&state).delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// What the `Std` column of the list sends.
#[derive(Debug, Deserialize, ToSchema)]
pub struct StandardInput {
    pub standard: bool,
}

/// Sets or clears the `Standard` flag — the `Std` column of the list.
#[utoipa::path(
    put,
    path = "/api/v1/bank-accounts/{id}/standard",
    tag = "bankverbindungen",
    params(("id" = i64, Path, description = "Key of the account")),
    request_body = StandardInput,
    responses(
        (status = 200, body = BankAccount),
        (status = 404, description = "Unknown account", body = where_api::error::Problem)
    )
)]
pub async fn set_standard(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<StandardInput>,
) -> Result<Json<BankAccount>, ApiError> {
    Ok(Json(
        bank_accounts(&state)
            .set_standard(id, input.standard)
            .await?,
    ))
}

/// „Kto & BLZ in IBAN & BIC umwandeln“: sort code and account number in, IBAN, BIC and
/// bank data out.
#[utoipa::path(
    post,
    path = "/api/v1/bank-accounts/convert",
    tag = "bankverbindungen",
    request_body = ConversionRequest,
    responses(
        (status = 200, body = ConversionResult),
        (status = 422, description = "Invalid sort code or account number", body = where_api::error::Problem)
    )
)]
pub async fn convert(
    State(state): State<AppState>,
    Json(request): Json<ConversionRequest>,
) -> Result<Json<ConversionResult>, ApiError> {
    Ok(Json(bank_accounts(&state).convert(request).await?))
}
