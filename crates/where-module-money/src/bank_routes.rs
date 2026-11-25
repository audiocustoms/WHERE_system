//! Bank list (`img/Admin Menü/Bank - Swift BIC Liste`).
//!
//! The list can be edited by hand and synchronised with an online source. The
//! download runs here rather than in the browser: the sources (the German "BLZ-Datei",
//! open BIC lists) send no CORS header, and the server can cache one request for
//! every workstation.

use std::time::Duration;

use axum::Json;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use where_api::ApiError;
use where_api::routes::addresses::DeletedResponse;
use where_api::state::{AppState, State};
use where_domain::DomainError;

use crate::bank::{Bank, BankInput, BankSyncReport, BankSyncRequest, parse_banks};
use crate::banks_repo::BankRepository;

fn banks(state: &AppState) -> BankRepository {
    BankRepository::new(state.pool.clone())
}

/// A page of the list plus the number of matches, so the mask can show "x von y".
#[derive(Debug, Serialize, ToSchema)]
pub struct BankListResponse {
    pub rows: Vec<Bank>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct BankParams {
    /// Search over code, name, town, sort code and country.
    pub q: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Rows of the bank list.
#[utoipa::path(
    get,
    path = "/api/v1/banks",
    tag = "banken",
    params(
        ("q" = Option<String>, Query, description = "Suche über Code, Name, Ort, BLZ, Land"),
        ("limit" = Option<i64>, Query, description = "Zeilen, höchstens 20000"),
        ("offset" = Option<i64>, Query, description = "Startzeile")
    ),
    responses((status = 200, body = BankListResponse))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<BankParams>,
) -> Result<Json<BankListResponse>, ApiError> {
    let page = banks(&state)
        .list(
            params.q.as_deref(),
            params.limit.unwrap_or(100),
            params.offset.unwrap_or(0),
        )
        .await?;
    Ok(Json(BankListResponse {
        rows: page.rows,
        total: page.total,
    }))
}

/// Adds a bank by hand.
#[utoipa::path(
    post,
    path = "/api/v1/banks",
    tag = "banken",
    request_body = BankInput,
    responses(
        (status = 201, body = Bank),
        (status = 422, description = "Invalid code or name", body = where_api::error::Problem)
    )
)]
pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<BankInput>,
) -> Result<(StatusCode, Json<Bank>), ApiError> {
    let bank = banks(&state).create(input).await?;
    Ok((StatusCode::CREATED, Json(bank)))
}

/// Changes a bank.
#[utoipa::path(
    put,
    path = "/api/v1/banks/{id}",
    tag = "banken",
    params(("id" = i64, Path, description = "Key of the row")),
    request_body = BankInput,
    responses(
        (status = 200, body = Bank),
        (status = 404, description = "Unknown row", body = where_api::error::Problem),
        (status = 422, description = "Invalid code or name", body = where_api::error::Problem)
    )
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(input): Json<BankInput>,
) -> Result<Json<Bank>, ApiError> {
    Ok(Json(banks(&state).update(id, input).await?))
}

/// Removes a bank. An address that still carries the code keeps it readable — the
/// legacy schema has no foreign keys.
#[utoipa::path(
    delete,
    path = "/api/v1/banks/{id}",
    tag = "banken",
    params(("id" = i64, Path, description = "Key of the row")),
    responses(
        (status = 200, body = DeletedResponse),
        (status = 404, description = "Unknown row", body = where_api::error::Problem)
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeletedResponse>, ApiError> {
    banks(&state).delete(id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

/// Fetches the list; a page is followed to its CSV file, a file is taken as it is.
///
/// The Bundesbank publishes the bank file behind a page whose download link carries a
/// new hash with every release, so the page is the stable part: the newest
/// `…-csv-data.csv` link on it is the list. The bytes decide the encoding — the file
/// says UTF-8 in its header but carries ISO-8859-1.
async fn fetch_list(url: &str) -> Result<(String, String), DomainError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .user_agent(concat!("WHERE/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| DomainError::Upstream(format!("HTTP-Client: {error}")))?;
    let (final_url, body, is_html) = get(&client, url).await?;
    if !is_html {
        return Ok((final_url, decode_text(&body)));
    }
    let page = decode_text(&body);
    let link = find_csv_link(&page).ok_or_else(|| {
        DomainError::Upstream(format!(
            "{url}: auf der Seite steht kein Verweis auf eine CSV-Datei."
        ))
    })?;
    let file_url = absolute(url, &link);
    let (final_url, body, _) = get(&client, &file_url).await?;
    Ok((final_url, decode_text(&body)))
}

/// One request: final URL, body and whether it is a page.
async fn get(client: &reqwest::Client, url: &str) -> Result<(String, Vec<u8>, bool), DomainError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| DomainError::Upstream(format!("{url}: {error}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(DomainError::Upstream(format!(
            "{url}: der Server antwortet mit {status}"
        )));
    }
    let final_url = response.url().to_string();
    let is_html = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase().contains("html"))
        .unwrap_or(false);
    let body = response
        .bytes()
        .await
        .map_err(|error| DomainError::Upstream(format!("{url}: {error}")))?
        .to_vec();
    Ok((final_url, body, is_html))
}

