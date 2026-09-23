//! PostgreSQL implementation of [`ShelfStore`]. Reads live in
//! `store_pg_read`; this file holds the connection and the writes.

use async_trait::async_trait;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::config::DatabaseConfig;
use crate::domain::{NewVersion, PieceDetail, PieceSummary, VersionSummary};
use crate::error::{AppError, AppResult};
use crate::store::{
    object_key, CreatedFile, CreatedVersion, ExpectedObject, PendingVersion, ShelfStore, StoredFile,
};
use crate::store_pg_read;

pub const SHELF_MIGRATION: &str = include_str!("../../../db/migrations/001_create_shelf.sql");
pub const SHELF_ROLLBACK: &str =
    include_str!("../../../db/migrations/rollback/001_create_shelf.sql");

const MAX_POOL_CONNECTIONS: u32 = 5;

pub struct PgShelfStore {
    pool: PgPool,
}

impl PgShelfStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn connect(config: &DatabaseConfig) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_POOL_CONNECTIONS)
            .connect(&database_url(config))
            .await?;
        Ok(Self::new(pool))
    }
}

pub fn database_url(config: &DatabaseConfig) -> String {
    format!(
        "postgres://{}:{}@{}:{}/{}?sslmode=require",
        encode_userinfo(&config.username),
        encode_userinfo(&config.password),
        config.host,
        config.port,
        config.name
    )
}

fn encode_userinfo(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[async_trait]
impl ShelfStore for PgShelfStore {
    async fn list_pieces(&self) -> AppResult<Vec<PieceSummary>> {
        store_pg_read::list_pieces(&self.pool).await
    }

    async fn piece_detail(&self, slug: &str) -> AppResult<Option<PieceDetail>> {
        store_pg_read::piece_detail(&self.pool, slug).await
    }

    async fn create_version(&self, version: &NewVersion) -> AppResult<CreatedVersion> {
        let mut tx = self.pool.begin().await?;
        let piece_id = upsert_piece(&mut tx, version).await?;
        let number = next_version_number(&mut tx, piece_id).await?;
        let version_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO versions (id, piece_id, number, label, notes, source, source_ref, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending')",
        )
        .bind(version_id)
        .bind(piece_id)
        .bind(number)
        .bind(&version.label)
        .bind(&version.notes)
        .bind(version.source)
        .bind(&version.source_ref)
        .execute(&mut *tx)
        .await?;
        let files = insert_files(&mut tx, version, version_id, number).await?;
        tx.commit().await?;
        Ok(CreatedVersion {
            version_id,
            number,
            files,
        })
    }

    async fn pending_version(&self, version_id: Uuid) -> AppResult<Option<PendingVersion>> {
        let status: Option<String> =
            sqlx::query_scalar("SELECT status FROM versions WHERE id = $1")
                .bind(version_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(status) = status else {
            return Ok(None);
        };
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT object_key, size_bytes FROM version_files WHERE version_id = $1
             ORDER BY filename",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(Some(PendingVersion {
            ready: status == "ready",
            files: rows
                .into_iter()
                .map(|(object_key, size_bytes)| ExpectedObject {
                    object_key,
                    size_bytes,
                })
                .collect(),
        }))
    }

    async fn mark_ready(&self, version_id: Uuid) -> AppResult<VersionSummary> {
        let mut tx = self.pool.begin().await?;
        let piece_id: Option<Uuid> = sqlx::query_scalar(
            "UPDATE versions SET status = 'ready', committed_at = now()
             WHERE id = $1 AND status = 'pending' RETURNING piece_id",
        )
        .bind(version_id)
        .fetch_optional(&mut *tx)
        .await?;
        let piece_id =
            piece_id.ok_or_else(|| AppError::Conflict("version is not pending".to_string()))?;
        sqlx::query("UPDATE pieces SET updated_at = now() WHERE id = $1")
            .bind(piece_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        store_pg_read::version_summary(&self.pool, version_id).await
    }

    async fn ready_file(&self, file_id: Uuid) -> AppResult<Option<StoredFile>> {
        store_pg_read::ready_file(&self.pool, file_id).await
    }
}

async fn upsert_piece(tx: &mut Transaction<'_, Postgres>, version: &NewVersion) -> AppResult<Uuid> {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO pieces (id, slug, title) VALUES ($1, $2, COALESCE($3, $2))
         ON CONFLICT (slug) DO UPDATE SET title = COALESCE($3, pieces.title)
         RETURNING id",
    )
    .bind(Uuid::new_v4())
    .bind(&version.slug)
    .bind(&version.title)
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}

async fn next_version_number(tx: &mut Transaction<'_, Postgres>, piece_id: Uuid) -> AppResult<i32> {
    sqlx::query("SELECT id FROM pieces WHERE id = $1 FOR UPDATE")
        .bind(piece_id)
        .execute(&mut **tx)
        .await?;
    let number: i32 =
        sqlx::query_scalar("SELECT COALESCE(MAX(number), 0) + 1 FROM versions WHERE piece_id = $1")
            .bind(piece_id)
            .fetch_one(&mut **tx)
            .await?;
    Ok(number)
}

async fn insert_files(
    tx: &mut Transaction<'_, Postgres>,
    version: &NewVersion,
    version_id: Uuid,
    number: i32,
) -> AppResult<Vec<CreatedFile>> {
    let mut created = Vec::with_capacity(version.files.len());
    for file in &version.files {
        let key = object_key(&version.slug, number, file.id, &file.filename);
        sqlx::query(
            "INSERT INTO version_files
               (id, version_id, kind, filename, content_type, size_bytes, object_key)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(file.id)
        .bind(version_id)
        .bind(file.kind.as_str())
        .bind(&file.filename)
        .bind(&file.content_type)
        .bind(file.size_bytes)
        .bind(&key)
        .execute(&mut **tx)
        .await?;
        created.push(CreatedFile {
            file_id: file.id,
            filename: file.filename.clone(),
            content_type: file.content_type.clone(),
            object_key: key,
        });
    }
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_url_encodes_credentials_and_requires_tls() {
        let config = DatabaseConfig {
            host: "db.internal".to_string(),
            port: 6543,
            name: "score_shelf".to_string(),
            username: "app_user".to_string(),
            password: "p@ss word".to_string(),
        };
        assert_eq!(
            database_url(&config),
            "postgres://app_user:p%40ss%20word@db.internal:6543/score_shelf?sslmode=require"
        );
    }

    #[test]
    fn test_migration_and_rollback_cover_all_tables() {
        for table in ["pieces", "versions", "version_files"] {
            assert!(SHELF_MIGRATION.contains(&format!("CREATE TABLE {table}")));
            assert!(SHELF_ROLLBACK.contains(&format!("DROP TABLE IF EXISTS {table}")));
        }
    }
}
