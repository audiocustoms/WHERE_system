//! Calendar (in epirent "TeamAgenda"): appointments of a range, their series and calendars.
//!
//! The view asks for one range (month or week) and gets every occurrence in it —
//! a weekly series therefore arrives as one entry per week, not as one entry.

use crate::state::State;
use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use where_domain::{
    AgendaCategory, AgendaCategoryInput, AgendaEvent, AgendaEventInput, AgendaRange,
};

use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::AppState;

/// Range and filters of the calendar view.
#[derive(Debug, Deserialize)]
pub struct AgendaParams {
    pub from: Option<String>,
    pub to: Option<String>,
    pub category: Option<i32>,
    pub user: Option<i32>,
    pub q: Option<String>,
}

impl AgendaParams {
    fn range(&self) -> Result<(NaiveDate, NaiveDate), ApiError> {
        let today = chrono::Local::now().date_naive();
        let from = match &self.from {
            Some(text) => parse_date(text)?,
            None => NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today),
        };
        let to = match &self.to {
            Some(text) => parse_date(text)?,
            None => end_of_month(from),
        };
        if to < from {
            return Err(ApiError(where_domain::DomainError::Validation(
                "Das Ende des Zeitraums liegt vor dem Beginn.".into(),
            )));
        }
        Ok((from, to))
    }
}

fn parse_date(text: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d").map_err(|_| {
        ApiError(where_domain::DomainError::Validation(format!(
            "„{text}\u{201c} ist kein Datum im Format JJJJ-MM-TT."
        )))
    })
}

fn end_of_month(date: NaiveDate) -> NaiveDate {
    let (year, month) = (date.year(), date.month());
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.and_then(|first| first.pred_opt()).unwrap_or(date)
}

/// Appointments of a range, series expanded.
#[utoipa::path(
    get,
    path = "/api/v1/agenda",
    tag = "zentrale",
    params(
        ("from" = Option<String>, Query, description = "First day, `YYYY-MM-DD` (default: first of the month)"),
        ("to" = Option<String>, Query, description = "Last day, `YYYY-MM-DD` (default: end of the month)"),
        ("category" = Option<i32>, Query, description = "Only this calendar (`Storage_TACategory`)"),
        ("user" = Option<i32>, Query, description = "Also show this user's private appointments"),
        ("q" = Option<String>, Query, description = "Search in title, text, venue and address")
    ),
    responses(
        (status = 200, description = "Occurrences of the range", body = AgendaRange),
        (status = 422, description = "Invalid date", body = crate::error::Problem)
    )
)]
pub async fn range(
    State(state): State<AppState>,
    Query(params): Query<AgendaParams>,
) -> Result<Json<AgendaRange>, ApiError> {
    let (from, to) = params.range()?;
    let search = params.q.as_deref().map(str::trim).filter(|q| !q.is_empty());
    Ok(Json(
        state
            .agenda
            .range(from, to, params.category, params.user, search)
            .await?,
    ))
}

/// One appointment including its series settings.
#[utoipa::path(
    get,
    path = "/api/v1/agenda/{id}",
    tag = "zentrale",
    params(("id" = i32, Path, description = "Appointment")),
    responses(
        (status = 200, body = AgendaEvent),
        (status = 404, description = "Unknown appointment", body = crate::error::Problem)
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<AgendaEvent>, ApiError> {
    Ok(Json(state.agenda.get(id).await?))
}

/// Creates an appointment.
#[utoipa::path(
    post,
    path = "/api/v1/agenda",
    tag = "zentrale",
    request_body = AgendaEventInput,
    responses(
        (status = 201, body = AgendaEvent),
        (status = 422, description = "Invalid appointment", body = crate::error::Problem)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<AgendaEventInput>,
) -> Result<(StatusCode, Json<AgendaEvent>), ApiError> {
    let event = state.agenda.create(input, current_user()).await?;
    Ok((StatusCode::CREATED, Json(event)))
}

/// Replaces an appointment; a changed series is written to the same rule.
#[utoipa::path(
    put,
    path = "/api/v1/agenda/{id}",
    tag = "zentrale",
    params(("id" = i32, Path, description = "Appointment")),
    request_body = AgendaEventInput,
    responses(
        (status = 200, body = AgendaEvent),
        (status = 404, description = "Unknown appointment", body = crate::error::Problem),
        (status = 422, description = "Invalid appointment", body = crate::error::Problem)
    )
)]
pub async fn replace(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(input): Json<AgendaEventInput>,
) -> Result<Json<AgendaEvent>, ApiError> {
    Ok(Json(state.agenda.update(id, input, current_user()).await?))
}

