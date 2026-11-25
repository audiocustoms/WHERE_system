pub mod addresses;
pub mod agenda;
pub mod articles;
pub mod auth;
pub mod dashboard;
pub mod files;
pub mod health;
pub mod import;
pub mod inspection;
pub mod master_data;
pub mod modules;
pub mod settings;
pub mod tenants;
pub mod users;
pub mod warehouses;

use axum::Router;
use axum::routing::{delete, get, post, put};

use crate::state::AppState;

/// `/api/v1` route tree.
///
/// Resource names are English; the legacy schema identifiers stay untouched
/// inside the persistence layer.
pub fn api_v1() -> Router<AppState> {
    Router::new()
        // Start page ("Zentrale"): one aggregate read for the whole dashboard.
        .route("/dashboard", get(dashboard::load))
        // Kalender ("TeamAgenda" in epirent): appointments of a range, series expanded.
        .route("/agenda", get(agenda::range).post(agenda::create))
        .route(
            "/agenda/categories",
            get(agenda::categories).post(agenda::category_create),
        )
        .route(
            "/agenda/categories/{id}",
            put(agenda::category_update).delete(agenda::category_delete),
        )
        .route(
            "/agenda/{id}",
            get(agenda::get_one)
                .put(agenda::replace)
                .delete(agenda::remove),
        )
        .route("/agenda/{id}/occurrence", delete(agenda::skip_occurrence))
        .route("/addresses", get(addresses::list).post(addresses::create))
        // Duplicate check of the "Dublettenverhinderer" window before a new address.
        .route("/addresses/duplicates", get(addresses::duplicates))
        // Search form from img/suche/Adresssuche.PNG: criteria as body.
        .route("/addresses/search", post(addresses::search))
        .route(
            "/addresses/{id}",
            get(addresses::get_one)
                .put(addresses::replace)
                .delete(addresses::remove),
        )
        // Article master data — form from img/articles_01..05.PNG
        .route("/articles", get(articles::list).post(articles::create))
        .route("/articles/new", get(articles::create_template))
        // Search form from img/article-search.PNG: criteria as body, plus the
        // saved searches behind the "process" selector.
        .route("/articles/search", post(articles::search))
        .route(
            "/articles/searches",
            get(articles::saved_searches).post(articles::search_save),
        )
        .route("/articles/searches/{id}", delete(articles::search_delete))
        // Stock tab of the form: stock rows on top, devices/batches below.
        .route("/articles/{id}/stock", get(articles::stock))
        .route("/articles/{id}/devices", post(articles::goods_receipt))
        .route(
            "/articles/{id}/devices/{device_id}",
            get(articles::device)
                .put(articles::device_update)
                .delete(articles::device_delete),
        )
        .route(
            "/articles/{id}",
            get(articles::get_one)
                .put(articles::replace)
                .delete(articles::remove),
        )
        // File storage — dialog from img/ablage_01.PNG
        .route("/articles/{id}/files", get(files::list).post(files::upload))
        .route("/articles/{id}/files/history", get(files::history))
        .route("/articles/{id}/files/links", post(files::link))
        .route("/articles/{id}/files/{file_id}", delete(files::remove))
        .route(
            "/articles/{id}/files/{file_id}/content",
            get(files::content),
        )
        // Inspection reports (`Artikel mit Seriennummern → Prüfprotokolle`) and the
        // article image.
        .route("/articles/{id}/reports", get(inspection::list_all))
        .route(
            "/articles/{id}/devices/{device_id}/reports",
            get(inspection::list).post(inspection::upload),
        )
        .route(
            "/articles/{id}/devices/{device_id}/reports/{report_id}",
            delete(inspection::remove),
        )
        .route(
            "/articles/{id}/devices/{device_id}/reports/{report_id}/content",
            get(inspection::content),
        )
        .route(
            "/articles/{id}/image",
            get(inspection::image)
                .post(inspection::image_upload)
                .delete(inspection::image_remove),
        )
        .route(
            "/articles/{id}/image/placeholder",
            put(inspection::image_placeholder),
        )
        // Import manager (`Datei → Aktion → Import-Manager`): the rows arrive
        // mapped onto the fields of the mask.
        .route("/import/articles", post(import::articles))
        .route("/import/addresses", post(import::addresses))
        // Warehouses and bin locations (`img/Admin Menü/Lager und Lagerplätze`).
        .route(
            "/warehouses",
            get(warehouses::list).post(warehouses::create),
        )
        .route(
            "/warehouses/{id}",
            put(warehouses::update).delete(warehouses::remove),
        )
        .route("/warehouses/{id}/assign", post(warehouses::assign))
        .route(
            "/warehouses/{id}/locations",
            get(warehouses::locations).post(warehouses::location_create),
        )
        .route(
            "/warehouses/{id}/locations/{location_id}",
            put(warehouses::location_update).delete(warehouses::location_delete),
        )
        .route(
            "/warehouses/{id}/bins",
            get(warehouses::bins).post(warehouses::bin_create),
        )
        .route(
            "/warehouses/{id}/bins/{bin_id}",
            put(warehouses::bin_update).delete(warehouses::bin_delete),
        )
        // Selection lists of the form and the admin maintenance screens.
        .route(
            "/master-data/article-groups",
            get(master_data::article_groups).post(master_data::article_group_create),
        )
        .route(
            "/master-data/article-groups/{id}",
            put(master_data::article_group_update).delete(master_data::article_group_delete),
        )
        .route(
            "/master-data/number-series",
            get(master_data::number_series_list),
        )
        .route(
            "/master-data/number-series/{key}",
            get(master_data::number_series_get).put(master_data::number_series_update),
        )
        .route(
            "/master-data/number-series/{key}/reset",
            post(master_data::number_series_reset),
        )
        .route("/master-data/languages", get(master_data::languages))
        .route(
            "/master-data/address-index",
            get(master_data::address_index),
        )
        .route("/master-data/warehouses", get(master_data::warehouses))
        .route("/master-data/search-index", get(master_data::search_index))
        // Settings lists of the admin menu (img/Admin Menü): one catalogue, one
        // family of routes.
        .route("/settings", get(settings::areas))
        .route(
            "/settings/{key}",
            get(settings::list).post(settings::create),
        )
        .route(
            "/settings/{key}/{id}",
            put(settings::update).delete(settings::remove),
        )
        // Options of the masks (checkboxes and small entries of a form).
        .route("/prefs", get(settings::prefs))
        .route("/prefs/{key}", put(settings::pref_save))
        // Module registry: which of the four WHERE products share this database.
        // `router_with_modules` mounts the endpoints of a module itself.
        .route("/modules", get(modules::list))
        .route("/installation", get(modules::installation))
        .route(
            "/modules/{id}",
            post(modules::install).delete(modules::remove),
        )
        // Tenants (img/Admin Menü/Mandanten): the companies of the installation. The
        // two option blocks of the window are settings below `mandanten.` at `/prefs`.
        .route("/tenants", get(tenants::list).post(tenants::create))
        .route("/tenants/reference", get(tenants::reference))
        .route(
            "/tenants/{id}",
            get(tenants::get_one)
                .put(tenants::replace)
                .delete(tenants::remove),
        )
        // User administration (img/Admin Menü/Benutzerverwaltung): accounts, their
        // permission groups and the rights tree of the group dialog.
        .route("/users", get(users::list).post(users::create))
        .route(
            "/users/{id}",
            get(users::get_one)
                .put(users::replace)
                .delete(users::remove),
        )
        .route(
            "/users/{id}/groups/{group}",
            put(users::group_add).delete(users::group_unassign),
        )
        .route(
            "/user-groups",
            get(users::group_list).post(users::group_create),
        )
        .route(
            "/user-groups/{id}",
            get(users::group_get)
                .put(users::group_replace)
                .delete(users::group_remove),
        )
        .route("/user-groups/{id}/copy", post(users::group_copy))
        .route("/permissions", get(users::permissions))
        // Login (img/Admin Menü/Login Logout): account list of the mask, sign in,
        // check the session and sign out.
        .route("/auth/tenants", get(auth::tenants))
        .route("/auth/users", get(auth::users))
        .route("/auth/login", post(auth::login))
        .route("/auth/session", get(auth::session))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/password", post(auth::password))
        .route("/auth/mandant", post(auth::switch_mandant))
}
