//! Employee endpoints of `WHERE - are my people?`.

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Extension, Path, Query};
use axum::http::StatusCode;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{Datelike, NaiveDate, Utc};
use serde::Deserialize;
use utoipa::ToSchema;
use where_api::ApiError;
use where_api::authz::DepartmentScope;
use where_api::dto::ListParams;
use where_api::routes::addresses::DeletedResponse;
use where_api::state::{AppState, State};
use where_domain::{DomainError, SessionUser};

use crate::demand::{DayDemand, DemandCopied, DemandCopyInput, DemandEntry, DemandInput};
use crate::demand_repo::DemandRepository;
use crate::dispo::{
    self, DispoDayInput, DispoEntry, DispoIgnoredInput, DispoImportReport, DispoQuery, DispoScan,
};
use crate::dispo_demand::{self, DayProposal};
use crate::dispo_repo::DispoRepository;
use crate::employee::{Employee, EmployeeInput, EmployeeUserInput, NextPersonnelNo};
use crate::employee_repo::EmployeeRepository;
use crate::exchange::{self, ExchangeQuery, ExchangeReport, ExchangeScan};
use crate::flag_repo::FlagRepository;
use crate::flags::{
    Flag, FlagExclusionsInput, FlagInput, FlagPresetReport, FlagStaffing, FlagStaffingInput,
};
use crate::generate::{self, GenerateInput, GenerateReport};
use crate::holiday_repo::HolidayRepository;
use crate::holiday_source::HolidaySource;
use crate::holidays::{
    Holiday, HolidayCalendar, HolidayCounts, HolidayImportReport, SCOPE_NATIONAL, SCOPE_REGIONAL,
    SchoolHoliday,
};
use crate::ical::{self, IcalSendInput, IcalSendReport, IcalSkip};
use crate::list_repo::PeopleListRepository;
use crate::lists::{
    DEFAULT_TEXT_ROW, ListColorInput, ListEntry, ListEntryInput, ListKind, ListOrderInput,
    LocationDepartmentInput, LocationForemanInput,
};
use crate::masterdata::{self, MasterArea, MasterReport};
use crate::planning::PlanningRules;
use crate::planning_repo::PlanningRepository;
use crate::schedule::{
    BoardQuery, DepartmentQuery, KIND_FREE, KIND_SHIFT, KIND_SICK, KIND_VACATION, MAX_NOTE_CHARS,
    MonthQuery, ResetReport, RestoreCell, ScheduleEntry, ScheduleInput, ScheduleQuery,
    ScheduleSeriesInput, ScheduleSeriesResult,
};
use crate::schedule_audit::{self, AuditCell, AuditPerson, ScheduleAudit};
use crate::schedule_repo::ScheduleRepository;
use crate::scope::PeopleScope;
use crate::shift_repo::ShiftRepository;
use crate::shifts::{Shift, ShiftExclusionsInput, ShiftInput};
use where_persistence::UserRepository;

/// The right that unlocks birth data, address and the social security numbers.
pub const PERSONAL_DATA_RIGHT: &str = "personal.persoenliche-daten-sehen";

fn employees(state: &AppState) -> EmployeeRepository {
    EmployeeRepository::new(state.pool.clone())
}

fn people_list(state: &AppState, kind: ListKind) -> PeopleListRepository {
    PeopleListRepository::new(state.pool.clone(), kind)
}

fn shifts(state: &AppState) -> ShiftRepository {
    ShiftRepository::new(state.pool.clone())
}

fn planning(state: &AppState) -> PlanningRepository {
    PlanningRepository::new(state.pool.clone())
}

fn schedule(state: &AppState) -> ScheduleRepository {
    ScheduleRepository::new(state.pool.clone())
}

fn demand(state: &AppState) -> DemandRepository {
    DemandRepository::new(state.pool.clone())
}

fn dispo_entries(state: &AppState) -> DispoRepository {
    DispoRepository::new(state.pool.clone())
}

fn flags(state: &AppState) -> FlagRepository {
    FlagRepository::new(state.pool.clone())
}

fn holiday_calendar(state: &AppState) -> HolidayRepository {
    HolidayRepository::new(state.pool.clone())
}

/// Location id → name: the houses of the disposition.
async fn house_names(state: &AppState) -> Result<BTreeMap<i64, dispo_demand::House>, ApiError> {
    Ok(people_list(state, ListKind::Locations)
        .list()
        .await?
        .into_iter()
        .map(|location| {
            (
                location.id,
                dispo_demand::House {
                    name: location.name,
                    needs_foreman: location.needs_foreman,
                },
            )
        })
        .collect())
}

/// Locations (lower case) that are run without a foreman.
async fn foremanless_houses(state: &AppState) -> Result<BTreeSet<String>, ApiError> {
    Ok(people_list(state, ListKind::Locations)
        .list()
        .await?
        .into_iter()
        .filter(|location| !location.needs_foreman)
        .map(|location| location.name.trim().to_lowercase())
        .collect())
}

/// Location name (lower case) → the sub-department that works there.
async fn house_subdepartments(state: &AppState) -> Result<HashMap<String, String>, ApiError> {
    Ok(people_list(state, ListKind::Locations)
        .list()
        .await?
        .into_iter()
        .filter_map(|location| {
            let department = location.department?;
            let department = department.trim();
            if department.is_empty() {
                return None;
            }
            Some((location.name.trim().to_lowercase(), department.to_string()))
        })
        .collect())
}

/// `40.00` hours as `2400` minutes.
fn weekly_minutes(hours: rust_decimal::Decimal) -> i32 {
    use rust_decimal::prelude::ToPrimitive;
    (hours.to_f64().unwrap_or(0.0) * 60.0).round() as i32
}

/// The locations of the disposition the session may work in: a location belongs to the
/// department of its sub-department. `only` narrows the result to one department (the
/// view filter of the board). Without a restriction every location counts; a location
/// without a sub-department belongs to nobody.
async fn disposible_locations(
    state: &AppState,
    scope: &PeopleScope,
    only: Option<&str>,
) -> Result<Vec<ListEntry>, ApiError> {
    let orte = people_list(state, ListKind::Locations).list().await?;
    if scope.dispo_all() && only.is_none() {
        return Ok(orte);
    }
    let subdepartments: HashMap<String, String> = people_list(state, ListKind::Subdepartments)
        .list()
        .await?
        .into_iter()
        .filter_map(|entry| {
            let department = entry.department?;
            let department = department.trim().to_lowercase();
            if department.is_empty() {
                return None;
            }
            Some((entry.name.trim().to_lowercase(), department))
        })
        .collect();
    Ok(orte
        .into_iter()
        .filter(|ort| {
            // A typed row belongs to nobody's department, so every dispatcher sees it.
            if ort.is_text {
                return true;
            }
            match ort.department.as_deref() {
                Some(name) => match subdepartments.get(&name.trim().to_lowercase()) {
                    Some(department) => {
                        scope.allows_dispo(department)
                            && match only {
                                Some(only) => only == department,
                                None => true,
                            }
                    }
                    None => false,
                },
                None => false,
            }
        })
        .collect())
}

/// The ids of those locations, `None` when `Personal-Disposition` is not restricted and
/// no department was asked for.
async fn disposible_location_ids(
    state: &AppState,
    scope: &PeopleScope,
    only: Option<&str>,
) -> Result<Option<HashSet<i64>>, ApiError> {
    if scope.dispo_all() && only.is_none() {
        return Ok(None);
    }
    Ok(Some(
        disposible_locations(state, scope, only)
            .await?
            .into_iter()
            .map(|ort| ort.id)
            .collect(),
    ))
}

/// Booked minus owed minutes per employee from `start` up to the day before
/// `until` — where every employee stands when the period starts.
async fn balance_before(
    state: &AppState,
    shifts: &[Shift],
    balance: &generate::Balance,
    start: NaiveDate,
    until: NaiveDate,
    people: &[generate::Person],
) -> Result<HashMap<i64, i32>, ApiError> {
    let mut booked: HashMap<i64, i32> = HashMap::new();
    if start < until {
        for entry in schedule(state)
            .range(start, until.pred_opt().unwrap_or(start))
            .await?
        {
            let cell = generate::Cell {
                employee_id: entry.employee_id,
                day: entry.day,
                kind: entry.kind,
                shift_id: entry.shift_id,
                starts_at: entry.starts_at,
                ends_at: entry.ends_at,
                break_minutes: entry.break_minutes,
            };
            *booked.entry(entry.employee_id).or_insert(0) += balance.minutes_of(&cell, shifts);
        }
    }
    let mut before = HashMap::new();
    for person in people {
        let owed: i32 = start
            .iter_days()
            .take_while(|day| *day < until)
            .map(|day| generate::target_minutes(person, day))
            .sum();
        let have = booked.get(&person.id).copied().unwrap_or(0);
        before.insert(person.id, have - owed);
    }
    Ok(before)
}

/// Whether the cell is a free day: the cell of a shift of the `frei` band.
fn is_free_day(shifts: &[Shift], cell: &generate::Cell) -> bool {
    cell.shift_id
        .and_then(|id| shifts.iter().find(|shift| shift.id == id))
        .map(|shift| shift.band.is_free())
        .unwrap_or(false)
}

