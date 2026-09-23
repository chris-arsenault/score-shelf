//! Caller identity from the Authorization header.
//!
//! The shared ALB's `jwt-validation` action has already verified the token's
//! signature and issuer against the shared Cognito pool, so this module only
//! decodes the claims and decides who the caller is. Any pool token passes the
//! ALB, which is why client id, token use and scope are checked here.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;

use crate::config::AuthConfig;
use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Principal {
    /// A signed-in user of the Score Shelf app client. The platform's
    /// pre-auth trigger restricts sign-in to users granted this app.
    Owner { username: String },
    /// The composition agent's client-credentials client.
    Publisher { client_id: String },
}

impl Principal {
    pub fn source(&self) -> &'static str {
        match self {
            Self::Owner { .. } => "owner",
            Self::Publisher { .. } => "agent",
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::Owner { .. } => "owner",
            Self::Publisher { .. } => "publisher",
        }
    }
}

#[derive(Deserialize)]
struct Claims {
    token_use: Option<String>,
    client_id: Option<String>,
    scope: Option<String>,
    username: Option<String>,
    sub: Option<String>,
}

pub fn principal_from_authorization(
    header: Option<&str>,
    config: &AuthConfig,
) -> AppResult<Principal> {
    let token = extract_bearer(header)?;
    let claims = decode_claims(token)?;
    principal_from_claims(claims, config)
}

fn principal_from_claims(claims: Claims, config: &AuthConfig) -> AppResult<Principal> {
    if claims.token_use.as_deref() != Some("access") {
        return Err(AppError::Unauthorized("not an access token".to_string()));
    }
    let client_id = claims
        .client_id
        .ok_or_else(|| AppError::Unauthorized("token has no client_id".to_string()))?;
    if client_id == config.publisher_client_id {
        let scopes = claims.scope.unwrap_or_default();
        if scopes
            .split_whitespace()
            .any(|s| s == config.publisher_scope)
        {
            return Ok(Principal::Publisher { client_id });
        }
        return Err(AppError::Forbidden(
            "publisher token lacks scope".to_string(),
        ));
    }
    if client_id == config.app_client_id {
        let username = claims
            .username
            .or(claims.sub)
            .ok_or_else(|| AppError::Unauthorized("token has no username".to_string()))?;
        return Ok(Principal::Owner { username });
    }
    Err(AppError::Forbidden(format!(
        "client {client_id} not accepted"
    )))
}

fn extract_bearer(header: Option<&str>) -> AppResult<&str> {
    let value = header.ok_or_else(|| AppError::Unauthorized("missing header".to_string()))?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
        .filter(|token| !token.is_empty())
        .ok_or_else(|| AppError::Unauthorized("not a bearer token".to_string()))
}

fn decode_claims(token: &str) -> AppResult<Claims> {
    let payload = token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::Unauthorized("malformed token".to_string()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| AppError::Unauthorized("malformed token payload".to_string()))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Unauthorized("malformed token claims".to_string()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn auth_config() -> AuthConfig {
        AuthConfig {
            app_client_id: "app-client".to_string(),
            publisher_client_id: "publisher-client".to_string(),
            publisher_scope: "score-shelf/publish".to_string(),
        }
    }

    pub(crate) fn bearer(claims: serde_json::Value) -> String {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none"}"#);
        let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
        format!("Bearer {header}.{payload}.sig")
    }

    pub(crate) fn owner_bearer() -> String {
        bearer(serde_json::json!({
            "token_use": "access", "client_id": "app-client", "username": "chris"
        }))
    }

    pub(crate) fn publisher_bearer() -> String {
        bearer(serde_json::json!({
            "token_use": "access", "client_id": "publisher-client",
            "scope": "score-shelf/publish"
        }))
    }

    fn principal(header: &str) -> AppResult<Principal> {
        principal_from_authorization(Some(header), &auth_config())
    }

    #[test]
    fn test_app_client_access_token_is_owner() {
        assert_eq!(
            principal(&owner_bearer()).unwrap(),
            Principal::Owner {
                username: "chris".to_string()
            }
        );
    }

    #[test]
    fn test_publisher_with_scope_is_publisher() {
        let result = principal(&publisher_bearer()).unwrap();
        assert_eq!(result.source(), "agent");
    }

    #[test]
    fn test_publisher_without_scope_is_forbidden() {
        let header = bearer(serde_json::json!({
            "token_use": "access", "client_id": "publisher-client", "scope": "other/read"
        }));
        assert!(matches!(principal(&header), Err(AppError::Forbidden(_))));
    }

    #[test]
    fn test_other_platform_clients_are_forbidden() {
        let header = bearer(serde_json::json!({
            "token_use": "access", "client_id": "tastebase-app", "username": "chris"
        }));
        assert!(matches!(principal(&header), Err(AppError::Forbidden(_))));
    }

    #[test]
    fn test_id_tokens_are_rejected() {
        let header = bearer(serde_json::json!({
            "token_use": "id", "aud": "app-client", "cognito:username": "chris"
        }));
        assert!(matches!(principal(&header), Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn test_missing_or_malformed_headers_are_unauthorized() {
        let config = auth_config();
        for header in [
            None,
            Some("Basic abc"),
            Some("Bearer "),
            Some("Bearer nodots"),
        ] {
            let result = principal_from_authorization(header, &config);
            assert!(
                matches!(result, Err(AppError::Unauthorized(_))),
                "{header:?}"
            );
        }
    }
}
