//! Authenticated routes. Both the owner and the publisher may read, publish
//! and download; the principal decides only the recorded version source.

use ahara_lambda_http::{json_body, json_response, Route};
use lambda_http::http::{Method, StatusCode};
use lambda_http::Request;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::Principal;
use crate::domain::{validate_new_version, validate_slug, NewVersionRequest, VersionSummary};
use crate::error::{AppError, AppResult};
use crate::objects::{UploadTarget, DOWNLOAD_EXPIRY_SECONDS};
use crate::{observe, ApiResponse, ApiState};

#[derive(Serialize)]
struct Me<'a> {
    kind: &'a str,
    username: Option<&'a str>,
}

#[derive(Serialize)]
struct CreatedVersionResponse {
    version_id: Uuid,
    number: i32,
    uploads: Vec<FileUpload>,
}

#[derive(Serialize)]
struct FileUpload {
    file_id: Uuid,
    filename: String,
    upload: UploadTarget,
}

#[derive(Serialize)]
struct DownloadResponse {
    url: String,
    filename: String,
    expires_in_seconds: u64,
}

pub async fn dispatch(
    route: &Route<'_>,
    request: &Request,
    state: &ApiState,
    principal: &Principal,
) -> AppResult<Option<ApiResponse>> {
    if route.is_match(Method::GET, "/me")? {
        return me(principal).map(Some);
    }
    if route.is_match(Method::GET, "/pieces")? {
        return observe("pieces.list", principal, list_pieces(state))
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::GET, "/pieces/{slug}")? {
        let slug = params.require("slug")?;
        return observe("pieces.detail", principal, piece_detail(state, slug))
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::POST, "/pieces/{slug}/versions")? {
        let slug = params.require("slug")?;
        let future = create_version(state, principal, slug, request);
        return observe("versions.create", principal, future)
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::POST, "/versions/{version_id}/commit")? {
        let version_id = params.parse::<Uuid>("version_id")?;
        let future = commit_version(state, version_id);
        return observe("versions.commit", principal, future)
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::GET, "/files/{file_id}/download")? {
        let file_id = params.parse::<Uuid>("file_id")?;
        let future = download(state, file_id);
        return observe("files.download", principal, future).await.map(Some);
    }
    Ok(None)
}

fn me(principal: &Principal) -> AppResult<ApiResponse> {
    let username = match principal {
        Principal::Owner { username } => Some(username.as_str()),
        Principal::Publisher { .. } => None,
    };
    ok(&Me {
        kind: principal.kind(),
        username,
    })
}

async fn list_pieces(state: &ApiState) -> AppResult<ApiResponse> {
    let pieces = state.store.list_pieces().await?;
    ok(&serde_json::json!({ "pieces": pieces }))
}

async fn piece_detail(state: &ApiState, slug: &str) -> AppResult<ApiResponse> {
    validate_slug(slug)?;
    let detail = state
        .store
        .piece_detail(slug)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("piece {slug}")))?;
    ok(&detail)
}

async fn create_version(
    state: &ApiState,
    principal: &Principal,
    slug: &str,
    request: &Request,
) -> AppResult<ApiResponse> {
    let body: NewVersionRequest = json_body(request)?;
    let version = validate_new_version(slug, body, principal.source())?;
    let created = state.store.create_version(&version).await?;
    let mut uploads = Vec::with_capacity(created.files.len());
    for file in created.files {
        let upload = state
            .objects
            .upload_target(&file.object_key, &file.content_type)
            .await?;
        uploads.push(FileUpload {
            file_id: file.file_id,
            filename: file.filename,
            upload,
        });
    }
    let response = CreatedVersionResponse {
        version_id: created.version_id,
        number: created.number,
        uploads,
    };
    Ok(json_response(StatusCode::CREATED, &response)?)
}

async fn commit_version(state: &ApiState, version_id: Uuid) -> AppResult<ApiResponse> {
    let pending = state
        .store
        .pending_version(version_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("version {version_id}")))?;
    if pending.ready {
        return Err(AppError::Conflict(
            "version is already committed".to_string(),
        ));
    }
    for expected in &pending.files {
        let actual = state.objects.object_size(&expected.object_key).await?;
        if actual != Some(expected.size_bytes) {
            return Err(AppError::Validation(format!(
                "file {} is missing or has the wrong size",
                expected.object_key.rsplit('/').next().unwrap_or_default()
            )));
        }
    }
    let version: VersionSummary = state.store.mark_ready(version_id).await?;
    ok(&version)
}

async fn download(state: &ApiState, file_id: Uuid) -> AppResult<ApiResponse> {
    let file = state
        .store
        .ready_file(file_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("file {file_id}")))?;
    let url = state
        .objects
        .download_url(&file.object_key, &file.content_type, &file.filename)
        .await?;
    ok(&DownloadResponse {
        url,
        filename: file.filename,
        expires_in_seconds: DOWNLOAD_EXPIRY_SECONDS,
    })
}

fn ok<T: Serialize>(value: &T) -> AppResult<ApiResponse> {
    Ok(json_response(StatusCode::OK, value)?)
}
