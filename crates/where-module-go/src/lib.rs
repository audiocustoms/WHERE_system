//! `WHERE - does my stuff go?` — orders, package slips and article pricing.
//!
//! The crate carries the order settings (`Zeitplan`, `Überschriften`, `Makros`),
//! the scale lists of the rental and sale tabs and the module flag for the
//! article mask. It plugs into the core through [`where_api::ModulePlugin`];
//! the core itself contains no order code.
//!
//! Data lives in the shared database (`where_order_*`, `StaffelKategorie`), so a
//! module installation that existed before the core keeps its data when the
//! core joins.

pub mod order_settings;
pub mod order_settings_repo;
pub mod routes;

use axum::Router;
use axum::routing::{get, put};
use utoipa::OpenApi as _;
use where_api::ModulePlugin;
use where_api::state::AppState;
use where_modules::ModuleId;

/// The order and package-slip module.
#[derive(Debug, Default, Clone, Copy)]
pub struct GoModule;

impl ModulePlugin for GoModule {
    fn id(&self) -> ModuleId {
        ModuleId::Go
    }

    fn router(&self) -> Router<AppState> {
        Router::new()
            .route("/orders/timeline", get(routes::timeline))
            .route("/orders/timeline/{key}", put(routes::timeline_save))
            .route("/orders/headings", get(routes::headings))
            .route("/orders/headings/{number}", put(routes::heading_save))
            .route(
                "/orders/macros",
                get(routes::macros).post(routes::macro_create),
            )
            .route(
                "/orders/macros/{id}",
                put(routes::macro_update).delete(routes::macro_delete),
            )
            .route("/master-data/scales", get(routes::scales))
            .route("/master-data/scales/{id}/steps", get(routes::scale_steps))
    }

    fn openapi(&self) -> utoipa::openapi::OpenApi {
        ApiDoc::openapi()
    }
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags(
        (name = "auftraege", description = "Auftrags- und Packscheineinstellungen sowie Staffeln")
    ),
    paths(
        routes::timeline,
        routes::timeline_save,
        routes::headings,
        routes::heading_save,
        routes::macros,
        routes::macro_create,
        routes::macro_update,
        routes::macro_delete,
        routes::scales,
        routes::scale_steps,
    )
)]
struct ApiDoc;
