//! PostgreSQL store behaviour against a real database (`make db-test`).

use api::domain::{FileKind, NewVersion, PlannedFile};
use api::error::AppError;
use api::store::ShelfStore;
use api::store_pg::{PgShelfStore, SHELF_MIGRATION, SHELF_ROLLBACK};
use sqlx::PgPool;
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner;
use testcontainers_modules::testcontainers::ContainerAsync;
use uuid::Uuid;

async fn database() -> (ContainerAsync<Postgres>, PgPool) {
    let container = Postgres::default()
        .start()
        .await
        .expect("postgres container");
    let port = container.get_host_port_ipv4(5432).await.expect("port");
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = PgPool::connect(&url).await.expect("connect");
    sqlx::raw_sql(SHELF_MIGRATION)
        .execute(&pool)
        .await
        .expect("migrate");
    (container, pool)
}

fn version(label: &str, files: &[(&str, i64)]) -> NewVersion {
    NewVersion {
        slug: "boreal_pocket".to_string(),
        title: Some("Boreal Pocket".to_string()),
        label: label.to_string(),
        notes: String::new(),
        source: "agent",
        source_ref: Some("66db377".to_string()),
        files: files
            .iter()
            .map(|(name, size)| PlannedFile {
                id: Uuid::new_v4(),
                filename: name.to_string(),
                kind: FileKind::Midi,
                content_type: "audio/midi".to_string(),
                size_bytes: *size,
            })
            .collect(),
    }
}

#[tokio::test]
async fn test_versions_number_per_piece_and_stay_hidden_until_ready() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool);
    let first = store
        .create_version(&version("first", &[("a.mid", 10)]))
        .await
        .unwrap();
    let second = store
        .create_version(&version("second", &[("a.mid", 12)]))
        .await
        .unwrap();
    assert_eq!((first.number, second.number), (1, 2));
    assert!(first.files[0]
        .object_key
        .starts_with("pieces/boreal_pocket/v1/"));

    assert!(store.list_pieces().await.unwrap().is_empty());
    assert!(store.piece_detail("boreal_pocket").await.unwrap().is_none());

    let pending = store
        .pending_version(second.version_id)
        .await
        .unwrap()
        .unwrap();
    assert!(!pending.ready);
    assert_eq!(pending.files[0].size_bytes, 12);

    let ready = store.mark_ready(second.version_id).await.unwrap();
    assert_eq!(ready.label, "second");
    let pieces = store.list_pieces().await.unwrap();
    assert_eq!(pieces[0].version_count, 1);
    assert_eq!(pieces[0].latest.as_ref().unwrap().number, 2);
    let detail = store.piece_detail("boreal_pocket").await.unwrap().unwrap();
    assert_eq!(detail.title, "Boreal Pocket");
    assert_eq!(detail.versions[0].files.len(), 1);
}

#[tokio::test]
async fn test_mark_ready_is_not_repeatable() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool);
    let created = store
        .create_version(&version("v", &[("a.mid", 5)]))
        .await
        .unwrap();
    store.mark_ready(created.version_id).await.unwrap();
    let again = store.mark_ready(created.version_id).await;
    assert!(matches!(again, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn test_ready_file_only_returns_committed_files() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool);
    let created = store
        .create_version(&version("v", &[("a.mid", 5)]))
        .await
        .unwrap();
    let file_id = created.files[0].file_id;
    assert!(store.ready_file(file_id).await.unwrap().is_none());
    store.mark_ready(created.version_id).await.unwrap();
    let file = store.ready_file(file_id).await.unwrap().unwrap();
    assert_eq!(file.filename, "a.mid");
    assert_eq!(file.object_key, created.files[0].object_key);
}

#[tokio::test]
async fn test_duplicate_filenames_in_a_version_are_rejected_by_the_schema() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool);
    let result = store
        .create_version(&version("v", &[("a.mid", 5), ("a.mid", 6)]))
        .await;
    assert!(matches!(result, Err(AppError::Database(_))));
    assert!(store.list_pieces().await.unwrap().is_empty());
}

#[tokio::test]
async fn test_rollback_drops_project_tables() {
    let (_container, pool) = database().await;
    sqlx::raw_sql(SHELF_ROLLBACK)
        .execute(&pool)
        .await
        .expect("rollback");
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables
         WHERE table_name IN ('pieces', 'versions', 'version_files')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, 0);
}
