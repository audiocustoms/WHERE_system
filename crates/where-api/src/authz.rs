//! Who may call what.
//!
//! One middleware for the whole `/api/v1` tree: every route except the login mask
//! needs a session, and the business routes need the right of their area from the
//! permission groups (`where_user_group_rights`, `img/Admin Menü/Benutzerverwaltung/
//! Berechtigungsgruppe`).
//!
//! An account holds a right when any of its groups carries it in `Max. Rechte` or
//! `Min. Rechte` (see `where_persistence::user_repo`). The table below is the missing
//! link between those keys and the routes.

use axum::extract::{MatchedPath, Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use where_domain::permissions::catalogue;
use where_domain::{ADMIN_RIGHT, DomainError};

use crate::error::ApiError;
use crate::routes::auth::bearer;
use crate::state::AppState;

/// What a route asks of the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Guard {
    /// No session needed — the login mask and the sign-in itself.
    Public,
    /// A session, but no particular right (the start page, the selection lists
    /// every mask needs).
    SignedIn,
    /// A session holding this right of the catalogue.
    Right(&'static str),
    /// A session holding at least one of these rights of the catalogue.
    Any(&'static [&'static str]),
}

fn admin() -> Guard {
    Guard::Right(ADMIN_RIGHT)
}

/// Creating an entry of `Stammdaten → Abteilungen und Positionen` or a shift of an own
/// department (`Personal-Disposition`).
const DISPO_CREATE: &str = "personaldispo.personaldispo-anlegen";
/// Changing an existing entry of the master data of the disposition.
const DISPO_EDIT: &str = "personaldispo.personaldispo-editieren";

fn address_read() -> Guard {
    Guard::Right("adressen.adresse-lesen")
}

fn article_read() -> Guard {
    Guard::Right("artikel.artikel-lesen")
}