/// The ids of the free days the generator writes: `FT` for an empty day and
/// `SoFT` for the compensation of a worked Sunday (short name, band `frei`).
fn free_shift_ids(shifts: &[Shift]) -> (Option<i64>, Option<i64>) {
    let find = |name: &str| {
        shifts
            .iter()
            .find(|shift| {
                shift.band.is_free()
                    && shift
                        .short_name
                        .as_deref()
                        .map(|short| short.trim().eq_ignore_ascii_case(name))
                        .unwrap_or(false)
            })
            .map(|shift| shift.id)
    };
    (find("FT"), find("SoFT"))
}

/// `departments`, `subdepartments`, `positions` or `locations` from the path; everything
/// else is unknown.
fn list_kind(key: &str) -> Result<ListKind, ApiError> {
    ListKind::from_key(key).ok_or_else(|| {
        ApiError::from(where_domain::DomainError::NotFoundKey {
            entity: "people list",
            key: format!("{key} (departments, subdepartments, positions, locations)"),
        })
    })
}

fn may_see_personal_data(user: &SessionUser) -> bool {
    user.has(PERSONAL_DATA_RIGHT)
}

/// All employees, optionally filtered by a search term.
#[utoipa::path(
    get,
    path = "/api/v1/people",
    tag = "personal",
    params(ListParams),
    responses((status = 200, body = Vec<Employee>))
)]
pub async fn list(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Extension(scope): Extension<DepartmentScope>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Employee>>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let list = scope.keep(employees(&state).list(params.search()).await?);
    // The names of the employees of both areas (the board needs them), the
    // personal data only from `Personal`.
    let list = list
        .into_iter()
        .map(|employee| {
            if may_see_personal_data(&user)
                && scope.allows_personal(employee.department.as_deref().unwrap_or(""))
            {
                employee
            } else {
                employee.without_personal_data()
            }
        })
        .collect();
    Ok(Json(list))
}

/// One employee.
#[utoipa::path(
    get,
    path = "/api/v1/people/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Employee id")),
    responses(
        (status = 200, body = Employee),
        (status = 404, description = "Unknown", body = where_api::error::Problem)
    )
)]
pub async fn get_one(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Extension(scope): Extension<DepartmentScope>,
    Path(id): Path<i64>,
) -> Result<Json<Employee>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let employee = employees(&state).get(id).await?;
    scope.check(&employee)?;
    let employee = if may_see_personal_data(&user)
        && scope.allows_personal(employee.department.as_deref().unwrap_or(""))
    {
        employee
    } else {
        employee.without_personal_data()
    };
    Ok(Json(employee))
}

/// The next personnel number from the number series — suggestion for the mask.
#[utoipa::path(
    get,
    path = "/api/v1/people/new",
    tag = "personal",
    responses(
        (status = 200, body = NextPersonnelNo),
        (status = 409, description = "Number series missing or inactive", body = where_api::error::Problem)
    )
)]
pub async fn next_number(State(state): State<AppState>) -> Result<Json<NextPersonnelNo>, ApiError> {
    let series = state
        .number_series
        .get(crate::employee_repo::NUMBER_SERIES)
        .await?;
    let personnel_no = series
        .preview_today()
        .map(|(_, text)| text)
        .ok_or_else(|| {
            ApiError::from(where_domain::DomainError::Conflict(
                "The number series “Personnel number” is inactive.".to_string(),
            ))
        })?;
    Ok(Json(NextPersonnelNo { personnel_no }))
}

/// Adds an employee.
#[utoipa::path(
    post,
    path = "/api/v1/people",
    tag = "personal",
    request_body = EmployeeInput,
    responses(
        (status = 201, body = Employee),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Extension(scope): Extension<DepartmentScope>,
    Json(input): Json<EmployeeInput>,
) -> Result<(StatusCode, Json<Employee>), ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    scope.check_personal(input.department.as_deref())?;
    let input = if may_see_personal_data(&user) {
        input
    } else {
        input.without_personal_data()
    };
    let employee = employees(&state).create(&input).await?;
    Ok((StatusCode::CREATED, Json(employee)))
}

/// Replaces an employee.
#[utoipa::path(
    put,
    path = "/api/v1/people/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Employee id")),
    request_body = EmployeeInput,
    responses(
        (status = 200, body = Employee),
        (status = 404, description = "Unknown", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn replace(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Extension(scope): Extension<DepartmentScope>,
    Path(id): Path<i64>,
    Json(input): Json<EmployeeInput>,
) -> Result<Json<Employee>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    scope.check(&employees(&state).get(id).await?)?;
    scope.check_personal(input.department.as_deref())?;
    let input = if may_see_personal_data(&user) {
        input
    } else {
        input.without_personal_data()
    };
    Ok(Json(employees(&state).update(id, &input).await?))
}

/// Names the user account behind an employee — or takes it away.
///
/// The user administration sends this after it created or changed an account: the
/// employee list marks a name with a check as soon as the reference stands.
#[utoipa::path(
    put,
    path = "/api/v1/people/{id}/user",
    tag = "personal",
    params(("id" = i64, Path, description = "Employee id")),
    request_body = EmployeeUserInput,
    responses(
        (status = 200, body = Employee),
        (status = 404, description = "Unknown employee", body = where_api::error::Problem)
    )
)]
pub async fn set_employee_user(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Path(id): Path<i64>,
    Json(input): Json<EmployeeUserInput>,
) -> Result<Json<Employee>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    scope.check(&employees(&state).get(id).await?)?;
    Ok(Json(employees(&state).set_user(id, input.user_id).await?))
}

/// Removes an employee.
#[utoipa::path(
    delete,
    path = "/api/v1/people/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Employee id")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown", body = where_api::error::Problem)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    scope.check(&employees(&state).get(id).await?)?;
    employees(&state).delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

// ---------------------------------------------------- Departments and positions

