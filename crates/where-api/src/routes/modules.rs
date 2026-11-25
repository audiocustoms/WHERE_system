//! Module registry of this installation.
//!
//! `GET` reports which of the four products share this database. `POST`
//! installs a module into it and `DELETE` removes it again. Installing a module
//! after the core simply joins the existing database; installing the core after
//! a module reuses the module's database instead of creating another one — the
//! decision lives in `where_modules::resolve_install`.

use axum::Extension;
use axum::Json;
use axum::extract::{Path, State as AxumState};
use serde::Serialize;
use utoipa::ToSchema;
use where_domain::{DomainError, SessionUser};
use where_modules::{InstallOutcome, ModuleError, ModuleId, ModuleSnapshot};

use crate::error::ApiError;
use crate::state::AppState;

fn module_error(error: ModuleError) -> ApiError {
    match error {
        ModuleError::UnknownModule(key) | ModuleError::NotInstalled(key) => {
            DomainError::NotFoundKey {
                entity: "module",
                key,
            }
        }
        ModuleError::NotAModule(key) => DomainError::Validation(format!(
            "{key} ist die Kernanwendung und wird nicht als Modul installiert."
        )),
        ModuleError::CoreCannotBeRemoved => {
            DomainError::Conflict("Die Kernanwendung lässt sich nicht entfernen.".to_string())
        }
        ModuleError::Database(error) => DomainError::Internal(error.to_string()),
    }
    .into()
}

/// The four products of the installation and whether they are installed.
#[utoipa::path(
    get,
    path = "/api/v1/modules",
    tag = "module",
    responses((status = 200, body = ModuleSnapshot))
)]
pub async fn list(AxumState(state): AxumState<AppState>) -> Result<Json<ModuleSnapshot>, ApiError> {
    let snapshot = where_modules::snapshot(&state.pool)
        .await
        .map_err(module_error)?;
    state.modules.replace(snapshot.clone());
    Ok(Json(snapshot))
}

/// Which product this installation runs.
///
/// The login mask shows the name before a session exists, so this one field of the
/// registry is readable without signing in.
#[derive(Debug, Serialize, ToSchema)]
pub struct InstallationInfo {
    pub product_name: String,
}

/// The name of the product, for the login mask and the window title.
#[utoipa::path(
    get,
    path = "/api/v1/installation",
    tag = "module",
    responses((status = 200, body = InstallationInfo))
)]
pub async fn installation(AxumState(state): AxumState<AppState>) -> Json<InstallationInfo> {
    let snapshot = state.modules.snapshot();
    let product_name = snapshot
        .product_name()
        .unwrap_or_else(|| ModuleId::Stuff.manifest().product_name)
        .to_string();
    Json(InstallationInfo { product_name })
}

/// Installs a module into the installation's database.
#[utoipa::path(
    post,
    path = "/api/v1/modules/{id}",
    tag = "module",
    params(("id" = String, Path, description = "`go`, `people` oder `money`")),
    responses(
        (status = 200, body = InstallOutcome),
        (status = 404, description = "Unbekanntes Modul", body = crate::error::Problem)
    )
)]
pub async fn install(
    AxumState(state): AxumState<AppState>,
    Path(id): Path<String>,
    Extension(user): Extension<SessionUser>,
) -> Result<Json<InstallOutcome>, ApiError> {
    let module = ModuleId::from_key(&id)
        .ok_or_else(|| module_error(ModuleError::UnknownModule(id.clone())))?;
    if module == ModuleId::Stuff {
        return Err(module_error(ModuleError::NotAModule(id)));
    }
    let outcome = where_modules::install(&state.pool, module, Some(&user.user_name))
        .await
        .map_err(module_error)?;
    if let Ok(snapshot) = where_modules::snapshot(&state.pool).await {
        state.modules.replace(snapshot);
    }
    Ok(Json(outcome))
}

/// Removes a module from the registry; its data stays in the database.
#[utoipa::path(
    delete,
    path = "/api/v1/modules/{id}",
    tag = "module",
    params(("id" = String, Path, description = "`go`, `people` oder `money`")),
    responses(
        (status = 200, body = ModuleSnapshot),
        (status = 404, description = "Modul ist nicht installiert", body = crate::error::Problem),
        (status = 409, description = "Die Kernanwendung bleibt installiert", body = crate::error::Problem)
    )
)]
pub async fn remove(
    AxumState(state): AxumState<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ModuleSnapshot>, ApiError> {
    let module =
        ModuleId::from_key(&id).ok_or_else(|| module_error(ModuleError::UnknownModule(id)))?;
    where_modules::remove(&state.pool, module)
        .await
        .map_err(module_error)?;
    let snapshot = where_modules::snapshot(&state.pool)
        .await
        .map_err(module_error)?;
    state.modules.replace(snapshot.clone());
    Ok(Json(snapshot))
}