/// The newest bank list behind a download page: `…-csv-data.csv`, otherwise any CSV.
fn find_csv_link(page: &str) -> Option<String> {
    for needle in ["blz-aktuell-csv-data.csv", "-csv-data.csv", ".csv"] {
        let mut from = 0;
        while let Some(position) = page[from..].find(needle) {
            let at = from + position;
            let end = at + needle.len();
            // The link starts before the file name, behind an `href="`.
            let start = page[..at].rfind("href=\"").map(|index| index + 6)?;
            let value = page[start..end].trim();
            if value.contains(needle) && !value.contains(' ') {
                return Some(value.to_string());
            }
            from = end;
        }
    }
    None
}

/// Makes a link of the page absolute.
fn absolute(base: &str, link: &str) -> String {
    if link.starts_with("http://") || link.starts_with("https://") {
        return link.to_string();
    }
    let scheme_end = base.find("://").map(|index| index + 3).unwrap_or(0);
    let host_end = base[scheme_end..]
        .find('/')
        .map(|index| scheme_end + index)
        .unwrap_or(base.len());
    if link.starts_with('/') {
        format!("{}{}", &base[..host_end], link)
    } else {
        format!("{}/{}", &base[..host_end], link)
    }
}

/// The page WHERE reads the bank list from; the link to the file behind it changes
/// with every quarterly release, the page does not.
pub const DEFAULT_SOURCE: &str = "https://www.bundesbank.de/de/aufgaben/unbarer-zahlungsverkehr/serviceangebot/bankleitzahlen/download-bankleitzahlen-602592";

/// UTF-8 if it parses, otherwise Latin-1 — the Bundesbank file carries umlauts in it.
fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|byte| *byte as char).collect(),
    }
}

/// Where the built-in source lives, so the window can offer it.
#[derive(Debug, Serialize, ToSchema)]
pub struct BankSource {
    pub url: String,
    /// Plain-language note about the source.
    pub note: String,
}

/// The built-in source for `Update`.
#[utoipa::path(
    get,
    path = "/api/v1/banks/source",
    tag = "banken",
    responses((status = 200, body = BankSource))
)]
pub async fn source() -> Json<BankSource> {
    Json(BankSource {
        url: DEFAULT_SOURCE.to_string(),
        note: "Deutsche Bundesbank, Bankleitzahlen-Download: die Seite wird gelesen und die \
               aktuelle CSV-Datei geladen."
            .to_string(),
    })
}

/// Synchronises the list: from the URL of the request, from the list in the body, or
/// — without both — from the built-in source.
#[utoipa::path(
    post,
    path = "/api/v1/banks/sync",
    tag = "banken",
    request_body = BankSyncRequest,
    responses(
        (status = 200, body = BankSyncReport),
        (status = 502, description = "The source could not be read", body = where_api::error::Problem)
    )
)]
pub async fn sync(
    State(state): State<AppState>,
    Json(request): Json<BankSyncRequest>,
) -> Result<Json<BankSyncReport>, ApiError> {
    let url = request
        .url
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());
    let eingabe = request
        .csv
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty());

    let (text, source) = match (url, eingabe) {
        (Some(url), _) => {
            let (final_url, text) = fetch_list(url).await?;
            (text, final_url)
        }
        (None, Some(csv)) => (csv.to_string(), "Eingabe".to_string()),
        (None, None) => {
            // Without a source the stored one is used — one click on `Update` is
            // enough.
            let (final_url, text) = fetch_list(DEFAULT_SOURCE).await?;
            (text, final_url)
        }
    };

    let (rows, skipped) = parse_banks(&text);
    let mut report = banks(&state).upsert(rows).await?;
    report.source = source;
    report.skipped = skipped;
    Ok(Json(report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin1_bytes_become_text() {
        // "Rückliefern" in ISO-8859-1.
        let bytes = b"R\xfcckliefern";
        assert_eq!(decode_text(bytes), "Rückliefern");
        assert_eq!(decode_text("Bühne".as_bytes()), "Bühne");
    }
}