/// All entries of one list, sorted by name.
#[utoipa::path(
    get,
    path = "/api/v1/people/lists/{kind}",
    tag = "personal",
    params(("kind" = String, Path, description = "`departments`, `subdepartments`, `positions` or `locations`")),
    responses(
        (status = 200, body = Vec<ListEntry>),
        (status = 404, description = "Unknown list", body = where_api::error::Problem)
    )
)]
pub async fn list_entries(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<Vec<ListEntry>>, ApiError> {
    Ok(Json(people_list(&state, list_kind(&kind)?).list().await?))
}

/// Adds an entry.
#[utoipa::path(
    post,
    path = "/api/v1/people/lists/{kind}",
    tag = "personal",
    params(("kind" = String, Path, description = "`departments`, `subdepartments`, `positions` or `locations`")),
    request_body = ListEntryInput,
    responses(
        (status = 201, body = ListEntry),
        (status = 404, description = "Unknown list", body = where_api::error::Problem),
        (status = 409, description = "Name already exists", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn create_entry(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Json(input): Json<ListEntryInput>,
) -> Result<(StatusCode, Json<ListEntry>), ApiError> {
    let entry = people_list(&state, list_kind(&kind)?)
        .create(&input)
        .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

/// Renames an entry; employees that carry the old name follow in the same
/// transaction.
#[utoipa::path(
    put,
    path = "/api/v1/people/lists/{kind}/{id}",
    tag = "personal",
    params(
        ("kind" = String, Path, description = "`departments`, `subdepartments`, `positions` or `locations`"),
        ("id" = i64, Path, description = "Entry id")
    ),
    request_body = ListEntryInput,
    responses(
        (status = 200, body = ListEntry),
        (status = 404, description = "Unknown", body = where_api::error::Problem),
        (status = 409, description = "Name already exists", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn rename_entry(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, i64)>,
    Json(input): Json<ListEntryInput>,
) -> Result<Json<ListEntry>, ApiError> {
    Ok(Json(
        people_list(&state, list_kind(&kind)?)
            .rename(id, &input)
            .await?,
    ))
}

/// Binds a location to the sub-department that works there, or a sub-department to its
/// department.
#[utoipa::path(
    put,
    path = "/api/v1/people/lists/{kind}/{id}/department",
    tag = "personal",
    params(
        ("kind" = String, Path, description = "`locations` or `subdepartments`"),
        ("id" = i64, Path, description = "Entry id")
    ),
    request_body = LocationDepartmentInput,
    responses(
        (status = 200, body = ListEntry),
        (status = 404, description = "Unknown", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn set_list_department(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, i64)>,
    Json(input): Json<LocationDepartmentInput>,
) -> Result<Json<ListEntry>, ApiError> {
    let department = input.clean()?;
    Ok(Json(
        people_list(&state, list_kind(&kind)?)
            .set_department(id, department.as_deref())
            .await?,
    ))
}

/// Says whether a location is staffed with a foreman.
#[utoipa::path(
    put,
    path = "/api/v1/people/lists/locations/{id}/foreman",
    tag = "personal",
    params(("id" = i64, Path, description = "Location id")),
    request_body = LocationForemanInput,
    responses(
        (status = 200, body = ListEntry),
        (status = 404, description = "Unknown", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn set_location_foreman(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<LocationForemanInput>,
) -> Result<Json<ListEntry>, ApiError> {
    Ok(Json(
        people_list(&state, ListKind::Locations)
            .set_needs_foreman(id, input.needs_foreman)
            .await?,
    ))
}

/// The tint of a list entry; an empty colour removes it.
#[utoipa::path(
    put,
    path = "/api/v1/people/lists/{kind}/{id}/color",
    tag = "personal",
    params(
        ("kind" = String, Path, description = "`departments`, `subdepartments`, `positions` or `locations`"),
        ("id" = i64, Path, description = "Entry id")
    ),
    request_body = ListColorInput,
    responses(
        (status = 200, body = ListEntry),
        (status = 404, description = "Unknown", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn set_list_color(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, i64)>,
    Json(input): Json<ListColorInput>,
) -> Result<Json<ListEntry>, ApiError> {
    let color = input.clean()?;
    Ok(Json(
        people_list(&state, list_kind(&kind)?)
            .set_color(id, color.as_deref())
            .await?,
    ))
}

/// What writing an order did.
#[derive(Debug, Clone, serde::Serialize, Deserialize, ToSchema)]
pub struct OrderReport {
    /// Rows that took a new place.
    pub written: usize,
}

/// Writes the order of the employees on the schedule board; the ids come top to bottom.
/// What the board writes is the visible crew, so the answer names how many rows.
#[utoipa::path(
    put,
    path = "/api/v1/people/board-order",
    tag = "personal",
    request_body = ListOrderInput,
    responses(
        (status = 200, body = OrderReport),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn board_order(
    State(state): State<AppState>,
    Json(input): Json<ListOrderInput>,
) -> Result<Json<OrderReport>, ApiError> {
    let written = employees(&state)
        .set_board_order(&input.clean_ids())
        .await?;
    Ok(Json(OrderReport { written }))
}

/// Removes an entry.
#[utoipa::path(
    delete,
    path = "/api/v1/people/lists/{kind}/{id}",
    tag = "personal",
    params(
        ("kind" = String, Path, description = "`departments`, `subdepartments`, `positions` or `locations`"),
        ("id" = i64, Path, description = "Entry id")
    ),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown", body = where_api::error::Problem)
    )
)]
pub async fn delete_entry(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, i64)>,
) -> Result<Json<DeletedResponse>, ApiError> {
    people_list(&state, list_kind(&kind)?).delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Writes a new order; the ids come top to bottom.
#[utoipa::path(
    put,
    path = "/api/v1/people/lists/{kind}/order",
    tag = "personal",
    params(("kind" = String, Path, description = "`departments`, `subdepartments`, `positions` or `locations`")),
    request_body = ListOrderInput,
    responses(
        (status = 200, body = Vec<ListEntry>),
        (status = 404, description = "Unknown list or id", body = where_api::error::Problem)
    )
)]
pub async fn reorder_entries(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Json(input): Json<ListOrderInput>,
) -> Result<Json<Vec<ListEntry>>, ApiError> {
    let entries = people_list(&state, list_kind(&kind)?)
        .reorder(&input.clean_ids())
        .await?;
    Ok(Json(entries))
}

// ---------------------------------------------------- Master data as exchange files

/// The switches of a master-data import.
#[derive(Debug, Clone, Default, Deserialize, utoipa::IntoParams)]
pub struct MasterQuery {
    /// Overwrite the records the file finds again; without it only new ones are written.
    #[serde(default)]
    pub replace: bool,
}

/// Reads a master-data file (`lists`, `employees`, `shifts`) and says what it carries:
/// how many records it would create, change or leave alone, and everything it skipped.
/// Nothing is written.
#[utoipa::path(
    post,
    path = "/api/v1/people/masterdata/{area}/scan",
    tag = "personal",
    params(
        ("area" = String, Path, description = "`lists`, `employees` or `shifts`"),
        MasterQuery
    ),
    request_body(content = String, description = "The file itself", content_type = "application/octet-stream"),
    responses(
        (status = 200, body = MasterReport),
        (status = 422, description = "Not a readable master-data file", body = where_api::error::Problem)
    )
)]
pub async fn masterdata_scan(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Extension(scope): Extension<DepartmentScope>,
    Path(area): Path<String>,
    Query(query): Query<MasterQuery>,
    body: Bytes,
) -> Result<Json<MasterReport>, ApiError> {
    let area = master_area(&area)?;
    require_area_right(area, &user)?;
    require_file_scope(&scope, area)?;
    let report = masterdata::import(&state.pool, area, &body, query.replace, false).await?;
    Ok(Json(report))
}

/// Writes master data from a file. An import never deletes: it creates what the file
/// carries and, with `replace`, overwrites the records it finds again.
#[utoipa::path(
    post,
    path = "/api/v1/people/masterdata/{area}",
    tag = "personal",
    params(
        ("area" = String, Path, description = "`lists`, `employees` or `shifts`"),
        MasterQuery
    ),
    request_body(content = String, description = "The file itself", content_type = "application/octet-stream"),
    responses(
        (status = 200, body = MasterReport),
        (status = 403, description = "The personal data of employees may not be written", body = where_api::error::Problem),
        (status = 422, description = "Not a readable master-data file", body = where_api::error::Problem)
    )
)]
pub async fn masterdata_import(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Extension(scope): Extension<DepartmentScope>,
    Path(area): Path<String>,
    Query(query): Query<MasterQuery>,
    body: Bytes,
) -> Result<Json<MasterReport>, ApiError> {
    let area = master_area(&area)?;
    require_area_right(area, &user)?;
    require_file_scope(&scope, area)?;
    // The employee sheet carries birth dates, addresses and the social security number.
    if area == MasterArea::Employees && !may_see_personal_data(&user) {
        return Err(ApiError::from(DomainError::Forbidden(format!(
            "Für den Import von Personal fehlt das Recht „{PERSONAL_DATA_RIGHT}“."
        ))));
    }
    let report = masterdata::import(&state.pool, area, &body, query.replace, true).await?;
    Ok(Json(report))
}

/// The plan file of a month carries the cells of every employee, so a group that may
/// only work in some departments does not exchange it.
fn require_plan_scope(scope: &DepartmentScope) -> Result<(), ApiError> {
    if scope.dispo.is_none() {
        return Ok(());
    }
    Err(ApiError::from(DomainError::Forbidden(
        "Der Austausch des Dienstplans umfasst alle Abteilungen und gehört zur Administration."
            .to_string(),
    )))
}

/// The personnel file carries every employee, so a group that may only work in some
/// departments does not exchange it — neither reading nor writing.
fn require_file_scope(scope: &DepartmentScope, area: MasterArea) -> Result<(), ApiError> {
    if area != MasterArea::Employees || scope.personal.is_none() {
        return Ok(());
    }
    Err(ApiError::from(DomainError::Forbidden(
        "Der Personalaustausch umfasst alle Abteilungen und gehört zur Administration.".to_string(),
    )))
}

/// The lists and the shifts are maintained by the administration in their masks, so
/// their files are administration work as well; the employees follow the personnel
/// rights, which the route already asked for.
fn require_area_right(area: MasterArea, user: &SessionUser) -> Result<(), ApiError> {
    if area == MasterArea::Employees || user.has(where_domain::ADMIN_RIGHT) {
        return Ok(());
    }
    Err(ApiError::from(DomainError::Forbidden(
        "Abteilungen, Positionen, Standorte und Schichten pflegt die Administration.".to_string(),
    )))
}

/// The same for a stored shift, by its id.
async fn check_stored_shift(
    state: &AppState,
    scope: &PeopleScope,
    id: i64,
) -> Result<(), ApiError> {
    let list = shifts(state).list().await?;
    let Some(shift) = list.iter().find(|shift| shift.id == id) else {
        return Ok(());
    };
    scope.check_dispo(shift.department.as_deref())
}

/// Refuses a shift that belongs to a department the session may not dispose.
async fn check_shift_scope(
    state: &AppState,
    scope: &PeopleScope,
    shift_id: Option<i64>,
) -> Result<(), ApiError> {
    match shift_id {
        Some(id) => check_stored_shift(state, scope, id).await,
        None => Ok(()),
    }
}

/// The area of the path, or a `422` naming the three there are.
fn master_area(key: &str) -> Result<MasterArea, ApiError> {
    MasterArea::from_key(key).ok_or_else(|| {
        ApiError::from(DomainError::Validation(format!(
            "Unbekannte Stammdaten „{key}“ — erlaubt sind lists, employees und shifts."
        )))
    })
}

// ------------------------------------------------------------------------ Shifts

/// All shifts of the schedule planning, in their maintained order.
#[utoipa::path(
    get,
    path = "/api/v1/people/shifts",
    tag = "personal",
    responses((status = 200, body = Vec<Shift>))
)]
pub async fn list_shifts(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
) -> Result<Json<Vec<Shift>>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let mut list = shifts(&state).list().await?;
    // A shift without a department belongs to everybody; one with a department only
    // to those who may dispose it.
    list.retain(|shift| {
        shift
            .department
            .as_deref()
            .map(|department| scope.allows_dispo(department))
            .unwrap_or(true)
    });
    Ok(Json(list))
}

