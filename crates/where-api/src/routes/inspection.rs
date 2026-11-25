//! Inspection reports and the article image.
//!
//! Both hang on one article, so they live next to the article routes: the protocols
//! under `/articles/{id}/devices/{device_id}/reports`, the picture under
//! `/articles/{id}/image`. The bytes come from the same content-addressed store as
//! the file storage; a download therefore has to pass `content` like every other
//! API call.

use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use where_domain::{ArticleImage, InspectionReport};

use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::{AppState, State};

/// Same ceiling as the file storage (`MAX_UPLOAD_BYTES`, 20 MiB by default).
fn max_upload_bytes() -> usize {
    std::env::var("MAX_UPLOAD_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20 * 1024 * 1024)
}

/// One uploaded field of the multipart form.
struct Feld {
    name: String,
    dateiname: Option<String>,
    mime: Option<String>,
    bytes: Vec<u8>,
}

/// Reads the fields that are used here; everything else is dropped.
async fn felder(mut multipart: Multipart) -> Result<Vec<Feld>, ApiError> {
    let mut out = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError(where_domain::DomainError::Validation(e.to_string())))?
    {
        let name = field.name().unwrap_or_default().to_string();
        let dateiname = field.file_name().map(str::to_string);
        let mime = field.content_type().map(str::to_string);
        let bytes = field
            .bytes()
            .await
            .map_err(|e| ApiError(where_domain::DomainError::Validation(e.to_string())))?;
        out.push(Feld {
            name,
            dateiname,
            mime,
            bytes: bytes.to_vec(),
        });
    }
    Ok(out)
}

/// Takes `datei` (also as `file`) and reports a missing field in plain words.
fn datei(felder: &[Feld]) -> Result<&Feld, ApiError> {
    felder
        .iter()
        .find(|f| (f.name == "datei" || f.name == "file") && !f.bytes.is_empty())
        .ok_or_else(|| {
            ApiError(where_domain::DomainError::Validation(
                "Es wurde keine Datei übertragen.".into(),
            ))
        })
}

fn text(felder: &[Feld], name: &str) -> Option<String> {
    felder
        .iter()
        .find(|f| f.name == name)
        .map(|f| String::from_utf8_lossy(&f.bytes).trim().to_string())
        .filter(|wert| !wert.is_empty())
}

fn limit_pruefen(bytes: &[u8]) -> Result<(), ApiError> {
    let limit = max_upload_bytes();
    if bytes.len() > limit {
        return Err(ApiError(where_domain::DomainError::Validation(format!(
            "Die Datei ist größer als {:.0} MB.",
            limit as f64 / (1024.0 * 1024.0)
        ))));
    }
    Ok(())
}

/// Inspection reports of one device.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/devices/{device_id}/reports",
    tag = "files",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device (\"Seriennummern\")")
    ),
    responses((status = 200, description = "Protokolle", body = Vec<InspectionReport>))
)]
pub async fn list(
    State(state): State<AppState>,
    Path((article_id, device_id)): Path<(i32, i32)>,
) -> Result<Json<Vec<InspectionReport>>, ApiError> {
    Ok(Json(
        state
            .inspections
            .list_for_device(article_id, device_id)
            .await?,
    ))
}

/// All inspection reports of one article (for the overview over all devices).
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/reports",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    responses((status = 200, description = "Protokolle", body = Vec<InspectionReport>))
)]
pub async fn list_all(
    State(state): State<AppState>,
    Path(article_id): Path<i32>,
) -> Result<Json<Vec<InspectionReport>>, ApiError> {
    Ok(Json(state.inspections.list(article_id).await?))
}

/// Uploads an inspection report (PDF) for a device.
///
/// Fields: `datei` (required, PDF), `titel` (optional; otherwise the file name),
/// `benutzer` (optional).
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/devices/{device_id}/reports",
    tag = "files",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device (\"Seriennummern\")")
    ),
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Gespeichert", body = InspectionReport),
        (status = 413, description = "File too large"),
        (status = 422, description = "Kein PDF oder Gerät fehlt")
    )
)]
pub async fn upload(
    State(state): State<AppState>,
    Path((article_id, device_id)): Path<(i32, i32)>,
    multipart: Multipart,
) -> Result<(StatusCode, Json<InspectionReport>), ApiError> {
    // The device must belong to this article — otherwise the report would hang on a
    // foreign piece.
    let device = state.stock.device(article_id, device_id).await?;

    let felder = felder(multipart).await?;
    let datei = datei(&felder)?;
    limit_pruefen(&datei.bytes)?;
    if !where_domain::inspection::is_pdf(&datei.bytes) {
        return Err(ApiError(where_domain::DomainError::Validation(
            "Prüfprotokolle müssen PDF-Dateien sein.".into(),
        )));
    }

    let dateiname = datei
        .dateiname
        .clone()
        .unwrap_or_else(|| "pruefprotokoll.pdf".to_string());
    let titel = text(&felder, "titel").unwrap_or_else(|| {
        dateiname
            .rsplit_once('.')
            .map(|(vorne, _)| vorne.to_string())
            .unwrap_or_else(|| dateiname.clone())
    });
    let benutzer = text(&felder, "benutzer");

    let report = state
        .inspections
        .upload(where_persistence::NewReport {
            article_id,
            device_id,
            binding: where_persistence::DeviceBinding {
                serial_no: device.serial_no.clone(),
                inventory_no: device.inventory_no.clone(),
            },
            title: &titel,
            mime: datei.mime.as_deref(),
            content: &datei.bytes,
            user: benutzer.as_deref(),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(report)))
}

