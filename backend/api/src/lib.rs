//! Score Shelf API: pieces, their versions, and presigned file transfer.

pub mod auth;
pub mod config;
pub mod domain;
pub mod error;
pub mod objects;
mod routes;
pub mod store;
pub mod store_pg;
mod store_pg_read;

#[cfg(test)]
mod routes_tests;
#[cfg(test)]
mod test_support;

use std::sync::Arc;

use ahara_lambda_http::{
    default_cors, error_response, json_value_response, PublicHttpError, Route,
};
use ahara_lambda_telemetry::{Operation, OperationKind, TelemetryConfig};
use lambda_http::http::{Method, StatusCode};
use lambda_http::{Body, Request, Response};

use crate::auth::{principal_from_authorization, Principal};
use crate::config::{AppConfig, AuthConfig};
use crate::error::{AppError, AppResult};
use crate::objects::{ObjectStore, S3ObjectStore};
use crate::store::ShelfStore;
use crate::store_pg::PgShelfStore;

pub const SERVICE_NAME: &str = "score-shelf-api";

pub type ApiResponse = Response<Body>;

#[derive(Clone)]
pub struct ApiState {
    pub auth: AuthConfig,
    pub store: Arc<dyn ShelfStore>,
    pub objects: Arc<dyn ObjectStore>,
}

impl ApiState {
    pub async fn from_env() -> AppResult<Self> {
        let config = AppConfig::from_env()?;
        let store = PgShelfStore::connect(&config.database).await?;
        let objects = S3ObjectStore::new(config.files_bucket.clone()).await;
        Ok(Self {
            auth: config.auth,
            store: Arc::new(store),
            objects: Arc::new(objects),
        })
    }
}

pub async fn handle_request(request: Request, state: Arc<ApiState>) -> ApiResponse {
    let response = dispatch(&request, &state).await.unwrap_or_else(|err| {
        if matches!(err.code().as_ref(), "internal_error") {
            tracing::error!(error = %err, "request failed");
        }
        error_response(&err)
    });
    default_cors(response)
}

async fn dispatch(request: &Request, state: &ApiState) -> AppResult<ApiResponse> {
    let route = Route::from_request(request);
    if route.is_match(Method::GET, "/health")? {
        return Ok(json_value_response(
            StatusCode::OK,
            serde_json::json!({"status": "ok"}),
        ));
    }
    let principal = authenticate(request, &state.auth)?;
    if let Some(response) = routes::dispatch(&route, request, state, &principal).await? {
        return Ok(response);
    }
    Err(AppError::NotFound(request.uri().path().to_string()))
}

fn authenticate(request: &Request, config: &AuthConfig) -> AppResult<Principal> {
    let header = request
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    principal_from_authorization(header, config)
}

/// Runs a handler inside a telemetry operation that records who called it.
pub(crate) async fn observe<T, Fut>(
    name: &'static str,
    principal: &Principal,
    future: Fut,
) -> AppResult<T>
where
    Fut: std::future::Future<Output = AppResult<T>>,
{
    Operation::new(TelemetryConfig::new(SERVICE_NAME), name)
        .with_domain("api")
        .with_kind(OperationKind::UserInteraction)
        .with_detail("actor.kind", principal.kind())
        .observe(future)
        .await
}