/// Adds a shift; it is appended at the end.
#[utoipa::path(
    post,
    path = "/api/v1/people/shifts",
    tag = "personal",
    request_body = ShiftInput,
    responses(
        (status = 201, body = Shift),
        (status = 409, description = "Name already exists", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn create_shift(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Json(input): Json<ShiftInput>,
) -> Result<(StatusCode, Json<Shift>), ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let einsatz = input.clean()?;
    scope.check_dispo(einsatz.department.as_deref())?;
    let shift = shifts(&state).create(&einsatz).await?;
    Ok((StatusCode::CREATED, Json(shift)))
}

/// Replaces one shift.
#[utoipa::path(
    put,
    path = "/api/v1/people/shifts/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Shift id")),
    request_body = ShiftInput,
    responses(
        (status = 200, body = Shift),
        (status = 404, description = "Unknown", body = where_api::error::Problem),
        (status = 409, description = "Name already exists", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn replace_shift(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Path(id): Path<i64>,
    Json(input): Json<ShiftInput>,
) -> Result<Json<Shift>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let einsatz = input.clean()?;
    check_stored_shift(&state, &scope, id).await?;
    scope.check_dispo(einsatz.department.as_deref())?;
    Ok(Json(shifts(&state).update(id, &einsatz).await?))
}

/// Removes one shift.
#[utoipa::path(
    delete,
    path = "/api/v1/people/shifts/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Shift id")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown", body = where_api::error::Problem)
    )
)]
pub async fn delete_shift(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    check_stored_shift(&state, &scope, id).await?;
    shifts(&state).delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Writes a new order; the ids come top to bottom.
#[utoipa::path(
    put,
    path = "/api/v1/people/shifts/order",
    tag = "personal",
    request_body = ListOrderInput,
    responses(
        (status = 200, body = Vec<Shift>),
        (status = 404, description = "Unknown id", body = where_api::error::Problem)
    )
)]
pub async fn reorder_shifts(
    State(state): State<AppState>,
    Json(input): Json<ListOrderInput>,
) -> Result<Json<Vec<Shift>>, ApiError> {
    Ok(Json(shifts(&state).reorder(&input.clean_ids()).await?))
}

/// Replaces the positions a shift does not have; the list is taken as given.
#[utoipa::path(
    put,
    path = "/api/v1/people/shifts/{id}/exclusions",
    tag = "personal",
    params(("id" = i64, Path, description = "Shift id")),
    request_body = ShiftExclusionsInput,
    responses(
        (status = 200, body = Shift),
        (status = 404, description = "Unknown shift", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn shift_exclusions(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Path(id): Path<i64>,
    Json(input): Json<ShiftExclusionsInput>,
) -> Result<Json<Shift>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    check_stored_shift(&state, &scope, id).await?;
    Ok(Json(
        shifts(&state).set_exclusions(id, &input.clean()?).await?,
    ))
}

// ------------------------------------------------------------------------- Rules

/// The planning rules; without a stored row the legal defaults apply.
#[utoipa::path(
    get,
    path = "/api/v1/people/planning",
    tag = "personal",
    responses((status = 200, body = PlanningRules))
)]
pub async fn get_planning(State(state): State<AppState>) -> Result<Json<PlanningRules>, ApiError> {
    Ok(Json(planning(&state).get().await?))
}

/// Replaces the planning rules.
#[utoipa::path(
    put,
    path = "/api/v1/people/planning",
    tag = "personal",
    request_body = PlanningRules,
    responses(
        (status = 200, body = PlanningRules),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn save_planning(
    State(state): State<AppState>,
    Json(input): Json<PlanningRules>,
) -> Result<Json<PlanningRules>, ApiError> {
    Ok(Json(planning(&state).save(&input).await?))
}

// --------------------------------------------------------------------- Schedule plan

/// The schedule entries of a period.
#[utoipa::path(
    get,
    path = "/api/v1/people/schedule",
    tag = "personal",
    params(ScheduleQuery),
    responses(
        (status = 200, body = Vec<ScheduleEntry>),
        (status = 422, description = "Invalid range", body = where_api::error::Problem)
    )
)]
pub async fn schedule_range(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Query(query): Query<ScheduleQuery>,
) -> Result<Json<Vec<ScheduleEntry>>, ApiError> {
    query.validate()?;
    let scope = PeopleScope::load(&state, &scope).await?;
    let allowed = scope.employee_ids(&state, false).await?;
    let mut entries = schedule(&state).range(query.from, query.to).await?;
    if let Some(ids) = &allowed {
        entries.retain(|entry| ids.contains(&entry.employee_id));
    }
    Ok(Json(entries))
}

/// Checks the written plan of a period against the rules of the planning.
///
/// The days around the period are read as well — the rest period, the weekly
/// hours and the days in a row need them — but only the cells inside the period
/// are reported.
#[utoipa::path(
    get,
    path = "/api/v1/people/schedule/audit",
    tag = "personal",
    params(ScheduleQuery),
    responses(
        (status = 200, body = ScheduleAudit),
        (status = 422, description = "Invalid range", body = where_api::error::Problem)
    )
)]
pub async fn schedule_audit(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Query(query): Query<ScheduleQuery>,
) -> Result<Json<ScheduleAudit>, ApiError> {
    query.validate()?;
    let scope = PeopleScope::load(&state, &scope).await?;
    let rules = planning(&state).get().await?;
    let shifts = shifts(&state).list().await?;
    let people: Vec<AuditPerson> = scope
        .keep(employees(&state).list(None).await?)
        .into_iter()
        .filter_map(|employee| {
            employee.id.map(|id| AuditPerson {
                id,
                apprentice: employee.is_apprentice,
                position: employee.position.unwrap_or_default(),
            })
        })
        .collect();
    // A run of days may reach far back; a month around the period is enough.
    let around_from = query.from - chrono::Days::new(31);
    let around_to = query.to + chrono::Days::new(31);
    let known: HashSet<i64> = people.iter().map(|person| person.id).collect();
    let cells: Vec<AuditCell> = schedule(&state)
        .range(around_from, around_to)
        .await?
        .into_iter()
        .filter(|entry| known.contains(&entry.employee_id))
        .map(|entry| AuditCell {
            employee_id: entry.employee_id,
            day: entry.day,
            kind: entry.kind,
            shift_id: entry.shift_id,
        })
        .collect();
    let holidays: HashSet<NaiveDate> = holiday_calendar(&state)
        .range(around_from, around_to)
        .await?
        .holidays
        .into_iter()
        .map(|holiday| holiday.day)
        .collect();
    Ok(Json(schedule_audit::audit(
        &rules, &shifts, &people, &cells, &holidays, query.from, query.to,
    )))
}

/// Sets the cell of one employee on one day: a shift, `sick` or `vacation`;
/// without `kind` the cell is cleared.
#[utoipa::path(
    put,
    path = "/api/v1/people/schedule",
    tag = "personal",
    request_body = ScheduleInput,
    responses(
        (status = 200, description = "Set; empty when the cell was cleared"),
        (status = 404, description = "Employee or shift unknown", body = where_api::error::Problem),
        (status = 409, description = "The shift refuses the employee or the day", body = where_api::error::Problem),
        (status = 422, description = "Unknown kind", body = where_api::error::Problem)
    )
)]
pub async fn schedule_set(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Json(input): Json<ScheduleInput>,
) -> Result<Json<Option<ScheduleEntry>>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let allowed = scope.employee_ids(&state, false).await?;
    crate::scope::check_id(allowed.as_ref(), input.employee_id)?;
    check_shift_scope(&state, &scope, input.shift_id).await?;
    let rules = planning(&state).get().await?;
    match input.clean()? {
        Some(entry) => Ok(Json(Some(schedule(&state).set(&entry, &rules).await?))),
        None => {
            schedule(&state).clear(input.employee_id, input.day).await?;
            Ok(Json(None))
        }
    }
}

/// Writes the cell of one employee onto several days in a row: a shift, `sick`
/// or `vacation`. The first day counts, so `days: 5` fills five days. A shift
/// only reaches as far as the planning rules allow; the answer names the day and
/// the rule that stopped it.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/series",
    tag = "personal",
    request_body = ScheduleSeriesInput,
    responses(
        (status = 200, description = "The written days and the rule that stopped the series", body = ScheduleSeriesResult),
        (status = 404, description = "Shift unknown", body = where_api::error::Problem),
        (status = 422, description = "Invalid series or unknown employee", body = where_api::error::Problem)
    )
)]
pub async fn schedule_series(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Json(input): Json<ScheduleSeriesInput>,
) -> Result<Json<ScheduleSeriesResult>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let allowed = scope.employee_ids(&state, false).await?;
    crate::scope::check_id(allowed.as_ref(), input.employee_id)?;
    check_shift_scope(&state, &scope, input.shift_id).await?;
    let series = input.clean()?;
    let rules = planning(&state).get().await?;
    Ok(Json(schedule(&state).set_series(&series, &rules).await?))
}

/// Removes the shift cells of a month the reset of the settings throws away: the
/// shifts, the free days (`FT`, `SoFT`), the free time windows and the vacations.
/// Sick stays — it is a record of the personnel file, not a part of the plan.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/reset",
    tag = "personal",
    params(MonthQuery),
    responses(
        (status = 200, description = "The shifts of the month were removed", body = ResetReport),
        (status = 422, description = "Invalid month", body = where_api::error::Problem)
    )
)]
pub async fn schedule_reset(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Query(query): Query<MonthQuery>,
) -> Result<Json<ResetReport>, ApiError> {
    let (from, to) = query.range()?;
    let scope = PeopleScope::load(&state, &scope).await?;
    let cleared = match scope.departments(false) {
        None => schedule(&state).clear_shifts(from, to).await?,
        Some(names) => schedule(&state).clear_shifts_of(from, to, &names).await?,
    };
    Ok(Json(ResetReport { cleared }))
}

/// What one import asks for.
#[derive(Debug, Deserialize, ToSchema)]
pub struct HolidayImportInput {
    /// Year to import; without it the current year is taken.
    #[serde(default)]
    pub year: Option<i32>,
}