/// Delivers the PDF for viewing or downloading.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/devices/{device_id}/reports/{report_id}/content",
    tag = "files",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device"),
        ("report_id" = i32, Path, description = "Protokoll")
    ),
    responses(
        (status = 200, description = "PDF"),
        (status = 404, description = "Not found")
    )
)]
pub async fn content(
    State(state): State<AppState>,
    Path((article_id, device_id, report_id)): Path<(i32, i32, i32)>,
) -> Result<Response, ApiError> {
    let (report, bytes) = state.inspections.content(report_id).await?;
    if report.article_id != article_id || report.device_id != device_id {
        return Err(ApiError(where_domain::DomainError::NotFound {
            entity: "inspection_report",
            id: report_id,
        }));
    }

    let mime = report
        .mime
        .clone()
        .unwrap_or_else(|| "application/pdf".to_string());
    let dateiname = if report.title.to_ascii_lowercase().ends_with(".pdf") {
        report.title.clone()
    } else {
        format!("{}.pdf", report.title)
    };

    let mut response = (StatusCode::OK, Body::from(bytes)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.parse()
            .unwrap_or(header::HeaderValue::from_static("application/pdf")),
    );
    // `inline`: the browser shows the PDF in the frame instead of saving it.
    let disposition = format!("inline; filename*=UTF-8''{}", percent_encode(&dateiname));
    if let Ok(value) = disposition.parse() {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

/// Removes an inspection report (soft: the row stays for the history).
#[utoipa::path(
    delete,
    path = "/api/v1/articles/{id}/devices/{device_id}/reports/{report_id}",
    tag = "files",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("device_id" = i32, Path, description = "Device"),
        ("report_id" = i32, Path, description = "Protokoll")
    ),
    responses(
        (status = 200, description = "Entfernt", body = DeletedResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path((article_id, device_id, report_id)): Path<(i32, i32, i32)>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let report = state.inspections.get(report_id).await?;
    if report.article_id != article_id || report.device_id != device_id {
        return Err(ApiError(where_domain::DomainError::NotFound {
            entity: "inspection_report",
            id: report_id,
        }));
    }
    state.inspections.delete(report_id).await?;
    Ok(Json(DeletedResponse { deleted: true }))
}

// ------------------------------------------------------------- Article image

/// The article image of an article.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/image",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    responses((status = 200, description = "Bild", body = ArticleImage))
)]
pub async fn image(
    State(state): State<AppState>,
    Path(article_id): Path<i32>,
) -> Result<Json<ArticleImage>, ApiError> {
    Ok(Json(state.files.image(article_id).await?))
}

/// Uploads a new article image (replaces the previous one).
///
/// Fields: `datei` (required, image), `benutzer` (optional).
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/image",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Gespeichert", body = ArticleImage),
        (status = 413, description = "File too large"),
        (status = 422, description = "Kein Bild")
    )
)]
pub async fn image_upload(
    State(state): State<AppState>,
    Path(article_id): Path<i32>,
    multipart: Multipart,
) -> Result<(StatusCode, Json<ArticleImage>), ApiError> {
    state.article.get(article_id).await?;

    let felder = felder(multipart).await?;
    let datei = datei(&felder)?;
    limit_pruefen(&datei.bytes)?;

    let name = datei
        .dateiname
        .clone()
        .unwrap_or_else(|| "artikelbild".to_string());
    let benutzer = text(&felder, "benutzer");

    let bild = state
        .files
        .image_upload(
            article_id,
            &name,
            datei.mime.as_deref(),
            &datei.bytes,
            benutzer.as_deref(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(bild)))
}

/// Switches between the own image and the animated placeholder.
#[utoipa::path(
    put,
    path = "/api/v1/articles/{id}/image/placeholder",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    responses((status = 200, description = "Gespeichert", body = ArticleImage))
)]
pub async fn image_placeholder(
    State(state): State<AppState>,
    Path(article_id): Path<i32>,
    Json(body): Json<PlaceholderInput>,
) -> Result<Json<ArticleImage>, ApiError> {
    state.article.get(article_id).await?;
    Ok(Json(
        state
            .files
            .image_placeholder(article_id, body.placeholder)
            .await?,
    ))
}

/// Removes the uploaded article image.
#[utoipa::path(
    delete,
    path = "/api/v1/articles/{id}/image",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    responses((status = 200, description = "Entfernt", body = ArticleImage))
)]
pub async fn image_remove(
    State(state): State<AppState>,
    Path(article_id): Path<i32>,
) -> Result<Json<ArticleImage>, ApiError> {
    state.article.get(article_id).await?;
    Ok(Json(state.files.image_delete(article_id, None).await?))
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct PlaceholderInput {
    pub placeholder: bool,
}

/// Minimal percent encoding for the file name in the Content-Disposition header.
fn percent_encode(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}
