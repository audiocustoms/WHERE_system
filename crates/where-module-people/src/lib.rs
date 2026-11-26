//! `WHERE - are my people?` — employees and personnel.
//!
//! The module owns the employee master (`where_employees`) and plugs into the
//! core through [`where_api::ModulePlugin`]. Standalone it creates the database;
//! with `WHERE - is my stuff?` present it shares the core's database and
//! tenants.

pub mod demand;
pub mod demand_repo;
pub mod dispo;
pub mod dispo_demand;
pub mod dispo_repo;
pub mod employee;
pub mod employee_repo;
pub mod exchange;
pub mod flag_repo;
pub mod flags;
pub mod generate;
pub mod holiday_repo;
pub mod holiday_source;
pub mod holidays;
pub mod ical;
pub mod list_repo;
pub mod lists;
pub mod mail;
pub mod masterdata;
pub mod planning;
pub mod planning_repo;
pub mod routes;
pub mod schedule;
pub mod schedule_audit;
pub mod schedule_repo;
pub mod schedule_rules;
pub mod scope;
pub mod shift_repo;
pub mod shifts;

use axum::Router;
use axum::routing::get;
use utoipa::OpenApi as _;
use where_api::ModulePlugin;
use where_api::state::AppState;
use where_modules::ModuleId;

/// The personnel module.
#[derive(Debug, Default, Clone, Copy)]
pub struct PeopleModule;

impl ModulePlugin for PeopleModule {
    fn id(&self) -> ModuleId {
        ModuleId::People
    }

    fn router(&self) -> Router<AppState> {
        Router::new()
            .route("/people", get(routes::list).post(routes::create))
            .route("/people/new", get(routes::next_number))
            .route(
                "/people/{id}",
                get(routes::get_one)
                    .put(routes::replace)
                    .delete(routes::remove),
            )
            // The account of the user administration behind an employee.
            .route(
                "/people/{id}/user",
                axum::routing::put(routes::set_employee_user),
            )
            // Departments and positions of the mask.
            .route(
                "/people/lists/{kind}",
                get(routes::list_entries).post(routes::create_entry),
            )
            // The order route comes before `{id}` so `/order` is not an id attempt.
            .route(
                "/people/lists/{kind}/order",
                axum::routing::put(routes::reorder_entries),
            )
            .route(
                "/people/lists/{kind}/{id}",
                axum::routing::put(routes::rename_entry).delete(routes::delete_entry),
            )
            // Which sub-department works in a location, and which department a
            // sub-department sits under.
            .route(
                "/people/lists/{kind}/{id}/department",
                axum::routing::put(routes::set_list_department),
            )
            // Whether a house is staffed with a foreman.
            .route(
                "/people/lists/locations/{id}/foreman",
                axum::routing::put(routes::set_location_foreman),
            )
            // The tint of a department or a location in the schedule board.
            .route(
                "/people/lists/{kind}/{id}/color",
                axum::routing::put(routes::set_list_color),
            )
            // The order of the employees on the schedule board.
            .route(
                "/people/board-order",
                axum::routing::put(routes::board_order),
            )
            .route(
                "/people/masterdata/{area}/scan",
                axum::routing::post(routes::masterdata_scan),
            )
            .route(
                "/people/masterdata/{area}",
                axum::routing::post(routes::masterdata_import),
            )
            // Shifts and rules of the schedule planning.
            .route(
                "/people/shifts",
                get(routes::list_shifts).post(routes::create_shift),
            )
            .route(
                "/people/shifts/order",
                axum::routing::put(routes::reorder_shifts),
            )
            .route(
                "/people/shifts/{id}/exclusions",
                axum::routing::put(routes::shift_exclusions),
            )
            .route(
                "/people/shifts/{id}",
                axum::routing::put(routes::replace_shift).delete(routes::delete_shift),
            )
            .route(
                "/people/planning",
                get(routes::get_planning).put(routes::save_planning),
            )
            // Schedule plan: who works when in which shift.
            .route(
                "/people/schedule",
                get(routes::schedule_range).put(routes::schedule_set),
            )
            // The same cell on several days in a row.
            .route(
                "/people/schedule/series",
                axum::routing::post(routes::schedule_series),
            )
            // A reset of a whole month, asked for in the settings of the module.
            .route(
                "/people/schedule/reset",
                axum::routing::post(routes::schedule_reset),
            )
            // Public and school holidays of the tenant.
            .route("/people/holidays", get(routes::holiday_range))
            .route("/people/holidays/summary", get(routes::holiday_counts))
            .route(
                "/people/holidays/import",
                axum::routing::post(routes::holiday_import),
            )
            // Rows the board types itself: locations with a free-text name.
            .route(
                "/people/schedule/rows",
                axum::routing::post(routes::create_schedule_row),
            )
            .route(
                "/people/schedule/rows/{id}",
                axum::routing::put(routes::rename_schedule_row).delete(routes::delete_schedule_row),
            )
            // The disposition of the personnel manager (KOKOS.event PeM).
            .route("/people/schedule/dispo", get(routes::dispo_range))
            // The rows of the disposition on the board: only the locations of the
            // departments the session may dispose.
            .route(
                "/people/schedule/locations",
                get(routes::schedule_locations),
            )
            // The month of an employee as a calendar, sent by mail.
            .route(
                "/people/schedule/ical/send",
                axum::routing::post(routes::schedule_ical_send),
            )
            .route(
                "/people/schedule/dispo/scan",
                axum::routing::post(routes::dispo_scan),
            )
            .route(
                "/people/schedule/dispo/import",
                axum::routing::post(routes::dispo_import),
            )
            .route(
                "/people/schedule/dispo/reset",
                axum::routing::post(routes::dispo_reset),
            )
            // The exchange files of the board: one month out and back in.
            .route(
                "/people/schedule/exchange/{kind}/scan",
                axum::routing::post(routes::exchange_scan),
            )
            .route(
                "/people/schedule/exchange/dispo",
                axum::routing::post(routes::exchange_dispo),
            )
            .route(
                "/people/schedule/exchange/plan",
                axum::routing::post(routes::exchange_plan),
            )
            .route(
                "/people/schedule/dispo/{location}/{day}",
                axum::routing::put(routes::dispo_day_set),
            )
            .route(
                "/people/schedule/dispo/entry/{id}/ignored",
                axum::routing::put(routes::dispo_entry_ignored),
            )
            // The flags of the disposition: the codes PeM writes into the event
            // texts, each with the staffing it needs.
            .route(
                "/people/flags",
                get(routes::list_flags).post(routes::create_flag),
            )
            .route(
                "/people/flags/presets",
                axum::routing::post(routes::flag_presets),
            )
            .route(
                "/people/flags/{id}/staffing/{location}",
                axum::routing::put(routes::flag_staffing_set),
            )
            .route(
                "/people/flags/{id}/exclusions",
                axum::routing::put(routes::flag_exclusions_set),
            )
            .route(
                "/people/flags/{id}",
                axum::routing::put(routes::rename_flag).delete(routes::delete_flag),
            )
            // Staffing demand per day and position.
            .route("/people/schedule/demand", get(routes::schedule_demand))
            .route(
                "/people/schedule/demand/copy",
                axum::routing::post(routes::schedule_demand_copy),
            )
            .route(
                "/people/schedule/demand/{day}",
                axum::routing::put(routes::schedule_demand_set),
            )
            // The demand the disposition of the days asks for.
            .route("/people/schedule/dispo-demand", get(routes::dispo_demand))
            // Builds a plan from the demand and the rules.
            .route(
                "/people/schedule/generate",
                axum::routing::post(routes::schedule_generate),
            )
            // Checks the written plan against the rules of the planning.
            .route("/people/schedule/audit", get(routes::schedule_audit))
    }

