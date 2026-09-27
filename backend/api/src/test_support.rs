//! In-memory doubles for the store and object store, used by lib tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use lambda_http::http::Method;
use lambda_http::{Body, Request};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::tests::auth_config;
use crate::domain::{FileSummary, NewVersion, PieceDetail, PieceSummary, VersionSummary};
use crate::error::{AppError, AppResult};
use crate::objects::{ObjectStore, UploadTarget};
use crate::store::{
    object_key, CreatedFile, CreatedVersion, ExpectedObject, PendingVersion, ShelfStore, StoredFile,
};
use crate::ApiState;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Pending,
    Ready,
    Hidden,
}

struct StoredVersion {
    slug: String,
    status: Status,
    replaces: Option<Uuid>,
    summary: VersionSummary,
    keys: Vec<String>,
}

#[derive(Default)]
pub struct InMemoryShelfStore {
    titles: Mutex<HashMap<String, String>>,
    versions: Mutex<Vec<StoredVersion>>,
}

impl InMemoryShelfStore {
    fn ready_versions(&self, slug: &str) -> Vec<VersionSummary> {
        let versions = self.versions.lock().unwrap();
        let mut ready: Vec<VersionSummary> = versions
            .iter()
            .filter(|v| v.slug == slug && v.status == Status::Ready)
            .map(|v| v.summary.clone())
            .collect();
        ready.sort_by_key(|v| std::cmp::Reverse(v.number));
        ready
    }

    /// Same rules as `store_pg::plan_number`.
    fn plan_number(&self, version: &NewVersion) -> AppResult<(i32, Option<Uuid>)> {
        let latest = self.ready_versions(&version.slug).into_iter().next();
        let Some(wanted) = version.replaces else {
            let versions = self.versions.lock().unwrap();
            let highest = versions
                .iter()
                .filter(|v| v.slug == version.slug && v.status != Status::Hidden)
                .map(|v| v.summary.number)
                .max();
            return Ok((highest.unwrap_or(0) + 1, None));
        };
        match latest {
            Some(v) if v.number == wanted && v.source == version.source => Ok((wanted, Some(v.id))),
            Some(v) if v.number == wanted => Err(AppError::Forbidden(v.source)),
            _ => Err(AppError::Conflict(format!(
                "version {wanted} is not the latest"
            ))),
        }
    }
}

#[async_trait]
impl ShelfStore for InMemoryShelfStore {
    async fn list_pieces(&self) -> AppResult<Vec<PieceSummary>> {
        let titles = self.titles.lock().unwrap().clone();
        let mut pieces: Vec<PieceSummary> = titles
            .into_iter()
            .filter_map(|(slug, title)| {
                let versions = self.ready_versions(&slug);
                let latest = versions.first().cloned()?;
                Some(PieceSummary {
                    slug,
                    title,
                    version_count: versions.len() as i64,
                    latest: Some(latest),
                })
            })
            .collect();
        pieces.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(pieces)
    }

    async fn piece_detail(&self, slug: &str) -> AppResult<Option<PieceDetail>> {
        let title = self.titles.lock().unwrap().get(slug).cloned();
        let versions = self.ready_versions(slug);
        Ok(title
            .filter(|_| !versions.is_empty())
            .map(|title| PieceDetail {
                slug: slug.to_string(),
                title,
                versions,
            }))
    }

    async fn create_version(&self, version: &NewVersion) -> AppResult<CreatedVersion> {
        let mut titles = self.titles.lock().unwrap();
        let title = version
            .title
            .clone()
            .unwrap_or_else(|| version.slug.clone());
        let entry = titles.entry(version.slug.clone()).or_insert(title.clone());
        if version.title.is_some() {
            *entry = title;
        }
        drop(titles);
        let (number, replaces) = self.plan_number(version)?;
        let mut versions = self.versions.lock().unwrap();
        let version_id = Uuid::new_v4();
        let files: Vec<CreatedFile> = version
            .files
            .iter()
            .map(|file| CreatedFile {
                file_id: file.id,
                filename: file.filename.clone(),
                content_type: file.content_type.clone(),
                object_key: object_key(&version.slug, number, file.id, &file.filename),
            })
            .collect();
        versions.push(StoredVersion {
            slug: version.slug.clone(),
            status: Status::Pending,
            replaces,
            keys: files.iter().map(|f| f.object_key.clone()).collect(),
            summary: summary_of(version, version_id, number),
        });
        Ok(CreatedVersion {
            version_id,
            number,
            files,
        })
    }