/// The holidays and school holidays of a period.
#[utoipa::path(
    get,
    path = "/api/v1/people/holidays",
    tag = "personal",
    params(ScheduleQuery),
    responses(
        (status = 200, body = HolidayCalendar),
        (status = 422, description = "Invalid range", body = where_api::error::Problem)
    )
)]
pub async fn holiday_range(
    State(state): State<AppState>,
    Query(query): Query<ScheduleQuery>,
) -> Result<Json<HolidayCalendar>, ApiError> {
    query.validate()?;
    Ok(Json(
        holiday_calendar(&state).range(query.from, query.to).await?,
    ))
}

/// How many holidays and school holidays are stored.
#[utoipa::path(
    get,
    path = "/api/v1/people/holidays/summary",
    tag = "personal",
    responses((status = 200, body = HolidayCounts))
)]
pub async fn holiday_counts(
    State(state): State<AppState>,
) -> Result<Json<HolidayCounts>, ApiError> {
    Ok(Json(holiday_calendar(&state).counts().await?))
}

/// Imports the holidays of one year from the source the settings name.
///
/// The settings decide which lists are read and for which federal state; the
/// addresses themselves are fixed (`openholidaysapi.org`), so nothing has to be
/// typed in. The rows of the year are replaced by what the run brought; a list
/// whose source failed leaves the stored rows alone and is named in `errors`.
#[utoipa::path(
    post,
    path = "/api/v1/people/holidays/import",
    tag = "personal",
    request_body = HolidayImportInput,
    responses(
        (status = 200, body = HolidayImportReport),
        (status = 422, description = "Invalid year or missing federal state", body = where_api::error::Problem),
        (status = 502, description = "The source could not be read", body = where_api::error::Problem)
    )
)]
pub async fn holiday_import(
    State(state): State<AppState>,
    Json(input): Json<HolidayImportInput>,
) -> Result<Json<HolidayImportReport>, ApiError> {
    let rules = planning(&state).get().await?;
    let year = input.year.unwrap_or_else(|| Utc::now().year());
    if !(2000..=2100).contains(&year) {
        return Err(ApiError::from(DomainError::Validation(format!(
            "The year {year} lies outside 2000 to 2100."
        ))));
    }

    let mut report = HolidayImportReport {
        year,
        state: rules.holiday_state.clone(),
        national_holidays: 0,
        regional_holidays: 0,
        school_holidays: 0,
        errors: Vec::new(),
    };

    let mut public: Option<Vec<Holiday>> = None;
    if rules.import_public_holidays || rules.import_regional_holidays {
        let source = HolidaySource::new()?;
        match source.public(year, rules.holiday_state.as_deref()).await {
            Ok(holidays) => {
                let chosen: Vec<Holiday> = holidays
                    .into_iter()
                    .filter(|holiday| match holiday.scope.as_str() {
                        SCOPE_NATIONAL => rules.import_public_holidays,
                        SCOPE_REGIONAL => rules.import_regional_holidays,
                        _ => false,
                    })
                    .collect();
                report.national_holidays = chosen
                    .iter()
                    .filter(|holiday| holiday.scope == SCOPE_NATIONAL)
                    .count();
                report.regional_holidays = chosen.len() - report.national_holidays;
                public = Some(chosen);
            }
            Err(error) => report.errors.push(error.to_string()),
        }
    }

    let mut school: Option<Vec<SchoolHoliday>> = None;
    if rules.import_school_holidays {
        let Some(code) = rules.holiday_state.clone() else {
            return Err(ApiError::from(DomainError::Validation(
                "School holidays need a federal state.".to_string(),
            )));
        };
        let source = HolidaySource::new()?;
        match source.school(year, &code).await {
            Ok(holidays) => {
                report.school_holidays = holidays.len();
                school = Some(holidays);
            }
            Err(error) => report.errors.push(error.to_string()),
        }
    }

    if public.is_none() && school.is_none() {
        if report.errors.is_empty() {
            return Err(ApiError::from(DomainError::Validation(
                "No holiday import is switched on in the settings.".to_string(),
            )));
        }
        return Err(ApiError::from(DomainError::Upstream(
            report.errors.join(" "),
        )));
    }

    holiday_calendar(&state)
        .replace_year(year, public.as_deref(), school.as_deref())
        .await?;
    Ok(Json(report))
}

/// Adds a row the board types itself — a location whose name is free text.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/rows",
    tag = "personal",
    responses((status = 201, body = ListEntry))
)]
pub async fn create_schedule_row(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<ListEntry>), ApiError> {
    let entry = people_list(&state, ListKind::Locations)
        .create_text_row(DEFAULT_TEXT_ROW)
        .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

/// Renames a typed row of the board.
#[utoipa::path(
    put,
    path = "/api/v1/people/schedule/rows/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Id of the row")),
    request_body = ListEntryInput,
    responses(
        (status = 200, body = ListEntry),
        (status = 404, description = "Row unknown or a house", body = where_api::error::Problem),
        (status = 422, description = "Name empty or longer than 100 characters", body = where_api::error::Problem)
    )
)]
pub async fn rename_schedule_row(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<ListEntryInput>,
) -> Result<Json<ListEntry>, ApiError> {
    let name = input.clean()?;
    Ok(Json(
        people_list(&state, ListKind::Locations)
            .rename_text_row(id, &name)
            .await?,
    ))
}

/// Removes a typed row of the board with its events; a house stays untouched.
#[utoipa::path(
    delete,
    path = "/api/v1/people/schedule/rows/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Id of the row")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Row unknown or a house", body = where_api::error::Problem)
    )
)]
pub async fn delete_schedule_row(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    people_list(&state, ListKind::Locations)
        .delete_text_row(id)
        .await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

// --------------------------------------------------------------- PeM import

/// The locations and events of a PeM export, without writing anything.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/dispo/scan",
    tag = "personal",
    params(DispoQuery),
    request_body(content = String, description = "The xlsx file itself", content_type = "application/octet-stream"),
    responses(
        (status = 200, body = DispoScan),
        (status = 422, description = "Not a readable PeM export", body = where_api::error::Problem)
    )
)]
pub async fn dispo_scan(
    State(state): State<AppState>,
    Query(query): Query<DispoQuery>,
    body: Bytes,
) -> Result<Json<DispoScan>, ApiError> {
    let (from, to) = query.month_range()?;
    let stored = flags(&state).codes().await?;
    let codes = crate::flags::vocabulary(&stored);
    let mut scan = dispo::scan(&body, query.month, query.year, query.only_with_time, &codes)?;
    let fingerprint = dispo::fingerprint(&scan);
    let repository = dispo_entries(&state);
    scan.existing_entries = repository.count_range(from, to).await?;
    scan.already_imported =
        scan.existing_entries > 0 && repository.fingerprint_seen(from, &fingerprint).await?;
    Ok(Json(scan))
}

/// Writes the locations and events of a PeM export.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/dispo/import",
    tag = "personal",
    params(DispoQuery),
    request_body(content = String, description = "The xlsx file itself", content_type = "application/octet-stream"),
    responses(
        (status = 200, body = DispoImportReport),
        (status = 409, description = "This disposition was already imported for the month", body = where_api::error::Problem),
        (status = 422, description = "Not a readable PeM export", body = where_api::error::Problem)
    )
)]
pub async fn dispo_import(
    State(state): State<AppState>,
    Query(query): Query<DispoQuery>,
    body: Bytes,
) -> Result<Json<DispoImportReport>, ApiError> {
    let (from, to) = query.month_range()?;
    let stored = flags(&state).codes().await?;
    let codes = crate::flags::vocabulary(&stored);
    let scan = dispo::scan(&body, query.month, query.year, query.only_with_time, &codes)?;
    let fingerprint = dispo::fingerprint(&scan);
    let used = dispo::used_flags(&scan);
    let presets = flags(&state).create_presets_of(&used).await?;
    let report = dispo_entries(&state)
        .import(&scan, from, to, query.replace, &fingerprint)
        .await?;
    flags(&state).staff_presets_of(&used).await?;
    Ok(Json(DispoImportReport {
        flags_created: presets.created,
        ..report
    }))
}

/// Removes every entry of the disposition of a month.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/dispo/reset",
    tag = "personal",
    params(MonthQuery),
    responses(
        (status = 200, description = "The disposition of the month was removed", body = ResetReport),
        (status = 422, description = "Invalid month", body = where_api::error::Problem)
    )
)]
pub async fn dispo_reset(
    State(state): State<AppState>,
    Query(query): Query<MonthQuery>,
) -> Result<Json<ResetReport>, ApiError> {
    let (from, to) = query.range()?;
    let cleared = dispo_entries(&state).clear_range(from, to).await?;
    Ok(Json(ResetReport { cleared }))
}

/// Reads an exchange file without writing anything: what it carries, what stays out and
/// what the month already holds. `kind` is `dispo` or `plan`.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/exchange/{kind}/scan",
    tag = "personal",
    params(
        ("kind" = String, Path, description = "`dispo` or `plan`"),
        ExchangeQuery
    ),
    request_body(content = String, description = "The file itself", content_type = "application/octet-stream"),
    responses(
        (status = 200, body = ExchangeScan),
        (status = 422, description = "Not a readable exchange file", body = where_api::error::Problem)
    )
)]
pub async fn exchange_scan(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Path(kind): Path<String>,
    Query(query): Query<ExchangeQuery>,
    body: Bytes,
) -> Result<Json<ExchangeScan>, ApiError> {
    let (from, to) = query.range()?;
    let scan = match kind.as_str() {
        "dispo" => {
            let dispo = exchange::read_dispo(&body, from, to, query.only_with_time)?;
            let fingerprint = dispo::fingerprint(&dispo);
            let repository = dispo_entries(&state);
            let mut preview = exchange::preview_dispo(&dispo);
            preview.existing_entries = repository.count_range(from, to).await?;
            preview.already_imported = preview.existing_entries > 0
                && repository.fingerprint_seen(from, &fingerprint).await?;
            preview
        }
        "plan" => {
            require_plan_scope(&scope)?;
            let sheet = exchange::read_plan(&body)?;
            let mut preview = exchange::preview_plan(&sheet, from, to);
            preview.existing_entries = schedule(&state).range(from, to).await?.len();
            preview
        }
        other => {
            return Err(ApiError::from(where_domain::DomainError::Validation(
                format!("Unknown exchange kind “{other}”."),
            )));
        }
    };
    Ok(Json(scan))
}