    fn openapi(&self) -> utoipa::openapi::OpenApi {
        ApiDoc::openapi()
    }
}

#[derive(utoipa::OpenApi)]
#[openapi(
    tags((name = "personal", description = "Employee master data")),
    paths(
        routes::list,
        routes::get_one,
        routes::next_number,
        routes::create,
        routes::replace,
        routes::remove,
        routes::set_employee_user,
        routes::list_entries,
        routes::create_entry,
        routes::rename_entry,
        routes::set_list_department,
        routes::set_location_foreman,
        routes::set_list_color,
        routes::board_order,
        routes::delete_entry,
        routes::reorder_entries,
        routes::masterdata_scan,
        routes::masterdata_import,
        routes::list_shifts,
        routes::create_shift,
        routes::replace_shift,
        routes::delete_shift,
        routes::reorder_shifts,
        routes::shift_exclusions,
        routes::get_planning,
        routes::save_planning,
        routes::schedule_range,
        routes::schedule_set,
        routes::schedule_series,
        routes::schedule_reset,
        routes::holiday_range,
        routes::holiday_counts,
        routes::holiday_import,
        routes::create_schedule_row,
        routes::rename_schedule_row,
        routes::delete_schedule_row,
        routes::dispo_scan,
        routes::dispo_import,
        routes::dispo_reset,
        routes::exchange_scan,
        routes::exchange_dispo,
        routes::exchange_plan,
        routes::dispo_range,
        routes::schedule_locations,
        routes::schedule_ical_send,
        routes::dispo_day_set,
        routes::dispo_entry_ignored,
        routes::list_flags,
        routes::create_flag,
        routes::rename_flag,
        routes::delete_flag,
        routes::flag_staffing_set,
        routes::flag_exclusions_set,
        routes::flag_presets,
        routes::schedule_demand,
        routes::schedule_demand_set,
        routes::schedule_demand_copy,
        routes::dispo_demand,
        routes::schedule_generate,
        routes::schedule_audit,
    )
)]
struct ApiDoc;
