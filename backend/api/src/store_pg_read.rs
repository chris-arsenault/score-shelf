//! Read queries for the PostgreSQL store. Only ready versions are visible.

use std::collections::HashMap;

use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{FileKind, FileSummary, PieceDetail, PieceSummary, VersionSummary};
use crate::error::{AppError, AppResult};
use crate::store::StoredFile;

const VERSION_COLUMNS: &str =
    "v.id, v.number, v.label, v.notes, v.source, v.source_ref, v.created_at";

#[derive(FromRow)]
struct VersionRow {
    id: Uuid,
    number: i32,
    label: String,
    notes: String,
    source: String,
    source_ref: Option<String>,
    created_at: OffsetDateTime,
}

#[derive(FromRow)]
struct FileRow {
    id: Uuid,
    version_id: Uuid,
    kind: String,
    filename: String,
    content_type: String,
    size_bytes: i64,
}

#[derive(FromRow)]
struct PieceRow {
    slug: String,
    title: String,
    version_count: i64,
    latest_id: Option<Uuid>,
}

pub async fn list_pieces(pool: &PgPool) -> AppResult<Vec<PieceSummary>> {
    let pieces: Vec<PieceRow> = sqlx::query_as(
        "SELECT p.slug, p.title,
                (SELECT count(*) FROM versions c WHERE c.piece_id = p.id AND c.status = 'ready')
                  AS version_count,
                (SELECT l.id FROM versions l WHERE l.piece_id = p.id AND l.status = 'ready'
                  ORDER BY l.number DESC LIMIT 1) AS latest_id
         FROM pieces p ORDER BY p.updated_at DESC, p.slug",
    )
    .fetch_all(pool)
    .await?;
    let latest_ids: Vec<Uuid> = pieces.iter().filter_map(|p| p.latest_id).collect();
    let mut versions = versions_by_id(pool, &latest_ids).await?;
    Ok(pieces
        .into_iter()
        .filter(|piece| piece.version_count > 0)
        .map(|piece| PieceSummary {
            latest: piece.latest_id.and_then(|id| versions.remove(&id)),
            slug: piece.slug,
            title: piece.title,
            version_count: piece.version_count,
        })
        .collect())
}

pub async fn piece_detail(pool: &PgPool, slug: &str) -> AppResult<Option<PieceDetail>> {
    let piece: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, title FROM pieces WHERE slug = $1")
            .bind(slug)
            .fetch_optional(pool)
            .await?;
    let Some((piece_id, title)) = piece else {
        return Ok(None);
    };
    let rows: Vec<VersionRow> = sqlx::query_as(&format!(
        "SELECT {VERSION_COLUMNS} FROM versions v
         WHERE v.piece_id = $1 AND v.status = 'ready' ORDER BY v.number DESC"
    ))
    .bind(piece_id)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let ids: Vec<Uuid> = rows.iter().map(|row| row.id).collect();
    let mut files = files_by_version(pool, &ids).await?;
    let versions = rows
        .into_iter()
        .map(|row| {
            let files = files.remove(&row.id).unwrap_or_default();
            summary(row, files)
        })
        .collect();
    Ok(Some(PieceDetail {
        slug: slug.to_string(),
        title,
        versions,
    }))
}

pub async fn version_summary(pool: &PgPool, version_id: Uuid) -> AppResult<VersionSummary> {
    versions_by_id(pool, &[version_id])
        .await?
        .remove(&version_id)
        .ok_or_else(|| AppError::NotFound(format!("version {version_id}")))
}

pub async fn ready_file(pool: &PgPool, file_id: Uuid) -> AppResult<Option<StoredFile>> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT f.filename, f.content_type, f.object_key FROM version_files f
         JOIN versions v ON v.id = f.version_id
         WHERE f.id = $1 AND v.status = 'ready'",
    )
    .bind(file_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(filename, content_type, object_key)| StoredFile {
        filename,
        content_type,
        object_key,
    }))
}

async fn versions_by_id(pool: &PgPool, ids: &[Uuid]) -> AppResult<HashMap<Uuid, VersionSummary>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<VersionRow> = sqlx::query_as(&format!(
        "SELECT {VERSION_COLUMNS} FROM versions v WHERE v.id = ANY($1)"
    ))
    .bind(ids)
    .fetch_all(pool)
    .await?;
    let mut files = files_by_version(pool, ids).await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let id = row.id;
            let version_files = files.remove(&id).unwrap_or_default();
            (id, summary(row, version_files))
        })
        .collect())
}

async fn files_by_version(
    pool: &PgPool,
    ids: &[Uuid],
) -> AppResult<HashMap<Uuid, Vec<FileSummary>>> {
    let rows: Vec<FileRow> = sqlx::query_as(
        "SELECT id, version_id, kind, filename, content_type, size_bytes
         FROM version_files WHERE version_id = ANY($1) ORDER BY filename",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;
    let mut grouped: HashMap<Uuid, Vec<FileSummary>> = HashMap::new();
    for row in rows {
        grouped
            .entry(row.version_id)
            .or_default()
            .push(FileSummary {
                id: row.id,
                kind: FileKind::parse(&row.kind)?,
                filename: row.filename,
                content_type: row.content_type,
                size_bytes: row.size_bytes,
            });
    }
    Ok(grouped)
}

fn summary(row: VersionRow, files: Vec<FileSummary>) -> VersionSummary {
    VersionSummary {
        id: row.id,
        number: row.number,
        label: row.label,
        notes: row.notes,
        source: row.source,
        source_ref: row.source_ref,
        created_at: row.created_at,
        files,
    }
}
