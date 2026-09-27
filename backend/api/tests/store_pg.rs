//! PostgreSQL store behaviour against a real database (`make db-test`).

use api::domain::{FileKind, NewVersion, PlannedFile};
use api::error::AppError;
use api::store::ShelfStore;
use api::store_pg::{PgShelfStore, SHELF_MIGRATIONS, SHELF_ROLLBACKS};
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
    for migration in SHELF_MIGRATIONS {
        sqlx::raw_sql(migration)
            .execute(&pool)
            .await
            .expect("migrate");
    }
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
        replaces: None,
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

async fn ready(store: &PgShelfStore, version: NewVersion) -> i32 {
    let created = store.create_version(&version).await.unwrap();
    store.mark_ready(created.version_id).await.unwrap();
    created.number
}

fn numbers_and_labels(detail: &api::domain::PieceDetail) -> Vec<(i32, String)> {
    detail
        .versions
        .iter()
        .map(|v| (v.number, v.label.clone()))
        .collect()
}

#[tokio::test]
async fn test_replacement_takes_the_number_on_commit() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool);
    ready(&store, version("first", &[("a.mid", 5)])).await;
    ready(&store, version("trombone", &[("a.mid", 6)])).await;
    let mut fix = version("trombone, drum fix", &[("a.mid", 7)]);
    fix.replaces = Some(2);
    let created = store.create_version(&fix).await.unwrap();
    assert_eq!(created.number, 2);
    let before = store.piece_detail("boreal_pocket").await.unwrap().unwrap();
    assert_eq!(
        before.versions[0].label, "trombone",
        "old v2 stays until commit"
    );

    store.mark_ready(created.version_id).await.unwrap();
    let after = store.piece_detail("boreal_pocket").await.unwrap().unwrap();
    assert_eq!(
        numbers_and_labels(&after),
        vec![
            (2, "trombone, drum fix".to_string()),
            (1, "first".to_string())
        ]
    );
    assert_eq!(ready(&store, version("reorder", &[("a.mid", 8)])).await, 3);
}

#[tokio::test]
async fn test_replace_rejects_older_versions_and_other_sources() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool);
    ready(&store, version("first", &[("a.mid", 5)])).await;
    let mut hand_edit = version("hand edit", &[("a.mid", 6)]);
    hand_edit.source = "owner";
    ready(&store, hand_edit).await;
    let mut stale = version("stale", &[("a.mid", 7)]);
    stale.replaces = Some(1);
    assert!(matches!(
        store.create_version(&stale).await,
        Err(AppError::Conflict(_))
    ));
    stale.replaces = Some(2);
    assert!(matches!(
        store.create_version(&stale).await,
        Err(AppError::Forbidden(_))
    ));
}

#[tokio::test]
async fn test_retired_latest_frees_its_number() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool);
    ready(&store, version("first", &[("a.mid", 5)])).await;
    ready(&store, version("re-tag", &[("a.mid", 6)])).await;
    assert!(matches!(
        store
            .retire_version("boreal_pocket", 2, Some("owner"))
            .await,
        Err(AppError::NotFound(_))
    ));
    store
        .retire_version("boreal_pocket", 2, Some("agent"))
        .await
        .unwrap();
    assert_eq!(ready(&store, version("reorder", &[("a.mid", 7)])).await, 2);
    let detail = store.piece_detail("boreal_pocket").await.unwrap().unwrap();
    assert_eq!(
        numbers_and_labels(&detail),
        vec![(2, "reorder".to_string()), (1, "first".to_string())]
    );
}

#[tokio::test]
async fn test_rollback_of_replacements_restores_unique_numbers() {
    let (_container, pool) = database().await;
    let store = PgShelfStore::new(pool.clone());
    ready(&store, version("first", &[("a.mid", 5)])).await;
    let mut fix = version("fix", &[("a.mid", 6)]);
    fix.replaces = Some(1);
    ready(&store, fix).await;
    sqlx::raw_sql(SHELF_ROLLBACKS[0])
        .execute(&pool)
        .await
        .expect("rollback 002");
    let rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT number, status FROM versions ORDER BY number")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(rows, vec![(1, "ready".to_string())]);
}

#[tokio::test]
async fn test_rollback_drops_project_tables() {
    let (_container, pool) = database().await;
    for rollback in SHELF_ROLLBACKS {
        sqlx::raw_sql(rollback)
            .execute(&pool)
            .await
            .expect("rollback");
    }
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables
         WHERE table_name IN ('pieces', 'versions', 'version_files')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(remaining, 0);
}