/// Writes the disposition of a month from an exchange file: the sheet the module
/// hands out (`Standort; Datum; Beginn; Titel; Kennzeichen`), as CSV or XLSX. With
/// `replace` the month is emptied first; locations the file names are created.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/exchange/dispo",
    tag = "personal",
    params(ExchangeQuery),
    request_body(content = String, description = "The file itself", content_type = "application/octet-stream"),
    responses(
        (status = 200, body = ExchangeReport),
        (status = 409, description = "This disposition was already imported for the month", body = where_api::error::Problem),
        (status = 422, description = "Not a readable exchange file", body = where_api::error::Problem)
    )
)]
pub async fn exchange_dispo(
    State(state): State<AppState>,
    Query(query): Query<ExchangeQuery>,
    body: Bytes,
) -> Result<Json<ExchangeReport>, ApiError> {
    let (from, to) = query.range()?;
    let scan = exchange::read_dispo(&body, from, to, query.only_with_time)?;
    let rows = scan.locations.iter().map(|ort| ort.events.len()).sum();
    let fingerprint = dispo::fingerprint(&scan);
    let used = dispo::used_flags(&scan);
    flags(&state).create_presets_of(&used).await?;
    let report = dispo_entries(&state)
        .import(&scan, from, to, query.replace, &fingerprint)
        .await?;
    flags(&state).staff_presets_of(&used).await?;
    Ok(Json(ExchangeReport {
        rows,
        written: report.entries_created,
        warnings: scan.warnings.clone(),
    }))
}

/// Writes the plan of a month from an exchange file: the sheet the module hands out
/// (`Personalnummer; Name; Datum; Art; Schicht; Beginn; Ende; Pause; Notiz`), as CSV
/// or XLSX. With `replace` the month is emptied first; personnel numbers and shifts the
/// file names have to exist, everything else is named in `warnings`.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/exchange/plan",
    tag = "personal",
    params(ExchangeQuery),
    request_body(content = String, description = "The file itself", content_type = "application/octet-stream"),
    responses(
        (status = 200, body = ExchangeReport),
        (status = 422, description = "Not a readable exchange file", body = where_api::error::Problem)
    )
)]
pub async fn exchange_plan(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Query(query): Query<ExchangeQuery>,
    body: Bytes,
) -> Result<Json<ExchangeReport>, ApiError> {
    require_plan_scope(&scope)?;
    let (from, to) = query.range()?;
    let rows = exchange::read_plan(&body)?;
    let people = employees(&state).list(None).await?;
    let mut employee_ids: HashMap<String, i64> = HashMap::new();
    for person in &people {
        if let (Some(number), Some(id)) = (&person.personnel_no, person.id) {
            employee_ids.insert(number.trim().to_lowercase(), id);
        }
    }
    let mut shift_ids: HashMap<String, i64> = HashMap::new();
    for shift in shifts(&state).list().await? {
        if let Some(short) = &shift.short_name {
            shift_ids.insert(short.trim().to_lowercase(), shift.id);
        }
        shift_ids
            .entry(shift.name.trim().to_lowercase())
            .or_insert(shift.id);
    }
    let mut warnings: Vec<String> = rows.warnings.clone();
    let mut cells: Vec<RestoreCell> = Vec::new();
    let mut moved = 0usize;
    let mut first_foreign: Option<chrono::NaiveDate> = None;
    for row in &rows.rows {
        let Some(day) = exchange::day_in_month(row.day, from, to) else {
            warnings.push(format!(
                "Zeile {}: den {}. {} gibt es nicht.",
                row.line,
                row.day.day(),
                exchange::month_and_year(from)
            ));
            continue;
        };
        if day != row.day {
            moved += 1;
            first_foreign.get_or_insert(row.day);
        }
        let Some(employee_id) = employee_ids.get(&row.personnel_no.to_lowercase()).copied() else {
            warnings.push(format!(
                "Zeile {}: Personalnummer „{}“ ist unbekannt.",
                row.line, row.personnel_no
            ));
            continue;
        };
        let (kind, shift_id) = match row.art {
            exchange::PlanArt::Shift => {
                let name = row.shift.as_deref().unwrap_or_default().to_lowercase();
                let Some(shift_id) = shift_ids.get(&name).copied() else {
                    warnings.push(format!(
                        "Zeile {}: Schicht „{}“ ist unbekannt.",
                        row.line,
                        row.shift.as_deref().unwrap_or_default()
                    ));
                    continue;
                };
                (KIND_SHIFT.to_string(), Some(shift_id))
            }
            exchange::PlanArt::Free => {
                if row.starts_at.is_none() || row.ends_at.is_none() {
                    warnings.push(format!(
                        "Zeile {}: dem Zeitfenster fehlt Beginn oder Ende.",
                        row.line
                    ));
                    continue;
                }
                (KIND_FREE.to_string(), None)
            }
            exchange::PlanArt::Sick => (KIND_SICK.to_string(), None),
            exchange::PlanArt::Vacation => (KIND_VACATION.to_string(), None),
        };
        let free = kind == KIND_FREE;
        let note = row.note.as_ref().map(|note| {
            if note.chars().count() > MAX_NOTE_CHARS {
                warnings.push(format!(
                    "Zeile {}: die Notiz wurde auf {MAX_NOTE_CHARS} Zeichen gekürzt.",
                    row.line
                ));
                note.chars().take(MAX_NOTE_CHARS).collect::<String>()
            } else {
                note.clone()
            }
        });
        cells.push(RestoreCell {
            employee_id,
            day,
            kind,
            shift_id,
            starts_at: if free { row.starts_at.clone() } else { None },
            ends_at: if free { row.ends_at.clone() } else { None },
            break_minutes: if free { row.break_minutes } else { None },
            note,
        });
    }
    if let Some(source) = first_foreign {
        if let Some(note) = exchange::moved_note(source, from, moved) {
            warnings.push(note);
        }
    }
    let written = schedule(&state)
        .restore(from, to, &cells, query.replace)
        .await?;
    Ok(Json(ExchangeReport {
        rows: rows.rows.len(),
        written,
        warnings,
    }))
}

/// Sends the month of the chosen employees as a personal calendar, one mail each.
///
/// The mail goes to the address of the **employee** through the e-mail account of the
/// signed-in user; the file holds only that person's days, their shifts and their times.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/ical/send",
    tag = "personal",
    request_body = IcalSendInput,
    responses(
        (status = 200, body = IcalSendReport),
        (status = 422, description = "Invalid range or no e-mail account", body = where_api::error::Problem)
    )
)]
pub async fn schedule_ical_send(
    State(state): State<AppState>,
    Extension(user): Extension<SessionUser>,
    Extension(scope): Extension<DepartmentScope>,
    Json(input): Json<IcalSendInput>,
) -> Result<Json<IcalSendReport>, ApiError> {
    let query = ScheduleQuery {
        from: input.from,
        to: input.to,
    };
    query.validate()?;
    let scope = PeopleScope::load(&state, &scope).await?;
    let chosen: HashSet<i64> = input.employee_ids.iter().copied().collect();
    if chosen.is_empty() {
        return Err(ApiError::from(DomainError::Validation(
            "No employee was chosen.".to_string(),
        )));
    }
    let konto = UserRepository::new(state.installation_pool().clone())
        .mail_account(user.id)
        .await?;
    let Some(konto) = konto.filter(|konto| {
        !konto.email.trim().is_empty()
            && !konto.smtp_server.trim().is_empty()
            && konto.smtp_port != 0
    }) else {
        return Err(ApiError::from(DomainError::Validation(
            "Für diesen Benutzer ist kein eMail-Konto eingerichtet.".to_string(),
        )));
    };
    let shifts_liste = shifts(&state).list().await?;
    let cells = schedule(&state).range(query.from, query.to).await?;
    let people = scope.keep(employees(&state).list(None).await?);
    let mut bericht = IcalSendReport::default();
    for employee in people {
        if !employee.id.is_some_and(|id| chosen.contains(&id)) {
            continue;
        }
        let name = format!("{} {}", employee.first_name, employee.last_name)
            .trim()
            .to_string();
        let Some(address) = employee
            .email
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
        else {
            bericht.skipped.push(IcalSkip {
                name,
                reason: "keine eMail-Adresse".to_string(),
            });
            continue;
        };
        let month = ical::month_of(&employee, &cells, &shifts_liste);
        if month.days.is_empty() {
            bericht.skipped.push(IcalSkip {
                name,
                reason: "keine Schicht im Zeitraum".to_string(),
            });
            continue;
        }
        let text = ical::ics(&month, Utc::now().naive_utc());
        let betreff = ical::subject(&month);
        let mut koerper = format!("{}\r\n\r\n", ical::calendar_name(&month));
        for day in &month.days {
            koerper.push_str(&ical::line(day));
            koerper.push_str("\r\n");
        }
        if let Some(unterschrift) = konto.signature.as_deref() {
            koerper.push_str("\r\n");
            koerper.push_str(unterschrift.trim());
            koerper.push_str("\r\n");
        }
        let datei = format!("dienstplan-{}.ics", query.from.format("%Y-%m"));
        match crate::mail::send_calendar(
            &konto,
            (&name, address),
            &betreff,
            &koerper,
            &datei,
            &text,
        )
        .await
        {
            Ok(()) => bericht.sent += 1,
            Err(problem) => bericht.skipped.push(IcalSkip {
                name,
                reason: problem.to_string(),
            }),
        }
    }
    Ok(Json(bericht))
}

