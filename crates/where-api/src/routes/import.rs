//! Import manager — `Datei → Aktion → Import-Manager` (`img/import/*.PNG`).
//!
//! The window reads the CSV in the browser: it decodes the file with the chosen
//! code page, finds the separator and lets the operator drag the fields of the
//! mask onto the columns. Only columns that carry a field are sent here, already
//! shaped like the mask they belong to — so this module does what
//! [`super::articles::create`] and [`super::addresses::create`] do, once per row.
//!
//! One row at a time: a row that does not validate is reported with its number
//! and the import continues, instead of losing the whole file to one typo.

use std::collections::HashMap;

use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_domain::{AddressInput, Article, DomainError};

use crate::dto::AddressInputDto;
use crate::error::ApiError;
use crate::state::{AppState, State};

/// Number series used when the file carries no number of its own.
const CUSTOMER_SERIES: &str = "customer";
const SUPPLIER_SERIES: &str = "supplier";

/// One row that could not be imported.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ImportFailure {
    /// Position of the row in the request, counted from 1 — the window maps it
    /// back onto the line of the file.
    pub row: usize,
    /// Why the row was rejected (the message the mask would show as well).
    pub message: String,
}

/// Result of an import run.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ImportReport {
    /// Rows that were stored.
    pub created: usize,
    /// Rows that were rejected, in file order.
    pub failed: Vec<ImportFailure>,
}

/// One row of an article import.
///
/// `article` is the record as the mask would send it; `warengruppe` carries the
/// name the file names in its `Warengruppe` column — the mask picks the group from
/// the tree instead, so the name is resolved (and created) here.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ArticleImportRow {
    pub article: Article,
    /// Name of the article group. A group of this name is created when it is missing.
    #[serde(default)]
    pub warengruppe: Option<String>,
}

/// Rows of an article import, already mapped onto the fields of the mask.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ArticleImportInput {
    pub rows: Vec<ArticleImportRow>,
}

/// Rows of an address import, already mapped onto the fields of the mask.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddressImportInput {
    pub rows: Vec<AddressInputDto>,
}

/// Stores the rows of an article import, field for field as the mask would.
///
/// A `Warengruppe` named by the file is looked up by name; a group that does not
/// exist yet is created, so a file from another system brings its own groups.
#[utoipa::path(
    post,
    path = "/api/v1/import/articles",
    tag = "articles",
    request_body = ArticleImportInput,
    responses(
        (status = 200, description = "Number of stored and rejected rows", body = ImportReport),
        (status = 422, description = "The rows could not be read")
    )
)]
pub async fn articles(
    State(state): State<AppState>,
    Json(body): Json<ArticleImportInput>,
) -> Result<Json<ImportReport>, ApiError> {
    let mut report = ImportReport {
        created: 0,
        failed: Vec::new(),
    };
    // Groups of this run, keyed by their lowercase name: a file names the same group
    // in many rows, and it is only looked up (or created) once.
    let mut gruppen: HashMap<String, i32> = HashMap::new();
    for (index, row) in body.rows.into_iter().enumerate() {
        let mut article = row.article;
        // A row of the file never overwrites an existing article: `id` comes from
        // the template the window sent along, not from the operator.
        article.id = None;
        let outcome = async {
            if let Some(name) = row
                .warengruppe
                .as_deref()
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                let id = gruppe_finden(&state, &mut gruppen, name).await?;
                // The column decides the group; a selection made in the form has no
                // business in an import row.
                article.article_groups = vec![id];
            }
            state.article.create(&article).await
        }
        .await;
        match outcome {
            Ok(_) => report.created += 1,
            Err(error) => report.failed.push(ImportFailure {
                row: index + 1,
                message: error.to_string(),
            }),
        }
    }
    Ok(Json(report))
}

/// Finds the group of that name or creates it — the import is allowed to grow the
/// tree, an operator without the admin right is not.
async fn gruppe_finden(
    state: &AppState,
    bekannt: &mut HashMap<String, i32>,
    name: &str,
) -> Result<i32, DomainError> {
    let schluessel = name.to_lowercase();
    if let Some(id) = bekannt.get(&schluessel) {
        return Ok(*id);
    }
    let id = match state.article_groups.find_by_name(name).await? {
        Some(eintrag) => eintrag.id,
        None => match state.article_groups.create(name, None).await {
            Ok(eintrag) => eintrag.id,
            // Two runs at the same time: the other one was faster.
            Err(DomainError::Conflict(_)) => {
                state
                    .article_groups
                    .find_by_name(name)
                    .await?
                    .ok_or_else(|| {
                        DomainError::Conflict(format!(
                            "Warengruppe '{name}' konnte nicht angelegt werden"
                        ))
                    })?
                    .id
            }
            Err(error) => return Err(error),
        },
    };
    bekannt.insert(schluessel, id);
    Ok(id)
}

/// Stores the rows of an address import.
///
/// Two things the mask does by hand are done here: an address without any role
/// becomes a customer (a file of addresses is a customer file), and a missing
/// customer or supplier number is taken from its number series. That way a file
/// with nothing but a name can be imported.
#[utoipa::path(
    post,
    path = "/api/v1/import/addresses",
    tag = "addresses",
    request_body = AddressImportInput,
    responses(
        (status = 200, description = "Number of stored and rejected rows", body = ImportReport),
        (status = 422, description = "The rows could not be read")
    )
)]
pub async fn addresses(
    State(state): State<AppState>,
    Json(body): Json<AddressImportInput>,
) -> Result<Json<ImportReport>, ApiError> {
    let mut report = ImportReport {
        created: 0,
        failed: Vec::new(),
    };
    for (index, dto) in body.rows.into_iter().enumerate() {
        let mut input: AddressInput = dto.into();
        if !input.is_customer && !input.is_supplier && !input.is_interested {
            input.is_customer = true;
        }
        let outcome = async {
            if (input.is_customer || input.is_interested) && input.customer_no.is_none() {
                input.customer_no = Some(next_number(&state, CUSTOMER_SERIES).await?);
            }
            if input.is_supplier && input.supplier_no.is_none() {
                input.supplier_no = Some(next_number(&state, SUPPLIER_SERIES).await?);
            }
            state.addresses.create(&input).await
        }
        .await;
        match outcome {
            Ok(_) => report.created += 1,
            Err(error) => report.failed.push(ImportFailure {
                row: index + 1,
                message: error.to_string(),
            }),
        }
    }
    Ok(Json(report))
}

/// Takes the next number of a series — the same call the number dialog makes.
async fn next_number(state: &AppState, key: &str) -> Result<i32, DomainError> {
    let reserved = state.number_series.reserve(key).await?;
    i32::try_from(reserved.value)
        .map_err(|_| DomainError::Conflict(format!("the number series '{key}' is exhausted")))
}
