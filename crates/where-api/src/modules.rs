//! Plug-in contract for the WHERE modules.
//!
//! The core router knows nothing about orders, accounting or personnel. A
//! module crate implements [`ModulePlugin`] and hands its routes and OpenAPI
//! fragment in; the server binary assembles the list at start-up. Every
//! module router carries [`require_module`], so a module that is compiled in but
//! not installed in the database answers with `409` instead of serving data.
//!
//! This keeps the dependency direction acyclic: modules depend on `where-api`
//! (state, errors, DTOs), never the other way round.

use axum::Router;
use axum::extract::{Request, State as AxumState};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use utoipa::openapi::OpenApi;
use where_domain::DomainError;
use where_modules::ModuleId;

use crate::error::ApiError;
use crate::state::AppState;

/// One installable part of WHERE.
pub trait ModulePlugin: Send + Sync + 'static {
    /// Which product this plug-in implements.
    fn id(&self) -> ModuleId;

    /// Routes relative to `/api/v1`, e.g. `/orders/timeline`.
    fn router(&self) -> Router<AppState>;

    /// Paths and schemas this module contributes to `/api-docs/openapi.json`.
    fn openapi(&self) -> OpenApi;
}

/// Rejects requests to a module that is not installed in this database.
pub async fn require_module(
    AxumState((state, id)): AxumState<(AppState, ModuleId)>,
    request: Request,
    next: Next,
) -> Response {
    if state.modules.is_installed(id) {
        return next.run(request).await;
    }
    let name = id.manifest().product_name;
    ApiError::from(DomainError::Conflict(format!(
        "Das Modul „{name}“ ist nicht installiert."
    )))
    .into_response()
}