#[utoipa::path(
    get,
    path = "/api/v1/people/schedule/dispo",
    tag = "personal",
    params(BoardQuery),
    responses(
        (status = 200, body = Vec<DispoEntry>),
        (status = 422, description = "Invalid range", body = where_api::error::Problem)
    )
)]
pub async fn dispo_range(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Query(query): Query<BoardQuery>,
) -> Result<Json<Vec<DispoEntry>>, ApiError> {
    query.validate()?;
    let scope = PeopleScope::load(&state, &scope).await?;
    let only = query.department();
    let allowed = disposible_location_ids(&state, &scope, only.as_deref()).await?;
    let mut entries = dispo_entries(&state).range(query.from, query.to).await?;
    if let Some(allowed) = allowed {
        entries.retain(|entry| allowed.contains(&entry.location_id));
    }
    Ok(Json(entries))
}

/// The demand the disposition of the days asks for.
///
/// Every event of the disposition adds the staffing of its flag to the shift its
/// start time belongs to; events without a time, without a flag or without a
/// usable shift stay in `unclear` and mark their day.
#[utoipa::path(
    get,
    path = "/api/v1/people/schedule/dispo-demand",
    tag = "personal",
    params(
        ("from" = String, Query, description = "First day (`YYYY-MM-DD`)"),
        ("to" = String, Query, description = "Last day (`YYYY-MM-DD`)"),
        ("department" = Option<String>, Query, description = "Only this department")
    ),
    responses(
        (status = 200, body = Vec<DayProposal>),
        (status = 422, description = "Invalid range", body = where_api::error::Problem)
    )
)]
pub async fn dispo_demand(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Query(query): Query<BoardQuery>,
) -> Result<Json<Vec<DayProposal>>, ApiError> {
    query.validate()?;
    let scope = PeopleScope::load(&state, &scope).await?;
    let only = query.department();
    let allowed = disposible_location_ids(&state, &scope, only.as_deref()).await?;
    let mut entries = dispo_entries(&state).range(query.from, query.to).await?;
    let flags = flags(&state).list().await?;
    let shifts = shifts(&state).list().await?;
    let houses = house_names(&state).await?;
    if let Some(allowed) = allowed {
        entries.retain(|entry| allowed.contains(&entry.location_id));
    }
    Ok(Json(dispo_demand::derive(
        &entries, &flags, &shifts, &houses,
    )))
}

/// The locations the session may dispose — the rows of the disposition on the board.
///
/// A location belongs to the department of the sub-department that works there; the
/// scope of `Personal-Disposition` decides which of them are shown.
#[utoipa::path(
    get,
    path = "/api/v1/people/schedule/locations",
    tag = "personal",
    params(DepartmentQuery),
    responses((status = 200, body = Vec<ListEntry>))
)]
pub async fn schedule_locations(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Query(query): Query<DepartmentQuery>,
) -> Result<Json<Vec<ListEntry>>, ApiError> {
    let scope = PeopleScope::load(&state, &scope).await?;
    let only = query.department();
    Ok(Json(
        disposible_locations(&state, &scope, only.as_deref()).await?,
    ))
}

/// Builds the schedule plan of a period.
///
/// Free cells are filled from the demand of the day, following the planning
/// rules and the shift catalogue; what cannot be filled comes back as a gap.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/generate",
    tag = "personal",
    request_body = GenerateInput,
    responses(
        (status = 200, body = GenerateReport),
        (status = 422, description = "Invalid period", body = where_api::error::Problem)
    )
)]
pub async fn schedule_generate(
    State(state): State<AppState>,
    Extension(scope): Extension<DepartmentScope>,
    Json(input): Json<GenerateInput>,
) -> Result<Json<GenerateReport>, ApiError> {
    let days = input.days()?;
    let scope = PeopleScope::load(&state, &scope).await?;
    let rules = planning(&state).get().await?;
    let shifts: Vec<Shift> = shifts(&state)
        .list()
        .await?
        .into_iter()
        // Only the shifts the group may dispose (no department: all of them).
        .filter(|shift| {
            shift
                .department
                .as_deref()
                .map(|department| scope.allows_dispo(department))
                .unwrap_or(true)
        })
        .collect();
    let people: Vec<generate::Person> = scope
        .keep(employees(&state).list(None).await?)
        .into_iter()
        .filter(|employee| employee.active && employee.id.is_some())
        .map(|employee| generate::Person {
            id: employee.id.unwrap_or_default(),
            name: format!("{} {}", employee.first_name, employee.last_name)
                .trim()
                .to_string(),
            position: employee.position.unwrap_or_default(),
            department: employee.department.unwrap_or_default(),
            subdepartment: employee.subdepartment.unwrap_or_default(),
            apprentice: employee.is_apprentice,
            auto_plan: employee.auto_plan,
            work_days: employee.work_days,
            weekly_minutes: employee.weekly_hours.map(weekly_minutes),
        })
        .collect();

    let from = days.first().copied().unwrap_or(input.from);
    let to = days.last().copied().unwrap_or(input.to);
    let history_from =
        from - chrono::Days::new(generate::history_days(rules.shift_rotation_days) as u64);
    let entries = schedule(&state).range(history_from, to).await?;
    let cells: Vec<generate::Cell> = entries
        .into_iter()
        .map(|entry| generate::Cell {
            employee_id: entry.employee_id,
            day: entry.day,
            kind: entry.kind,
            shift_id: entry.shift_id,
            starts_at: entry.starts_at,
            ends_at: entry.ends_at,
            break_minutes: entry.break_minutes,
        })
        .collect();
    let (history, mut existing): (Vec<generate::Cell>, Vec<generate::Cell>) =
        cells.into_iter().partition(|cell| cell.day < from);

    // `overwrite` replaces what is planned; sick, vacation and free time stay. Without
    // it nothing is touched — a free day (`FT`, `SoFT`) nobody set aside by hand is a
    // cell of the plan like any other and blocks its day just the same.
    let mut replaced = 0usize;
    if input.overwrite {
        replaced = match scope.departments(false) {
            None => schedule(&state).clear_planned_shifts(from, to).await?,
            Some(names) => {
                schedule(&state)
                    .clear_planned_shifts_of(from, to, &names)
                    .await?
            }
        };
        // The free days are the markers of the plan itself: a month that is rewritten
        // gets them from this run again.
        existing.retain(|cell| !is_free_day(&shifts, cell));
        existing.retain(|cell| cell.kind != crate::schedule::KIND_SHIFT);
    }

    // The stored demand of the days; a day without one uses what the
    // disposition asks for.
    let mut demand_days: BTreeMap<NaiveDate, Vec<DemandEntry>> = BTreeMap::new();
    for day in demand(&state).range(from, to).await? {
        demand_days.insert(day.day, day.entries);
    }
    let dispo = dispo_entries(&state).range(from, to).await?;
    let flags = flags(&state).list().await?;
    let houses = house_names(&state).await?;
    let subdepartments = house_subdepartments(&state).await?;
    let derived = dispo_demand::derive(&dispo, &flags, &shifts, &houses);
    let unclear_days: Vec<NaiveDate> = derived
        .iter()
        .filter(|proposal| proposal.marked)
        .map(|proposal| proposal.day)
        .collect();
    for proposal in &derived {
        if proposal.entries.is_empty() {
            continue;
        }
        demand_days
            .entry(proposal.day)
            .or_insert_with(|| proposal.entries.clone());
    }

    let holidays: Vec<NaiveDate> = holiday_calendar(&state)
        .range(from, to)
        .await?
        .holidays
        .into_iter()
        .map(|holiday| holiday.day)
        .collect();

    // Where the hours stand when the period starts: the board counts from
    // `balance_from` (or 1 January), the generator wraps up the same balance.
    let balance_from = rules.balance_from.unwrap_or_else(|| {
        NaiveDate::from_ymd_opt(chrono::Datelike::year(&from), 1, 1).unwrap_or(from)
    });
    let mut balance = generate::Balance {
        before: HashMap::new(),
        vacation_minutes: weekly_minutes(rules.vacation_day_hours),
        sick_minutes: weekly_minutes(rules.sick_day_hours),
    };
    balance.before = balance_before(&state, &shifts, &balance, balance_from, from, &people).await?;

    let foremanless = foremanless_houses(&state).await?;
    let (free_shift, compensation_shift) = free_shift_ids(&shifts);
    let context = generate::Context {
        rules: &rules,
        shifts: &shifts,
        people: &people,
        history: &history,
        existing: &existing,
        holidays: &holidays,
        demand: &demand_days,
        subdepartments: &subdepartments,
        foremanless: &foremanless,
        balance: Some(balance),
        free_shift,
        compensation_shift,
    };
    let plan = generate::generate(&context, &days);
    let written = schedule(&state)
        .write_shifts(
            &plan
                .cells
                .iter()
                .map(|cell| (cell.employee_id, cell.day, cell.shift_id))
                .collect::<Vec<_>>(),
        )
        .await?;
    let free_days = schedule(&state)
        .write_shifts(
            &plan
                .free
                .iter()
                .map(|cell| (cell.employee_id, cell.day, cell.shift_id))
                .collect::<Vec<_>>(),
        )
        .await?;
    Ok(Json(GenerateReport {
        days: days.len(),
        assigned: written,
        kept: plan.kept,
        replaced,
        extra: plan.extra,
        free_days,
        gaps: plan.gaps,
        unclear_days,
    }))
}