/// The rule of a route, from its method and the path the router matched.
///
/// `path` may carry the `/api/v1` prefix or not; both are accepted. Unknown paths
/// require a session but no right, so a new route is never open by accident.
pub fn guard_for(method: &Method, path: &str) -> Guard {
    let path = path.strip_prefix("/api/v1").unwrap_or(path);
    match (method.as_str(), path) {
        ("OPTIONS", _) => Guard::Public,
        // The login mask itself.
        (
            _,
            "/auth/users" | "/auth/tenants" | "/auth/login" | "/auth/session" | "/auth/logout"
            | "/auth/password",
        ) => Guard::Public,
        // Name of the product: the mask shows it before anybody signs in.
        ("GET", "/installation") => Guard::Public,
        // Start page: figures from the database, no right of its own.
        (_, "/dashboard") => Guard::SignedIn,
        // TeamAgenda (`img/Team Agenda`).
        ("GET", p) if p == "/agenda" || p.starts_with("/agenda/") => {
            Guard::Right("teamagenda.termine-lesen")
        }
        ("POST", "/agenda") | ("POST", "/agenda/categories") => {
            Guard::Right("teamagenda.termine-anlegen")
        }
        ("PUT" | "DELETE", p) if p.starts_with("/agenda") => {
            Guard::Right("teamagenda.termine-editieren")
        }
        (_, p) if p.starts_with("/agenda") => Guard::Right("teamagenda.termine-lesen"),
        // Addresses (`img/Kunden`, `img/Lieferanten`, `img/Interessenten`).
        ("POST", "/addresses") => Guard::Right("adressen.adresse-anlegen"),
        ("GET", "/addresses") | ("GET", "/addresses/duplicates") => address_read(),
        ("POST", "/addresses/search") => address_read(),
        ("GET", "/addresses/{id}") => address_read(),
        ("PUT", "/addresses/{id}") => Guard::Right("adressen.adresse-editieren"),
        ("DELETE", "/addresses/{id}") => Guard::Right("adressen.adresse-loeschen"),
        // Articles (`img/articles_01..05.PNG`).
        ("GET", "/articles") | ("GET", "/articles/{id}") | ("GET", "/articles/{id}/stock") => {
            article_read()
        }
        ("GET", "/articles/search") | ("POST", "/articles/search") => article_read(),
        ("GET", "/articles/searches")
        | ("POST", "/articles/searches")
        | ("DELETE", "/articles/searches/{id}") => article_read(),
        ("GET", "/articles/{id}/devices/{device_id}")
        | ("GET", "/articles/{id}/files")
        | ("GET", "/articles/{id}/files/history")
        | ("GET", "/articles/{id}/files/{file_id}/content")
        | ("GET", "/articles/{id}/reports")
        | ("GET", "/articles/{id}/devices/{device_id}/reports")
        | ("GET", "/articles/{id}/devices/{device_id}/reports/{report_id}/content")
        | ("GET", "/articles/{id}/image") => article_read(),
        // The form of a new article brings the suggestions of creating one along.
        ("GET", "/articles/new") => Guard::Right("artikel.artikel-anlegen"),
        ("POST", "/articles") => Guard::Right("artikel.artikel-anlegen"),
        ("POST", "/articles/{id}/devices")
        | ("PUT", "/articles/{id}/devices/{device_id}")
        | ("DELETE", "/articles/{id}/devices/{device_id}")
        | ("PUT", "/articles/{id}")
        | ("POST", "/articles/{id}/files")
        | ("POST", "/articles/{id}/files/links")
        | ("DELETE", "/articles/{id}/files/{file_id}")
        | ("POST", "/articles/{id}/devices/{device_id}/reports")
        | ("DELETE", "/articles/{id}/devices/{device_id}/reports/{report_id}")
        | ("POST", "/articles/{id}/image")
        | ("DELETE", "/articles/{id}/image")
        | ("PUT", "/articles/{id}/image/placeholder") => Guard::Right("artikel.artikel-editieren"),
        ("DELETE", "/articles/{id}") => Guard::Right("artikel.artikel-loeschen"),
        // Import manager: it creates addresses or articles, so the same right.
        ("POST", "/import/articles") => Guard::Right("artikel.artikel-anlegen"),
        ("POST", "/import/addresses") => Guard::Right("adressen.adresse-anlegen"),
        // Selection lists: every signed-in user may read them, only administration
        // maintains them.
        ("GET", "/settings") => admin(),
        ("POST" | "PUT" | "DELETE", p) if p.starts_with("/settings") => admin(),
        ("PUT", p) if p.starts_with("/prefs/") => admin(),
        ("GET", p) if p.starts_with("/master-data/") => Guard::SignedIn,
        ("POST" | "PUT" | "DELETE", p) if p.starts_with("/master-data/") => admin(),
        // Module registry: every signed-in user may read it, only administration
        // installs. The endpoints of the modules themselves sit behind the same
        // right and additionally behind `modules::require_module`.
        ("GET", "/modules") => Guard::SignedIn,
        (_, p) if p.starts_with("/modules") => admin(),
        // The form of a new employee fetches the next personnel number.
        ("GET", "/people/new") => Guard::Right("personal.personal-anlegen"),
        // Master data of the personnel mask (departments, sub-departments, positions,
        // locations): reading with the personnel or the disposition right, creating
        // with `Personaldispo anlegen`, everything else with `Personaldispo editieren`
        // — the administration may always do all of it.
        ("GET", p) if p.starts_with("/people/lists") => Guard::Any(&[
            "personal.personal-lesen",
            "personaldispo.personaldispo-lesen",
            ADMIN_RIGHT,
        ]),
        ("POST", p) if p.starts_with("/people/lists") => Guard::Any(&[DISPO_CREATE, ADMIN_RIGHT]),
        (_, p) if p.starts_with("/people/lists") => Guard::Any(&[DISPO_EDIT, ADMIN_RIGHT]),
        // The order of the rows on the duty board is planning work: everyone who may
        // write the plan may arrange its rows.
        ("PUT", "/people/board-order") => Guard::Right("personal.personal-editieren"),
        // The account behind an employee: the user administration sets it, so the
        // administration may always name it.
        ("PUT", p) if p.ends_with("/user") && p.starts_with("/people/") => {
            Guard::Any(&["personal.personal-editieren", ADMIN_RIGHT])
        }
        // The master data as exchange files. Reading a file and writing the lists or the
        // shifts is administration work; the employees belong to the personnel work —
        // that they carry personal data is checked by the handler itself.
        ("POST", "/people/masterdata/{area}/scan") => Guard::Right("personal.personal-lesen"),
        ("POST", "/people/masterdata/{area}") => Guard::Right("personal.personal-editieren"),
        // Flags of the disposition and their staffing: the same rules as the lists
        // they belong to.
        ("GET", p) if p.starts_with("/people/flags") => Guard::Right("personal.personal-lesen"),
        (_, p) if p.starts_with("/people/flags") => admin(),
        // Duty plan master data (shifts, rules): read with the disposition right — the
        // board lives on it. A shift of an own department is created with
        // `Personaldispo anlegen` and changed with `Personaldispo editieren` (the
        // department scope is checked by the handlers); the order of the shifts and the
        // rules of the planning stay with the administration.
        ("GET", p) if p.starts_with("/people/shifts") || p == "/people/planning" => {
            Guard::Any(&["personaldispo.personaldispo-lesen", ADMIN_RIGHT])
        }
        ("PUT", "/people/shifts/order") => admin(),
        ("POST", p) if p.starts_with("/people/shifts") => Guard::Any(&[DISPO_CREATE, ADMIN_RIGHT]),
        (_, p) if p.starts_with("/people/shifts") => Guard::Any(&[DISPO_EDIT, ADMIN_RIGHT]),
        ("PUT", "/people/planning") => admin(),
        // The duty plan itself belongs to `Personal-Disposition`: read with its right,
        // written with the right to edit it.
        ("GET", "/people/duty") => Guard::Right("personaldispo.personaldispo-lesen"),
        // The rows of the disposition: the locations of the departments of the session.
        ("GET", "/people/duty/locations") => {
            Guard::Any(&["personaldispo.personaldispo-lesen", ADMIN_RIGHT])
        }
        ("PUT", "/people/duty") => Guard::Right("personaldispo.personaldispo-editieren"),
        ("POST", "/people/duty/series") => Guard::Right("personaldispo.personaldispo-editieren"),
        ("POST", "/people/duty/generate") => Guard::Right("personaldispo.personaldispo-editieren"),
        // Checking the written plan against the rules is reading.
        ("GET", "/people/duty/audit") => Guard::Right("personaldispo.personaldispo-lesen"),
        // Rows the board types itself are disposition rows: writing them is planning.
        (_, p) if p.starts_with("/people/duty/rows") => {
            Guard::Right("personaldispo.personaldispo-editieren")
        }
        // Staffing demand per day: read with the dispo right, written with the right
        // to edit the plan.
        ("GET", p) if p.starts_with("/people/duty/demand") => {
            Guard::Right("personaldispo.personaldispo-lesen")
        }
        (_, p) if p.starts_with("/people/duty/demand") => {
            Guard::Right("personaldispo.personaldispo-editieren")
        }
        // A reset empties a whole month of the plan or of the disposition; it is
        // asked for in the settings of the module, so only administration may do it.
        ("POST", "/people/duty/reset") => admin(),
        ("POST", "/people/duty/dispo/reset") => admin(),
        // The exchange files write a whole month at once; that is planning work.
        ("POST", p) if p.starts_with("/people/duty/exchange") => {
            Guard::Right("personaldispo.personaldispo-editieren")
        }
        // Sending the month of an employee as a calendar is disposition work as well.
        ("POST", "/people/duty/ical/send") => {
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        }
        // The PeM disposition: reading the entries needs the dispo right, reading a
        // file and writing needs the right to edit plan data.
        ("GET", p) if p.starts_with("/people/duty/dispo") => {
            Guard::Right("personaldispo.personaldispo-lesen")
        }
        (_, p) if p.starts_with("/people/duty/dispo") => {
            Guard::Right("personaldispo.personaldispo-editieren")
        }
        // Holidays come from an online source, so they are maintained by the
        // administration; reading them needs no more than the personnel right.
        ("GET", p) if p.starts_with("/people/holidays") => Guard::Right("personal.personal-lesen"),
        (_, p) if p.starts_with("/people/holidays") => admin(),
        // `WHERE - are my people?`: personnel master data.
        ("GET", p) if p == "/people" || p.starts_with("/people/") => {
            Guard::Right("personal.personal-lesen")
        }
        ("POST", "/people") => Guard::Right("personal.personal-anlegen"),
        ("PUT", p) if p.starts_with("/people/") => Guard::Right("personal.personal-editieren"),
        ("DELETE", p) if p.starts_with("/people/") => Guard::Right("personal.personal-loeschen"),
        // Admin menu of the modules: order settings, bank list, bank accounts,
        // tenants and the user administration.
        (_, p) if p.starts_with("/orders") => admin(),
        ("POST", "/banks") | ("PUT", "/banks/{id}") | ("DELETE", "/banks/{id}") => admin(),
        ("POST", "/banks/sync") => admin(),
        (_, p) if p.starts_with("/bank-accounts") => admin(),
        // Warehouses and bin locations (`img/Admin Menü/Lager und Lagerplätze`): admin mask.
        (_, p) if p.starts_with("/warehouses") => admin(),
        (_, p) if p.starts_with("/tenants") => admin(),
        (_, p) if p.starts_with("/users") || p.starts_with("/user-groups") => admin(),
        (_, "/permissions") => admin(),
        _ => Guard::SignedIn,
    }
}

