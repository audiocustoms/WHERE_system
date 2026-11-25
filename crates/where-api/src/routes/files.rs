use crate::state::State;
use axum::Json;
use axum::body::Body;
use axum::extract::{Multipart, Path};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use where_domain::ArticleFile;

use crate::dto::LinkInput;
use crate::error::ApiError;
use crate::routes::addresses::DeletedResponse;
use crate::state::AppState;

/// Default upload limit when the limit is not configured (20 MiB).
fn max_upload_bytes() -> usize {
    std::env::var("MAX_UPLOAD_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20 * 1024 * 1024)
}

/// Current file storage entries of an article.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/files",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    responses((status = 200, description = "Entries", body = Vec<ArticleFile>))
)]
pub async fn list(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<ArticleFile>>, ApiError> {
    Ok(Json(state.files.list(id).await?))
}

/// All versions of an article ("version overview").
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/files/history",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    responses((status = 200, description = "All versions", body = Vec<ArticleFile>))
)]
pub async fn history(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<ArticleFile>>, ApiError> {
    Ok(Json(state.files.history(id).await?))
}

/// Uploads a file (multipart/form-data).
///
/// Fields: `datei` (required), `ordner` (optional), `benutzer` (optional).
/// An existing name in the same folder becomes the next version.
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/files",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "Gespeichert", body = ArticleFile),
        (status = 413, description = "File too large"),
        (status = 422, description = "Invalid input")
    )
)]
pub async fn upload(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ArticleFile>), ApiError> {
    // Does the article exist? Otherwise the foreign key raises a 500 later.
    state.article.get(id).await?;

    let mut folder = String::new();
    let mut user: Option<String> = None;
    let mut file: Option<(String, Option<String>, Vec<u8>)> = None;
    let limit = max_upload_bytes();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError(where_domain::DomainError::Validation(e.to_string())))?
    {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "folder" => {
                folder = field
                    .text()
                    .await
                    .map_err(|e| ApiError(where_domain::DomainError::Validation(e.to_string())))?
            }
            "benutzer" => {
                user =
                    Some(field.text().await.map_err(|e| {
                        ApiError(where_domain::DomainError::Validation(e.to_string()))
                    })?)
            }
            "datei" => {
                let file_name = field.file_name().unwrap_or("datei").to_string();
                let mime = field.content_type().map(|m| m.to_string());
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError(where_domain::DomainError::Validation(e.to_string())))?;
                if bytes.len() > limit {
                    return Err(ApiError(where_domain::DomainError::Validation(format!(
                        "File exceeds the limit of {limit} bytes"
                    ))));
                }
                file = Some((file_name, mime, bytes.to_vec()));
            }
            _ => {
                // Skip unknown fields so forms stay extensible.
                let _ = field.bytes().await;
            }
        }
    }

    let (name, mime, bytes) = file.ok_or_else(|| {
        ApiError(where_domain::DomainError::Validation(
            "Feld 'datei' fehlt".into(),
        ))
    })?;

    let saved = state
        .files
        .upload(id, &folder, &name, mime.as_deref(), &bytes, user.as_deref())
        .await?;
    Ok((StatusCode::CREATED, Json(saved)))
}

/// "Add link": entry that points at a URL.
#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/files/links",
    tag = "files",
    params(("id" = i32, Path, description = "Article primary key")),
    request_body = LinkInput,
    responses((status = 201, description = "Linked", body = ArticleFile))
)]
pub async fn link(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<LinkInput>,
) -> Result<(StatusCode, Json<ArticleFile>), ApiError> {
    state.article.get(id).await?;
    let entry = state
        .files
        .add_link(
            id,
            &body.folder,
            &body.name,
            &body.target,
            body.user.as_deref(),
        )
        .await?;
    Ok((StatusCode::CREATED, Json(entry)))
}

/// Downloads the content of a file.
#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/files/{file_id}/content",
    tag = "files",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("file_id" = i32, Path, description = "Entry id")
    ),
    responses(
        (status = 200, description = "File content"),
        (status = 404, description = "Not found")
    )
)]
pub async fn content(
    State(state): State<AppState>,
    Path((article_id, file_id)): Path<(i32, i32)>,
) -> Result<Response, ApiError> {
    let (file, bytes) = state.files.content(file_id).await?;
    if file.article_id != article_id {
        return Err(ApiError(where_domain::DomainError::NotFound {
            entity: "article_file",
            id: file_id,
        }));
    }

    let mime = file
        .mime
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        percent_encode(&file.name)
    );

    let mut response = (StatusCode::OK, Body::from(bytes)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        mime.parse()
            .unwrap_or(header::HeaderValue::from_static("application/octet-stream")),
    );
    if let Ok(value) = disposition.parse() {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    Ok(response)
}

/// Removes a file storage entry (soft delete: the history is kept).
#[utoipa::path(
    delete,
    path = "/api/v1/articles/{id}/files/{datei_id}",
    tag = "files",
    params(
        ("id" = i32, Path, description = "Article primary key"),
        ("file_id" = i32, Path, description = "Eintrags-ID")
    ),
    responses(
        (status = 200, description = "Entfernt", body = DeletedResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    Path((article_id, file_id)): Path<(i32, i32)>,
) -> Result<Json<DeletedResponse>, ApiError> {
    let file = state.files.get(file_id).await?;
    if file.article_id != article_id {
        return Err(ApiError(where_domain::DomainError::NotFound {
            entity: "article_file",
            id: file_id,
        }));
    }
    state.files.delete(file_id, None).await?;
    Ok(Json(DeletedResponse { deleted: true }))
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
