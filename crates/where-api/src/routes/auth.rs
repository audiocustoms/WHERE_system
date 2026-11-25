//! Login and logout (`img/Admin Menü/Login Logout`).
//!
//! `GET /auth/users` feeds the picker of the mask, `POST /auth/login` opens a session,
//! `GET /auth/session` restores it after a reload and `POST /auth/logout` ends it. The
//! token travels as `Authorization: Bearer …`; the mask keeps it in `sessionStorage`,
//! so closing the window signs the user out.

use crate::state::State;
use axum::Json;
use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode};
use serde::Deserialize;
use where_domain::{
    DomainError, LoginRequest, LoginResponse, LoginTenant, LoginUser, PasswordChange, SessionUser,
};

use crate::error::ApiError;
use crate::state::AppState;

/// The `Bearer` token of a request, if there is one.
pub fn bearer(headers: &HeaderMap) -> Option<String> {
    let value = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?;
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_string())
}

/// `?mandant=<id>` of the account list.
#[derive(Debug, Default, Deserialize)]
pub struct MandantQuery {
    pub mandant: Option<i64>,
}

/// The tenant a session is moved to (`Mandantenwechsel` in the header).
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct MandantInput {
    pub mandant: i64,
}

/// The tenants the mask offers next to the account list.
#[utoipa::path(
    get,
    path = "/api/v1/auth/tenants",
    tag = "anmeldung",
    responses((status = 200, body = Vec<LoginTenant>))
)]
pub async fn tenants(State(state): State<AppState>) -> Result<Json<Vec<LoginTenant>>, ApiError> {
    Ok(Json(state.auth.tenants().await?))
}

/// The accounts the mask offers — of one tenant when `mandant` is given.
#[utoipa::path(
    get,
    path = "/api/v1/auth/users",
    tag = "anmeldung",
    params(("mandant" = Option<i64>, Query, description = "Mandant der Auswahl")),
    responses((status = 200, body = Vec<LoginUser>))
)]
pub async fn users(
    State(state): State<AppState>,
    Query(query): Query<MandantQuery>,
) -> Result<Json<Vec<LoginUser>>, ApiError> {
    Ok(Json(state.auth.candidates(query.mandant).await?))
}

/// Signs a user in; wrong credentials answer with `401`.
#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "anmeldung",
    request_body = LoginRequest,
    responses(
        (status = 200, body = LoginResponse),
        (status = 401, description = "Wrong name or password", body = crate::error::Problem),
        (status = 422, description = "Missing name or password", body = crate::error::Problem)
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let (user_name, password, mandant) = input.prepare()?;
    Ok(Json(
        state.auth.login(&user_name, &password, mandant).await?,
    ))
}

/// The account behind the token — used to restore the session after a reload.
#[utoipa::path(
    get,
    path = "/api/v1/auth/session",
    tag = "anmeldung",
    responses(
        (status = 200, body = SessionUser),
        (status = 401, description = "No valid session", body = crate::error::Problem)
    )
)]
pub async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionUser>, ApiError> {
    let token = bearer(&headers).ok_or_else(|| {
        ApiError::from(DomainError::Unauthorized("Nicht angemeldet.".to_string()))
    })?;
    match state.auth.session(&token).await? {
        Some(user) => Ok(Json(user)),
        None => Err(ApiError::from(DomainError::Unauthorized(
            "Die Sitzung ist abgelaufen.".to_string(),
        ))),
    }
}

/// Moves the session to another tenant of the account.
#[utoipa::path(
    post,
    path = "/api/v1/auth/mandant",
    tag = "anmeldung",
    request_body = MandantInput,
    responses(
        (status = 200, description = "Die Sitzung steht auf dem Mandanten", body = SessionUser),
        (status = 401, description = "No valid session", body = crate::error::Problem),
        (status = 403, description = "Account of another tenant", body = crate::error::Problem),
        (status = 404, description = "Unknown tenant", body = crate::error::Problem)
    )
)]
pub async fn switch_mandant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<MandantInput>,
) -> Result<Json<SessionUser>, ApiError> {
    let token = bearer(&headers).ok_or_else(|| {
        ApiError::from(DomainError::Unauthorized("Nicht angemeldet.".to_string()))
    })?;
    Ok(Json(
        state.auth.switch_mandant(&token, input.mandant).await?,
    ))
}

/// Ends the session of the token; an unknown token is not an error.
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "anmeldung",
    responses((status = 204, description = "Signed out"))
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    if let Some(token) = bearer(&headers) {
        state.auth.logout(&token).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Sets a new password for the signed-in account — the first login of an
/// installation passes through here.
#[utoipa::path(
    post,
    path = "/api/v1/auth/password",
    tag = "anmeldung",
    request_body = PasswordChange,
    responses(
        (status = 204, description = "Password changed"),
        (status = 401, description = "No valid session", body = crate::error::Problem),
        (status = 422, description = "Too short, not repeated or unchanged", body = crate::error::Problem)
    )
)]
pub async fn password(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<PasswordChange>,
) -> Result<StatusCode, ApiError> {
    let token = bearer(&headers).ok_or_else(|| {
        ApiError::from(DomainError::Unauthorized("Nicht angemeldet.".to_string()))
    })?;
    let password = input.prepare()?;
    state.auth.change_password(&token, &password).await?;
    Ok(StatusCode::NO_CONTENT)
}