/// The wording of a right for the message of a `403`.
pub fn right_label(key: &str) -> Option<String> {
    catalogue()
        .into_iter()
        .flat_map(|group| group.areas)
        .flat_map(|area| area.rights)
        .find(|right| right.key == key)
        .map(|right| right.label)
}

/// The departments a session may work in — per area, `None` = every one of them.
///
/// The scope of the permission groups (`where_user_group_scope`, kinds
/// `personal-department` and `personaldispo-department`), united over the groups of
/// the account; administration is never restricted. The ids are
/// `where_people_departments` keys, so the module that owns that list resolves them
/// into names. The two lists are independent: a department of `Personal` never opens
/// `Personal-Disposition` and the other way round.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DepartmentScope {
    /// Departments of `Personal`.
    pub personal: Option<Vec<i64>>,
    /// Departments of `Personal-Disposition`.
    pub dispo: Option<Vec<i64>>,
}

impl DepartmentScope {
    /// `true` when the session may work everywhere.
    pub fn all(&self) -> bool {
        self.personal.is_none() && self.dispo.is_none()
    }
}

/// Checks the session and the right of every `/api/v1` request.
pub async fn authorize(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let path = matched_path(&request);
    let guard = guard_for(request.method(), &path);
    if guard == Guard::Public {
        return next.run(request).await;
    }
    let Some(token) = bearer(request.headers()) else {
        return deny(DomainError::Unauthorized(
            "Für diese Aktion ist eine Anmeldung nötig.".to_string(),
        ));
    };
    let user = match state.auth.session(&token).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return deny(DomainError::Unauthorized(
                "Die Sitzung ist abgelaufen — bitte neu anmelden.".to_string(),
            ));
        }
        Err(error) => return ApiError::from(error).into_response(),
    };
    // Nothing continues without a changed password: only the login itself and the
    // password change stay open (`ChangeNextLogin`).
    if user.must_change_password()
        && !path
            .strip_prefix("/api/v1")
            .unwrap_or(&path)
            .starts_with("/auth")
    {
        return deny(DomainError::Forbidden(
            "Das Kennwort muss zuerst geändert werden.".to_string(),
        ));
    }
    if let Guard::Right(key) = guard
        && !user.has(key)
    {
        let label = right_label(key).unwrap_or_else(|| key.to_string());
        return deny(DomainError::Forbidden(format!(
            "Für diese Aktion fehlt das Recht „{label}“."
        )));
    }
    if let Guard::Any(keys) = guard
        && !keys.iter().any(|key| user.has(key))
    {
        let labels: Vec<String> = keys
            .iter()
            .map(|key| right_label(key).unwrap_or_else(|| (*key).to_string()))
            .collect();
        return deny(DomainError::Forbidden(format!(
            "Für diese Aktion fehlt eines der Rechte: {}.",
            labels.join(" / ")
        )));
    }
    // A tenant with its own database gets its state; the handlers keep asking for
    // `State<AppState>` and thus work on that database. The path here is the actual
    // one, because the `mandanten.*` option hangs on the key.
    let actual = request.uri().path().to_string();
    if !uses_installation_database(&actual, request.uri().query())
        && let Some(mandant) = user.mandant
        && let Some(tenant) = state.for_tenant(mandant)
    {
        request.extensions_mut().insert(tenant);
    }
    // The checked session is available to the handlers.
    request.extensions_mut().insert(user.clone());
    // Which departments the account may work in (personnel and disposition).
    let scope = if user.has(ADMIN_RIGHT) {
        DepartmentScope::default()
    } else {
        match state.users.department_scopes(user.id).await {
            Ok((personal, dispo)) => DepartmentScope { personal, dispo },
            Err(error) => return ApiError::from(error).into_response(),
        }
    };
    request.extensions_mut().insert(scope);
    next.run(request).await
}