/// Deletes an appointment or a whole series.
#[utoipa::path(
    delete,
    path = "/api/v1/agenda/{id}",
    tag = "zentrale",
    params(("id" = i32, Path, description = "Appointment")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown appointment", body = crate::error::Problem)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.agenda.delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

#[derive(Debug, Deserialize)]
pub struct SkipParams {
    /// First day of the occurrence that should disappear from the series.
    pub date: String,
}

/// Removes one occurrence of a series, the series itself stays.
#[utoipa::path(
    delete,
    path = "/api/v1/agenda/{id}/occurrence",
    tag = "zentrale",
    params(
        ("id" = i32, Path, description = "Appointment"),
        ("date" = String, Query, description = "First day of the occurrence, `YYYY-MM-DD`")
    ),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 422, description = "Not a series or invalid date", body = crate::error::Problem)
    )
)]
pub async fn skip_occurrence(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(params): Query<SkipParams>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let date = parse_date(&params.date)?;
    state.agenda.skip_occurrence(id, date).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Calendars of the left column.
#[utoipa::path(
    get,
    path = "/api/v1/agenda/categories",
    tag = "zentrale",
    responses((status = 200, body = Vec<AgendaCategory>))
)]
pub async fn categories(
    State(state): State<AppState>,
) -> Result<Json<Vec<AgendaCategory>>, ApiError> {
    Ok(Json(state.agenda.categories().await?))
}

/// Creates a calendar.
#[utoipa::path(
    post,
    path = "/api/v1/agenda/categories",
    tag = "zentrale",
    request_body = AgendaCategoryInput,
    responses(
        (status = 201, body = AgendaCategory),
        (status = 422, description = "Name missing", body = crate::error::Problem)
    )
)]
pub async fn category_create(
    State(state): State<AppState>,
    Json(input): Json<AgendaCategoryInput>,
) -> Result<(StatusCode, Json<AgendaCategory>), ApiError> {
    let name = input.validated()?;
    Ok((
        StatusCode::CREATED,
        Json(state.agenda.create_category(&name).await?),
    ))
}

/// Renames a calendar.
#[utoipa::path(
    put,
    path = "/api/v1/agenda/categories/{id}",
    tag = "zentrale",
    params(("id" = i32, Path, description = "Calendar")),
    request_body = AgendaCategoryInput,
    responses(
        (status = 200, body = AgendaCategory),
        (status = 404, description = "Unknown calendar", body = crate::error::Problem),
        (status = 422, description = "Name missing", body = crate::error::Problem)
    )
)]
pub async fn category_update(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(input): Json<AgendaCategoryInput>,
) -> Result<Json<AgendaCategory>, ApiError> {
    let name = input.validated()?;
    Ok(Json(state.agenda.rename_category(id, &name).await?))
}

/// Deletes an empty calendar.
#[utoipa::path(
    delete,
    path = "/api/v1/agenda/categories/{id}",
    tag = "zentrale",
    params(("id" = i32, Path, description = "Calendar")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 409, description = "Calendar still holds appointments", body = crate::error::Problem)
    )
)]
pub async fn category_delete(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<DeletedResponse>, ApiError> {
    state.agenda.delete_category(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// The application has no login yet, so the author of a change is fixed.
fn current_user() -> &'static str {
    "where"
}
