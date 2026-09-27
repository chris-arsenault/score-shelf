//! Persistence contract for pieces, versions and files.

use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{NewVersion, PieceDetail, PieceSummary, VersionSummary};
use crate::error::AppResult;

/// A version created in `pending` state, with the S3 keys its files go to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedVersion {
    pub version_id: Uuid,
    pub number: i32,
    pub files: Vec<CreatedFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedFile {
    pub file_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub object_key: String,
}

/// What a commit must verify before a version becomes visible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingVersion {
    pub ready: bool,
    pub files: Vec<ExpectedObject>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpectedObject {
    pub object_key: String,
    pub size_bytes: i64,
}

/// A downloadable file of a ready version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredFile {
    pub filename: String,
    pub content_type: String,
    pub object_key: String,
}

pub fn object_key(slug: &str, number: i32, file_id: Uuid, filename: &str) -> String {
    format!("pieces/{slug}/v{number}/{file_id}/{filename}")
}

#[async_trait]
pub trait ShelfStore: Send + Sync {
    async fn list_pieces(&self) -> AppResult<Vec<PieceSummary>>;
    async fn piece_detail(&self, slug: &str) -> AppResult<Option<PieceDetail>>;
    /// Creates a pending version. With `replaces`, it must name the piece's latest
    /// ready version from the same source, and the new version takes its number.
    async fn create_version(&self, version: &NewVersion) -> AppResult<CreatedVersion>;
    async fn pending_version(&self, version_id: Uuid) -> AppResult<Option<PendingVersion>>;
    /// Makes a pending version ready; a replacement hides the version it replaces
    /// in the same transaction.
    async fn mark_ready(&self, version_id: Uuid) -> AppResult<VersionSummary>;
    async fn ready_file(&self, file_id: Uuid) -> AppResult<Option<StoredFile>>;
    /// Hides a ready version. `source` limits which versions the caller may retire.
    async fn retire_version(&self, slug: &str, number: i32, source: Option<&str>) -> AppResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_key_layout() {
        let id = Uuid::nil();
        assert_eq!(
            object_key("boreal_pocket", 3, id, "boreal_pocket.mid"),
            format!("pieces/boreal_pocket/v3/{id}/boreal_pocket.mid")
        );
    }
}
