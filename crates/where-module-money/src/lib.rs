//! `WHERE - is my money?` — accounting, bank master data and bank accounts.
//!
//! The module carries the bank list (SWIFT/BIC), the company's bank accounts and
//! — in later steps — invoicing. It plugs into the core through
//! [`where_api::ModulePlugin`] and shares the core's database and tenants.

pub mod bank;
pub mod bank_account;
pub mod bank_account_repo;
pub mod bank_account_routes;
pub mod bank_routes;
pub mod banks_repo;

use axum::Router;
use axum::routing::{get, post, put};
use utoipa::OpenApi as _;
use where_api::ModulePlugin;
use where_api::state::AppState;
use where_modules::ModuleId;

/// The accounting and invoicing module.
#[derive(Debug, Default, Clone, Copy)]
pub struct MoneyModule;

impl ModulePlugin for MoneyModule {
    fn id(&self) -> ModuleId {
        ModuleId::Money
    }

    fn router(&self) -> Router<AppState> {
        Router::new()
            .route("/banks", get(bank_routes::list).post(bank_routes::create))
            .route("/banks/source", get(bank_routes::source))
            .route("/banks/sync", post(bank_routes::sync))
            .route(
                "/banks/{id}",
                put(bank_routes::update).delete(bank_routes::remove),
            )
            .route(
                "/bank-accounts",
                get(bank_account_routes::list).post(bank_account_routes::create),
            )
            .route("/bank-accounts/convert", post(bank_account_routes::convert))
            .route(
                "/bank-accounts/{id}",
                get(bank_account_routes::get_one)
                    .put(bank_account_routes::replace)
                    .delete(bank_account_routes::remove),
            )
            .route(
                "/bank-accounts/{id}/standard",
                put(bank_account_routes::set_standard),
            )
    }

    fn openapi(&self) -> utoipa::openapi::OpenApi {
        ApiDoc::openapi()
    }
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags(
        (name = "banken", description = "Bankliste (SWIFT/BIC)"),
        (name = "bankverbindungen", description = "Bankverbindungen des Betriebs")
    ),
    paths(
        bank_routes::list,
        bank_routes::create,
        bank_routes::update,
        bank_routes::remove,
        bank_routes::source,
        bank_routes::sync,
        bank_account_routes::list,
        bank_account_routes::get_one,
        bank_account_routes::create,
        bank_account_routes::replace,
        bank_account_routes::remove,
        bank_account_routes::set_standard,
        bank_account_routes::convert,
    )
)]
struct ApiDoc;