/// Routes that belong to the installation itself, not to a tenant: the accounts, the
/// groups with their rights, the register of the tenants and the eleven options of the
/// tenant mask (`mandanten.*`), which the original keeps in one `"System"` row.
fn uses_installation_database(path: &str, query: Option<&str>) -> bool {
    let path = path.strip_prefix("/api/v1").unwrap_or(path);
    if [
        "/auth",
        "/users",
        "/user-groups",
        "/permissions",
        "/tenants",
        "/modules",
    ]
    .iter()
    .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
    {
        return true;
    }
    if let Some(key) = path.strip_prefix("/prefs/") {
        return key.starts_with("mandanten.");
    }
    path == "/prefs" && query.is_some_and(|query| query.contains("prefix=mandanten."))
}

/// The path as the router matched it, so `{id}` stays a placeholder.
fn matched_path(request: &Request) -> String {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string())
}

fn deny(error: DomainError) -> Response {
    ApiError::from(error).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard(method: &str, path: &str) -> Guard {
        guard_for(&Method::from_bytes(method.as_bytes()).unwrap(), path)
    }

    #[test]
    fn the_login_mask_and_the_start_page_need_no_right() {
        assert_eq!(guard("GET", "/api/v1/auth/users"), Guard::Public);
        assert_eq!(guard("POST", "/api/v1/auth/login"), Guard::Public);
        assert_eq!(guard("GET", "/api/v1/auth/session"), Guard::Public);
        assert_eq!(guard("POST", "/api/v1/auth/logout"), Guard::Public);
        // The mask shows the product name before anybody signs in.
        assert_eq!(guard("GET", "/api/v1/installation"), Guard::Public);
        assert_eq!(guard("OPTIONS", "/api/v1/articles"), Guard::Public);
        assert_eq!(guard("GET", "/api/v1/dashboard"), Guard::SignedIn);
    }

    #[test]
    fn writing_an_address_or_an_article_asks_for_its_right() {
        assert_eq!(
            guard("POST", "/api/v1/addresses"),
            Guard::Right("adressen.adresse-anlegen")
        );
        assert_eq!(
            guard("PUT", "/api/v1/addresses/{id}"),
            Guard::Right("adressen.adresse-editieren")
        );
        assert_eq!(
            guard("DELETE", "/api/v1/addresses/{id}"),
            Guard::Right("adressen.adresse-loeschen")
        );
        assert_eq!(
            guard("POST", "/api/v1/articles"),
            Guard::Right("artikel.artikel-anlegen")
        );
        assert_eq!(
            guard("PUT", "/api/v1/articles/{id}"),
            Guard::Right("artikel.artikel-editieren")
        );
        assert_eq!(
            guard("DELETE", "/api/v1/articles/{id}"),
            Guard::Right("artikel.artikel-loeschen")
        );
        // Import manager: creating through the CSV file is creating.
        assert_eq!(
            guard("POST", "/api/v1/import/articles"),
            Guard::Right("artikel.artikel-anlegen")
        );
        assert_eq!(
            guard("POST", "/api/v1/import/addresses"),
            Guard::Right("adressen.adresse-anlegen")
        );
    }

    #[test]
    fn reading_and_searching_ask_for_the_read_right() {
        assert_eq!(
            guard("GET", "/api/v1/addresses"),
            Guard::Right("adressen.adresse-lesen")
        );
        assert_eq!(
            guard("POST", "/api/v1/addresses/search"),
            Guard::Right("adressen.adresse-lesen")
        );
        assert_eq!(
            guard("GET", "/api/v1/articles"),
            Guard::Right("artikel.artikel-lesen")
        );
        assert_eq!(
            guard("GET", "/api/v1/articles/{id}/stock"),
            Guard::Right("artikel.artikel-lesen")
        );
        assert_eq!(
            guard("GET", "/api/v1/agenda"),
            Guard::Right("teamagenda.termine-lesen")
        );
        assert_eq!(
            guard("POST", "/api/v1/agenda"),
            Guard::Right("teamagenda.termine-anlegen")
        );
        assert_eq!(
            guard("PUT", "/api/v1/agenda/{id}"),
            Guard::Right("teamagenda.termine-editieren")
        );
        // The selection lists of the masks stay readable for every user.
        assert_eq!(guard("GET", "/api/v1/settings/anreden"), Guard::SignedIn);
        assert_eq!(
            guard("GET", "/api/v1/master-data/number-series/customer"),
            Guard::SignedIn
        );
        assert_eq!(
            guard("GET", "/api/v1/master-data/article-groups"),
            Guard::SignedIn
        );
    }

    #[test]
    fn the_admin_menu_needs_the_administrative_right() {
        for (method, path) in [
            ("GET", "/api/v1/users"),
            ("POST", "/api/v1/user-groups"),
            ("GET", "/api/v1/permissions"),
            ("GET", "/api/v1/tenants"),
            ("PUT", "/api/v1/settings/anreden/4"),
            ("POST", "/api/v1/banks/sync"),
            ("GET", "/api/v1/bank-accounts"),
            ("GET", "/api/v1/orders/timeline"),
            ("PUT", "/api/v1/prefs/mandanten.sync-aktiv"),
            ("POST", "/api/v1/master-data/article-groups"),
            ("PUT", "/api/v1/master-data/number-series/customer"),
            ("GET", "/api/v1/settings"),
            ("POST", "/api/v1/people/duty/reset"),
            ("POST", "/api/v1/people/duty/dispo/reset"),
        ] {
            assert_eq!(
                guard(method, path),
                Guard::Right(ADMIN_RIGHT),
                "{method} {path}"
            );
        }
    }

    #[test]
    fn only_the_installation_routes_ignore_the_tenant_database() {
        for path in [
            "/api/v1/users",
            "/api/v1/user-groups/7",
            "/api/v1/permissions",
            "/api/v1/tenants/1",
            "/api/v1/auth/session",
        ] {
            assert!(uses_installation_database(path, None), "{path}");
        }
        for path in [
            "/api/v1/articles",
            "/api/v1/addresses",
            "/api/v1/settings/anreden",
            "/api/v1/master-data/article-groups",
            "/api/v1/dashboard",
        ] {
            assert!(!uses_installation_database(path, None), "{path}");
        }
        // The eleven options of the tenant mask belong to the installation, the
        // settings of the other masks to the tenant.
        assert!(uses_installation_database(
            "/api/v1/prefs",
            Some("prefix=mandanten.")
        ));
        assert!(uses_installation_database(
            "/api/v1/prefs/mandanten.sync-aktiv",
            None
        ));
        assert!(!uses_installation_database(
            "/api/v1/prefs/address.maps.land",
            None
        ));
        assert!(!uses_installation_database(
            "/api/v1/prefs",
            Some("prefix=")
        ));
    }

    #[test]
    fn an_unknown_route_needs_a_session_but_no_right() {
        assert_eq!(guard("GET", "/api/v1/noch-nicht-da"), Guard::SignedIn);
    }

    #[test]
    fn the_personnel_lists_follow_the_personnel_and_the_disposition_rights() {
        assert_eq!(
            guard("GET", "/api/v1/people/lists/departments"),
            Guard::Any(&[
                "personal.personal-lesen",
                "personaldispo.personaldispo-lesen",
                ADMIN_RIGHT
            ])
        );
        assert_eq!(
            guard("GET", "/api/v1/people/lists/positions"),
            Guard::Any(&[
                "personal.personal-lesen",
                "personaldispo.personaldispo-lesen",
                ADMIN_RIGHT
            ])
        );
        assert_eq!(
            guard("POST", "/api/v1/people/lists/departments"),
            Guard::Any(&["personaldispo.personaldispo-anlegen", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/lists/positions/3"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/lists/locations/3/department"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/lists/subdepartments/3/department"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("DELETE", "/api/v1/people/lists/departments/4"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/lists/departments/4/color"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/lists/locations/4/color"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/lists/subdepartments/4/color"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/lists/positions/4/color"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/board-order"),
            Guard::Right("personal.personal-editieren")
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/7/user"),
            Guard::Any(&["personal.personal-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("GET", "/api/v1/people/flags"),
            Guard::Right("personal.personal-lesen")
        );
        assert_eq!(
            guard("POST", "/api/v1/people/flags/presets"),
            Guard::Right(ADMIN_RIGHT)
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/flags/3/staffing/1"),
            Guard::Right(ADMIN_RIGHT)
        );
        assert_eq!(
            guard("DELETE", "/api/v1/people/flags/5"),
            Guard::Right(ADMIN_RIGHT)
        );
        assert_eq!(
            guard("GET", "/api/v1/people/shifts"),
            Guard::Any(&["personaldispo.personaldispo-lesen", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("POST", "/api/v1/people/shifts"),
            Guard::Any(&["personaldispo.personaldispo-anlegen", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/shifts/order"),
            Guard::Right(ADMIN_RIGHT)
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/shifts/7"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("DELETE", "/api/v1/people/shifts/7"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/shifts/7/exclusions"),
            Guard::Any(&["personaldispo.personaldispo-editieren", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("GET", "/api/v1/people/planning"),
            Guard::Any(&["personaldispo.personaldispo-lesen", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/planning"),
            Guard::Right(ADMIN_RIGHT)
        );
        assert_eq!(
            guard("GET", "/api/v1/people/duty"),
            Guard::Right("personaldispo.personaldispo-lesen")
        );
        assert_eq!(
            guard("GET", "/api/v1/people/duty/locations"),
            Guard::Any(&["personaldispo.personaldispo-lesen", ADMIN_RIGHT])
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/duty"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("POST", "/api/v1/people/duty/series"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("POST", "/api/v1/people/duty/rows"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/duty/rows/7"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("DELETE", "/api/v1/people/duty/rows/7"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("GET", "/api/v1/people/duty/demand"),
            Guard::Right("personaldispo.personaldispo-lesen")
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/duty/demand/2026-11-20"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("POST", "/api/v1/people/duty/demand/copy"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("POST", "/api/v1/people/duty/dispo/scan"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("POST", "/api/v1/people/duty/dispo/import"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("GET", "/api/v1/people/duty/dispo"),
            Guard::Right("personaldispo.personaldispo-lesen")
        );
        assert_eq!(
            guard("PUT", "/api/v1/people/duty/dispo/4/2026-12-01"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
        assert_eq!(
            guard("GET", "/api/v1/people/duty/dispo-demand"),
            Guard::Right("personaldispo.personaldispo-lesen")
        );
        assert_eq!(
            guard("POST", "/api/v1/people/duty/generate"),
            Guard::Right("personaldispo.personaldispo-editieren")
        );
    }

    #[test]
    fn every_right_of_the_table_exists_in_the_catalogue() {
        let known = where_domain::all_right_keys();
        for key in [
            ADMIN_RIGHT,
            "adressen.adresse-lesen",
            "adressen.adresse-anlegen",
            "adressen.adresse-editieren",
            "adressen.adresse-loeschen",
            "artikel.artikel-lesen",
            "artikel.artikel-anlegen",
            "artikel.artikel-editieren",
            "artikel.artikel-loeschen",
            "teamagenda.termine-lesen",
            "teamagenda.termine-anlegen",
            "teamagenda.termine-editieren",
        ] {
            assert!(known.iter().any(|right| right == key), "{key} fehlt");
        }
        assert_eq!(
            right_label(ADMIN_RIGHT).as_deref(),
            Some("Zugriff erlauben")
        );
    }
}
