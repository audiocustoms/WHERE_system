//! Router-level tests that need no live database.
//!
//! The pool is created lazily, so request routing, OpenAPI generation and
//! problem-document mapping are exercised while any handler that touches the
//! database reports the expected failure mode.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use where_api::{AppState, ModulePlugin, router, router_with_modules};
use where_modules::ModuleId;

fn test_router() -> axum::Router {
    let pool = where_persistence::pool_lazy("postgres://where:where@127.0.0.1:5432/where")
        .expect("lazy pool");
    router(AppState::new(pool))
}

/// A plug-in that stands in for a real module: one route, no OpenAPI paths.
struct TestPlugin;

impl ModulePlugin for TestPlugin {
    fn id(&self) -> ModuleId {
        ModuleId::Go
    }

    fn router(&self) -> axum::Router<AppState> {
        axum::Router::new().route(
            "/orders/timeline",
            axum::routing::get(|| async { "module" }),
        )
    }

    fn openapi(&self) -> utoipa::openapi::OpenApi {
        utoipa::openapi::OpenApi::default()
    }
}

fn test_router_with_plugin() -> axum::Router {
    let pool = where_persistence::pool_lazy("postgres://where:where@127.0.0.1:5432/where")
        .expect("lazy pool");
    router_with_modules(AppState::new(pool), None, &[&TestPlugin])
}

async fn body_string(response: axum::response::Response) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

#[tokio::test]
async fn health_reports_ok() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert!(
        body.contains("\"status\":\"ok\""),
        "unexpected body: {body}"
    );
}

#[tokio::test]
async fn ready_reports_degraded_without_database() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = body_string(response).await;
    assert!(
        body.contains("\"database\":false"),
        "unexpected body: {body}"
    );
}

#[tokio::test]
async fn openapi_document_lists_the_phase_zero_resources() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    for expected in [
        "/api/v1/addresses",
        "/api/v1/addresses/duplicates",
        "/api/v1/addresses/search",
        "/api/v1/addresses/{id}",
        "/api/v1/articles",
        "/api/v1/articles/{id}",
        "/api/v1/master-data/article-groups",
        "/api/v1/master-data/article-groups/{id}",
        "/api/v1/master-data/number-series",
        "/api/v1/master-data/number-series/{key}",
        "/api/v1/master-data/number-series/{key}/reset",
        "/api/v1/articles/{id}/stock",
        "/api/v1/articles/{id}/devices",
        "/api/v1/articles/{id}/devices/{device_id}",
        "/api/v1/articles/search",
        "/api/v1/articles/searches",
        "/api/v1/articles/searches/{id}",
        "/api/v1/master-data/search-index",
        "/api/v1/modules",
        "/health",
        "/ready",
    ] {
        assert!(body.contains(expected), "missing {expected} in OpenAPI");
    }
    assert!(
        body.contains("\"AddressSearchInput\""),
        "missing AddressSearchInput schema"
    );
    assert!(body.contains("\"AddressDto\""), "missing AddressDto schema");
    assert!(
        body.contains("\"NumberSeriesDto\""),
        "missing NumberSeriesDto schema"
    );
    assert!(
        body.contains("\"GoodsReceiptInput\""),
        "missing GoodsReceiptInput schema"
    );
    assert!(
        body.contains("\"ArticleSearchInput\""),
        "missing ArticleSearchInput schema"
    );
    assert!(
        body.contains("\"PyroSearchInput\""),
        "missing PyroSearchInput schema"
    );
    assert!(
        body.contains("\"ArticlePricing\""),
        "missing ArticlePricing schema"
    );
    assert!(
        body.contains("\"ScaleCategory\""),
        "missing ScaleCategory schema"
    );
}

#[tokio::test]
async fn a_business_route_asks_for_a_session_before_it_parses_anything() {
    // The rights check runs before every handler: without a session it answers 401,
    // even before the criteria of the search form are checked (that would be 422).
    let response = test_router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/addresses/search")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"address_type":"vielleicht"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_string(response).await;
    assert!(body.contains("Anmeldung"), "unexpected body: {body}");
}

#[tokio::test]
async fn a_wrong_token_is_refused() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/articles")
                .header("authorization", "Bearer unbekannt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without a database the session cannot be checked — so it counts as expired.
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "unexpected status {}",
        response.status()
    );
}

#[tokio::test]
async fn the_login_mask_stays_open() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // No 401: the list of the mask hangs on no session (without a database the
    // handler answers with a server error).
    assert_ne!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_plugin_route_is_mounted_but_asks_for_a_session_first() {
    // Without a session the rights check answers 401; 404 would mean that the
    // module route is not mounted at all.
    let response = test_router_with_plugin()
        .oneshot(
            Request::builder()
                .uri("/api/v1/orders/timeline")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unknown_resource_returns_not_found() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/doesnotexist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_paging_is_clamped_instead_of_failing() {
    // limit=0 must not reach the database as LIMIT 0; the window is clamped to 1.
    // Without a database the handler fails later, so only the routing contract is
    // asserted here: the request must not be rejected as a bad request.
    let response = test_router()
        .oneshot(
            Request::builder()
                .uri("/api/v1/addresses?limit=0&offset=-5")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn cors_preflight_allows_the_tauri_webview_origin() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/addresses")
                .header("origin", "http://tauri.localhost")
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status().is_success(),
        "preflight rejected: {}",
        response.status()
    );
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://tauri.localhost")
    );
}

#[tokio::test]
async fn cors_rejects_an_unlisted_origin() {
    let response = test_router()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/addresses")
                .header("origin", "https://evil.example")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_none(),
        "unexpected allow-origin for a foreign origin"
    );
}

#[test]
fn paging_window_is_bounded() {
    use where_domain::PageRequest;

    assert_eq!(PageRequest::default().normalized(), (50, 0));
    assert_eq!(
        PageRequest {
            limit: Some(10_000),
            offset: Some(-3)
        }
        .normalized(),
        (500, 0)
    );
    assert_eq!(
        PageRequest {
            limit: Some(0),
            offset: Some(0)
        }
        .normalized(),
        (1, 0)
    );
}
