//! HTTP API layer: Axum router, DTOs, problem-document error mapping and OpenAPI.
//!
//! The route surface mirrors the resource names of the original 4D REST API
//! (`rest_urlopt_<resource>`, 24 resources). Phase 0 implements the two master
//! data resources end to end; the remaining ones follow the same pattern.

pub mod authz;
pub mod dto;
pub mod error;
pub mod modules;
pub mod openapi;
pub mod routes;
pub mod state;

pub use error::ApiError;
pub use modules::ModulePlugin;
pub use openapi::ApiDoc;
pub use state::{AppState, TenantPools};

use axum::Router;
use axum::http::{HeaderValue, Method, header};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// CORS policy for the Tauri webview and the Vite dev server.
///
/// The desktop shell serves the SPA from `http://tauri.localhost`, which is a
/// different origin than the API on `127.0.0.1:8090`, so the browser `fetch`
/// calls need an explicit allow-list. `configured` comes from the runtime
/// settings; without it `CORS_ALLOWED_ORIGINS` (comma separated) applies, and
/// the built-in list is the last fallback.
fn cors(configured: Option<&str>) -> CorsLayer {
    const DEFAULT_ORIGINS: &str = "http://tauri.localhost,https://tauri.localhost,tauri://localhost,http://localhost:5173,http://127.0.0.1:5173";

    let list = configured
        .map(str::to_string)
        .or_else(|| std::env::var("CORS_ALLOWED_ORIGINS").ok())
        .unwrap_or_else(|| DEFAULT_ORIGINS.to_string());

    let origins: Vec<HeaderValue> = list
        .split(',')
        .filter_map(|origin| origin.trim().parse::<HeaderValue>().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION])
}

/// Builds the complete application router with the built-in CORS list.
pub fn router(state: AppState) -> Router {
    router_with_origins(state, None)
}

/// Builds the router with an explicit CORS allow-list.
///
/// A central server that serves workstations from other hosts needs those
/// origins in the list; the desktop single-place installation does not.
pub fn router_with_origins(state: AppState, origins: Option<&str>) -> Router {
    router_with_modules(state, origins, &[])
}

/// Builds the router including the given module plug-ins.
///
/// A module that is compiled in but not installed in the database still gets
/// its routes mounted; [`modules::require_module`] answers `409` until the
/// product is registered. Installing therefore does not need a new binary,
/// only a refreshed registry (the admin page and the installer both do that).
pub fn router_with_modules(
    state: AppState,
    origins: Option<&str>,
    plugins: &[&dyn ModulePlugin],
) -> Router {
    let mut api = routes::api_v1();
    for plugin in plugins {
        let id = plugin.id();
        let gate =
            axum::middleware::from_fn_with_state((state.clone(), id), modules::require_module);
        api = api.merge(plugin.router().route_layer(gate));
    }
    // Check session and right of every request (`authz`) before a handler runs.
    let api = api.layer(axum::middleware::from_fn_with_state(
        state.clone(),
        authz::authorize,
    ));

    let mut document = ApiDoc::openapi();
    for plugin in plugins {
        document.merge(plugin.openapi());
    }

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", document))
        .nest("/api/v1", api)
        .merge(routes::health::router())
        .layer(cors(origins))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