    async fn pending_version(&self, version_id: Uuid) -> AppResult<Option<PendingVersion>> {
        let versions = self.versions.lock().unwrap();
        Ok(versions
            .iter()
            .find(|v| v.summary.id == version_id)
            .map(|v| PendingVersion {
                ready: v.status != Status::Pending,
                files: v
                    .keys
                    .iter()
                    .zip(&v.summary.files)
                    .map(|(key, file)| ExpectedObject {
                        object_key: key.clone(),
                        size_bytes: file.size_bytes,
                    })
                    .collect(),
            }))
    }

    async fn mark_ready(&self, version_id: Uuid) -> AppResult<VersionSummary> {
        let mut versions = self.versions.lock().unwrap();
        let index = versions
            .iter()
            .position(|v| v.summary.id == version_id && v.status == Status::Pending)
            .ok_or_else(|| AppError::Conflict("version is not pending".to_string()))?;
        if let Some(replaced_id) = versions[index].replaces {
            let replaced = versions
                .iter_mut()
                .find(|v| v.summary.id == replaced_id && v.status == Status::Ready)
                .ok_or_else(|| AppError::Conflict("replaced version changed".to_string()))?;
            replaced.status = Status::Hidden;
        }
        versions[index].status = Status::Ready;
        Ok(versions[index].summary.clone())
    }

    async fn ready_file(&self, file_id: Uuid) -> AppResult<Option<StoredFile>> {
        let versions = self.versions.lock().unwrap();
        let mut ready = versions.iter().filter(|v| v.status == Status::Ready);
        Ok(ready.find_map(|v| {
            let index = v.summary.files.iter().position(|f| f.id == file_id)?;
            let file = &v.summary.files[index];
            Some(StoredFile {
                filename: file.filename.clone(),
                content_type: file.content_type.clone(),
                object_key: v.keys[index].clone(),
            })
        }))
    }

    async fn retire_version(&self, slug: &str, number: i32, source: Option<&str>) -> AppResult<()> {
        let mut versions = self.versions.lock().unwrap();
        let version = versions
            .iter_mut()
            .find(|v| {
                v.slug == slug
                    && v.summary.number == number
                    && v.status == Status::Ready
                    && source.is_none_or(|s| s == v.summary.source)
            })
            .ok_or_else(|| AppError::NotFound(format!("version {number}")))?;
        version.status = Status::Hidden;
        Ok(())
    }
}

fn summary_of(version: &NewVersion, id: Uuid, number: i32) -> VersionSummary {
    VersionSummary {
        id,
        number,
        label: version.label.clone(),
        notes: version.notes.clone(),
        source: version.source.to_string(),
        source_ref: version.source_ref.clone(),
        created_at: OffsetDateTime::UNIX_EPOCH,
        files: version
            .files
            .iter()
            .map(|file| FileSummary {
                id: file.id,
                kind: file.kind,
                filename: file.filename.clone(),
                content_type: file.content_type.clone(),
                size_bytes: file.size_bytes,
            })
            .collect(),
    }
}

/// Object store double: records which keys were "uploaded" and their sizes.
#[derive(Default)]
pub struct InMemoryObjectStore {
    pub sizes: Mutex<HashMap<String, i64>>,
}

impl InMemoryObjectStore {
    pub fn put(&self, key: &str, size: i64) {
        self.sizes.lock().unwrap().insert(key.to_string(), size);
    }
}

#[async_trait]
impl ObjectStore for InMemoryObjectStore {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<UploadTarget> {
        Ok(UploadTarget {
            url: format!("https://s3.test/{key}"),
            method: "PUT",
            headers: HashMap::from([("content-type".to_string(), content_type.to_string())]),
        })
    }

    async fn download_url(&self, key: &str, _: &str, filename: &str) -> AppResult<String> {
        Ok(format!("https://s3.test/{key}?download={filename}"))
    }

    async fn object_size(&self, key: &str) -> AppResult<Option<i64>> {
        Ok(self.sizes.lock().unwrap().get(key).copied())
    }
}

pub struct TestApi {
    pub state: Arc<ApiState>,
    pub objects: Arc<InMemoryObjectStore>,
}

pub fn test_api() -> TestApi {
    let objects = Arc::new(InMemoryObjectStore::default());
    let state = Arc::new(ApiState {
        auth: auth_config(),
        store: Arc::new(InMemoryShelfStore::default()),
        objects: objects.clone(),
    });
    TestApi { state, objects }
}

pub fn request(method: Method, path: &str, bearer: Option<&str>, body: Option<String>) -> Request {
    let mut builder = lambda_http::http::Request::builder()
        .method(method)
        .uri(format!("https://api.score-shelf.test{path}"));
    if let Some(bearer) = bearer {
        builder = builder.header("authorization", bearer);
    }
    let body = body.map(Body::Text).unwrap_or(Body::Empty);
    builder.body(body).expect("request")
}
