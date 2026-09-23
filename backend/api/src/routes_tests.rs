//! Route behaviour through `handle_request` with in-memory doubles.

use lambda_http::http::{Method, StatusCode};
use lambda_http::Body;
use serde_json::{json, Value};

use crate::auth::tests::{bearer, owner_bearer, publisher_bearer};
use crate::handle_request;
use crate::test_support::{request, test_api, TestApi};

async fn call(
    api: &TestApi,
    method: Method,
    path: &str,
    auth: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let req = request(method, path, auth, body.map(|b| b.to_string()));
    let response = handle_request(req, api.state.clone()).await;
    let status = response.status();
    let value = match response.body() {
        Body::Text(text) => serde_json::from_str(text).unwrap_or(Value::Null),
        _ => Value::Null,
    };
    (status, value)
}

fn publish_body() -> Value {
    json!({
        "title": "Boreal Pocket",
        "label": "key ladders",
        "source_ref": "66db377",
        "files": [
            {"filename": "boreal_pocket.musicxml", "kind": "musicxml",
             "content_type": "application/vnd.recordare.musicxml+xml", "size_bytes": 1500},
            {"filename": "boreal_pocket.mid", "kind": "midi",
             "content_type": "audio/midi", "size_bytes": 300}
        ]
    })
}

async fn publish(api: &TestApi, auth: &str) -> Value {
    let (status, created) = call(
        api,
        Method::POST,
        "/pieces/boreal_pocket/versions",
        Some(auth),
        Some(publish_body()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created
}

fn upload_all(api: &TestApi, created: &Value) {
    for (upload, size) in created["uploads"]
        .as_array()
        .unwrap()
        .iter()
        .zip([1500, 300])
    {
        let url = upload["upload"]["url"].as_str().unwrap();
        api.objects
            .put(url.trim_start_matches("https://s3.test/"), size);
    }
}

#[tokio::test]
async fn test_health_needs_no_token() {
    let api = test_api();
    let (status, body) = call(&api, Method::GET, "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_routes_require_a_token() {
    let api = test_api();
    let (status, _) = call(&api, Method::GET, "/pieces", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_other_app_tokens_are_forbidden() {
    let api = test_api();
    let other = bearer(json!({"token_use": "access", "client_id": "tastebase-app",
                               "username": "chris"}));
    let (status, _) = call(&api, Method::GET, "/pieces", Some(&other), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_publish_commit_and_read_back() {
    let api = test_api();
    let created = publish(&api, &publisher_bearer()).await;
    assert_eq!(created["number"], 1);
    let version_id = created["version_id"].as_str().unwrap().to_string();

    let (_, listed) = call(&api, Method::GET, "/pieces", Some(&owner_bearer()), None).await;
    assert_eq!(listed["pieces"], json!([]), "pending versions stay hidden");

    upload_all(&api, &created);
    let commit = format!("/versions/{version_id}/commit");
    let (status, version) =
        call(&api, Method::POST, &commit, Some(&publisher_bearer()), None).await;
    assert_eq!(status, StatusCode::OK, "{version}");
    assert_eq!(version["source"], "agent");

    let (_, detail) = call(
        &api,
        Method::GET,
        "/pieces/boreal_pocket",
        Some(&owner_bearer()),
        None,
    )
    .await;
    assert_eq!(detail["title"], "Boreal Pocket");
    assert_eq!(detail["versions"][0]["label"], "key ladders");
    assert_eq!(detail["versions"][0]["files"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_commit_rejects_missing_or_wrong_size_uploads() {
    let api = test_api();
    let created = publish(&api, &publisher_bearer()).await;
    let version_id = created["version_id"].as_str().unwrap();
    let commit = format!("/versions/{version_id}/commit");
    let (status, _) = call(&api, Method::POST, &commit, Some(&publisher_bearer()), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    upload_all(&api, &created);
    let first_url = created["uploads"][0]["upload"]["url"].as_str().unwrap();
    api.objects
        .put(first_url.trim_start_matches("https://s3.test/"), 1);
    let (status, _) = call(&api, Method::POST, &commit, Some(&publisher_bearer()), None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_commit_twice_conflicts() {
    let api = test_api();
    let created = publish(&api, &publisher_bearer()).await;
    upload_all(&api, &created);
    let commit = format!(
        "/versions/{}/commit",
        created["version_id"].as_str().unwrap()
    );
    let (first, _) = call(&api, Method::POST, &commit, Some(&owner_bearer()), None).await;
    let (second, _) = call(&api, Method::POST, &commit, Some(&owner_bearer()), None).await;
    assert_eq!((first, second), (StatusCode::OK, StatusCode::CONFLICT));
}

#[tokio::test]
async fn test_owner_uploads_are_recorded_as_owner_versions() {
    let api = test_api();
    let created = publish(&api, &owner_bearer()).await;
    upload_all(&api, &created);
    let commit = format!(
        "/versions/{}/commit",
        created["version_id"].as_str().unwrap()
    );
    let (_, version) = call(&api, Method::POST, &commit, Some(&owner_bearer()), None).await;
    assert_eq!(version["source"], "owner");
}

#[tokio::test]
async fn test_download_only_for_ready_files() {
    let api = test_api();
    let created = publish(&api, &publisher_bearer()).await;
    let file_id = created["uploads"][0]["file_id"]
        .as_str()
        .unwrap()
        .to_string();
    let path = format!("/files/{file_id}/download");
    let (status, _) = call(&api, Method::GET, &path, Some(&owner_bearer()), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    upload_all(&api, &created);
    let commit = format!(
        "/versions/{}/commit",
        created["version_id"].as_str().unwrap()
    );
    call(&api, Method::POST, &commit, Some(&publisher_bearer()), None).await;
    let (status, body) = call(&api, Method::GET, &path, Some(&owner_bearer()), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["filename"], "boreal_pocket.musicxml");
    assert!(body["url"]
        .as_str()
        .unwrap()
        .contains("download=boreal_pocket.musicxml"));
}

#[tokio::test]
async fn test_invalid_publish_bodies_are_rejected() {
    let api = test_api();
    let mut body = publish_body();
    body["label"] = json!("");
    let path = "/pieces/boreal_pocket/versions";
    let (status, _) = call(
        &api,
        Method::POST,
        path,
        Some(&publisher_bearer()),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &api,
        Method::POST,
        "/pieces/Bad%20Slug/versions",
        Some(&publisher_bearer()),
        Some(publish_body()),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_me_reports_the_caller() {
    let api = test_api();
    let (_, owner) = call(&api, Method::GET, "/me", Some(&owner_bearer()), None).await;
    assert_eq!(owner, json!({"kind": "owner", "username": "chris"}));
    let (_, publisher) = call(&api, Method::GET, "/me", Some(&publisher_bearer()), None).await;
    assert_eq!(publisher["kind"], "publisher");
}