/// Replaces the entries of one location on one day.
#[utoipa::path(
    put,
    path = "/api/v1/people/schedule/dispo/{location}/{day}",
    tag = "personal",
    params(
        ("location" = i64, Path, description = "Id of the location"),
        ("day" = String, Path, description = "The day (`YYYY-MM-DD`)")
    ),
    request_body = DispoDayInput,
    responses(
        (status = 200, body = Vec<DispoEntry>),
        (status = 404, description = "Location unknown", body = where_api::error::Problem),
        (status = 422, description = "Invalid entry", body = where_api::error::Problem)
    )
)]
pub async fn dispo_day_set(
    State(state): State<AppState>,
    Path((location, day)): Path<(i64, String)>,
    Json(input): Json<DispoDayInput>,
) -> Result<Json<Vec<DispoEntry>>, ApiError> {
    let day = parse_day(&day)?;
    let entries = input.clean()?;
    let repository = dispo_entries(&state);
    if !repository.has_location(location).await? {
        return Err(ApiError::from(where_domain::DomainError::NotFound {
            entity: "location",
            id: location as i32,
        }));
    }
    Ok(Json(repository.replace_day(location, day, &entries).await?))
}

/// Sets one event of the disposition aside — or puts it back into the plan.
#[utoipa::path(
    put,
    path = "/api/v1/people/schedule/dispo/entry/{id}/ignored",
    tag = "personal",
    params(("id" = i64, Path, description = "Id of the entry")),
    request_body = DispoIgnoredInput,
    responses(
        (status = 200, body = DispoEntry),
        (status = 404, description = "Entry unknown", body = where_api::error::Problem)
    )
)]
pub async fn dispo_entry_ignored(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<DispoIgnoredInput>,
) -> Result<Json<DispoEntry>, ApiError> {
    let entry = dispo_entries(&state)
        .set_ignored(id, input.ignored)
        .await?
        .ok_or_else(|| {
            ApiError::from(where_domain::DomainError::NotFound {
                entity: "dispo entry",
                id: id as i32,
            })
        })?;
    Ok(Json(entry))
}

// ------------------------------------------------------------------- Flags

/// Every flag with its default staffing, in the maintained order.
#[utoipa::path(
    get,
    path = "/api/v1/people/flags",
    tag = "personal",
    responses((status = 200, body = Vec<Flag>))
)]
pub async fn list_flags(State(state): State<AppState>) -> Result<Json<Vec<Flag>>, ApiError> {
    Ok(Json(flags(&state).list().await?))
}

/// Adds a flag without staffing.
#[utoipa::path(
    post,
    path = "/api/v1/people/flags",
    tag = "personal",
    request_body = FlagInput,
    responses(
        (status = 201, body = Flag),
        (status = 409, description = "Code already exists", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn create_flag(
    State(state): State<AppState>,
    Json(input): Json<FlagInput>,
) -> Result<(StatusCode, Json<Flag>), ApiError> {
    let flag = flags(&state).create(&input.clean()?).await?;
    Ok((StatusCode::CREATED, Json(flag)))
}

/// Renames a flag; its entries keep it.
#[utoipa::path(
    put,
    path = "/api/v1/people/flags/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Flag id")),
    request_body = FlagInput,
    responses(
        (status = 200, body = Flag),
        (status = 404, description = "Unknown", body = where_api::error::Problem),
        (status = 409, description = "Code already exists", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn rename_flag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<FlagInput>,
) -> Result<Json<Flag>, ApiError> {
    let flag = flags(&state).rename(id, &input.clean()?).await?;
    match flag {
        Some(flag) => Ok(Json(flag)),
        None => Err(ApiError::from(DomainError::NotFound {
            entity: "flag",
            id: id as i32,
        })),
    }
}

/// Removes a flag; entries that carried it lose it.
#[utoipa::path(
    delete,
    path = "/api/v1/people/flags/{id}",
    tag = "personal",
    params(("id" = i64, Path, description = "Flag id")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown", body = where_api::error::Problem)
    )
)]
pub async fn delete_flag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    if !flags(&state).delete(id).await? {
        return Err(ApiError::from(DomainError::NotFound {
            entity: "flag",
            id: id as i32,
        }));
    }
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Replaces the staffing of a flag for one location.
#[utoipa::path(
    put,
    path = "/api/v1/people/flags/{id}/staffing/{location}",
    tag = "personal",
    params(
        ("id" = i64, Path, description = "Flag id"),
        ("location" = i64, Path, description = "Location id")
    ),
    request_body = FlagStaffingInput,
    responses(
        (status = 200, body = Vec<FlagStaffing>),
        (status = 404, description = "Unknown flag", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn flag_staffing_set(
    State(state): State<AppState>,
    Path((id, location)): Path<(i64, i64)>,
    Json(input): Json<FlagStaffingInput>,
) -> Result<Json<Vec<FlagStaffing>>, ApiError> {
    let rows = input.clean(location)?;
    match flags(&state).replace_staffing(id, location, &rows).await? {
        Some(staffing) => Ok(Json(staffing)),
        None => Err(ApiError::from(DomainError::NotFound {
            entity: "flag",
            id: id as i32,
        })),
    }
}

/// Replaces the positions a flag does not use.
#[utoipa::path(
    put,
    path = "/api/v1/people/flags/{id}/exclusions",
    tag = "personal",
    params(("id" = i64, Path, description = "Flag id")),
    request_body = FlagExclusionsInput,
    responses(
        (status = 200, body = Vec<String>),
        (status = 404, description = "Unknown flag", body = where_api::error::Problem),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn flag_exclusions_set(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<FlagExclusionsInput>,
) -> Result<Json<Vec<String>>, ApiError> {
    let positions = input.clean()?;
    match flags(&state).set_exclusions(id, &positions).await? {
        Some(excluded) => Ok(Json(excluded)),
        None => Err(ApiError::from(DomainError::NotFound {
            entity: "flag",
            id: id as i32,
        })),
    }
}

/// Creates the flags of the PeM legend that are missing, with their staffing.
#[utoipa::path(
    post,
    path = "/api/v1/people/flags/presets",
    tag = "personal",
    responses((status = 200, body = FlagPresetReport))
)]
pub async fn flag_presets(
    State(state): State<AppState>,
) -> Result<Json<FlagPresetReport>, ApiError> {
    Ok(Json(flags(&state).create_presets().await?))
}

// ------------------------------------------------------------------ Daily demand

/// `YYYY-MM-DD` from the path.
fn parse_day(text: &str) -> Result<chrono::NaiveDate, ApiError> {
    chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").map_err(|_| {
        ApiError::from(where_domain::DomainError::Validation(format!(
            "Unknown date “{text}”."
        )))
    })
}

/// How many employees of each position the days of the period need.
#[utoipa::path(
    get,
    path = "/api/v1/people/schedule/demand",
    tag = "personal",
    params(ScheduleQuery),
    responses(
        (status = 200, body = Vec<DayDemand>),
        (status = 422, description = "Invalid range", body = where_api::error::Problem)
    )
)]
pub async fn schedule_demand(
    State(state): State<AppState>,
    Query(query): Query<ScheduleQuery>,
) -> Result<Json<Vec<DayDemand>>, ApiError> {
    query.validate()?;
    Ok(Json(demand(&state).range(query.from, query.to).await?))
}

/// Replaces the demand of one day; positions without a number are dropped.
#[utoipa::path(
    put,
    path = "/api/v1/people/schedule/demand/{day}",
    tag = "personal",
    params(("day" = String, Path, description = "Day (`YYYY-MM-DD`)")),
    request_body = DemandInput,
    responses(
        (status = 200, body = DayDemand),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn schedule_demand_set(
    State(state): State<AppState>,
    Path(day): Path<String>,
    Json(input): Json<DemandInput>,
) -> Result<Json<DayDemand>, ApiError> {
    let day = parse_day(&day)?;
    let entries = input.clean()?;
    Ok(Json(demand(&state).replace(day, &entries).await?))
}

/// Copies the demand of one day onto the given days and replaces them.
#[utoipa::path(
    post,
    path = "/api/v1/people/schedule/demand/copy",
    tag = "personal",
    request_body = DemandCopyInput,
    responses(
        (status = 200, body = DemandCopied),
        (status = 422, description = "Validation failed", body = where_api::error::Problem)
    )
)]
pub async fn schedule_demand_copy(
    State(state): State<AppState>,
    Json(input): Json<DemandCopyInput>,
) -> Result<Json<DemandCopied>, ApiError> {
    let days = input.clean()?;
    let written = demand(&state).copy(input.from, &days).await?;
    Ok(Json(DemandCopied { days: written }))
}
