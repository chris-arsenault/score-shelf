use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String,
}

/// Which token issuers the API accepts. The shared ALB verifies signatures
/// only, so the API decides by client id and scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthConfig {
    pub app_client_id: String,
    pub publisher_client_id: String,
    pub publisher_scope: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub files_bucket: String,
}

impl AppConfig {
    pub fn from_env() -> AppResult<Self> {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> AppResult<Self> {
        let require = |name: &str| {
            lookup(name)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| AppError::Config(format!("missing env {name}")))
        };
        let port = require("DB_PORT")?
            .parse::<u16>()
            .map_err(|_| AppError::Config("invalid env DB_PORT".to_string()))?;
        Ok(Self {
            database: DatabaseConfig {
                host: require("DB_HOST")?,
                port,
                name: require("DB_NAME")?,
                username: require("DB_USERNAME")?,
                password: require("DB_PASSWORD")?,
            },
            auth: AuthConfig {
                app_client_id: require("COGNITO_CLIENT_ID")?,
                publisher_client_id: require("PUBLISHER_CLIENT_ID")?,
                publisher_scope: require("PUBLISHER_SCOPE")?,
            },
            files_bucket: require("FILES_BUCKET")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("DB_HOST", "db.internal"),
            ("DB_PORT", "5432"),
            ("DB_NAME", "score_shelf"),
            ("DB_USERNAME", "app"),
            ("DB_PASSWORD", "secret"),
            ("COGNITO_CLIENT_ID", "app-client"),
            ("PUBLISHER_CLIENT_ID", "publisher-client"),
            ("PUBLISHER_SCOPE", "score-shelf/publish"),
            ("FILES_BUCKET", "score-shelf-files"),
        ])
    }

    #[test]
    fn test_reads_complete_environment() {
        let values = env();
        let config =
            AppConfig::from_lookup(|name| values.get(name).map(|v| v.to_string())).expect("config");
        assert_eq!(config.database.port, 5432);
        assert_eq!(config.auth.publisher_scope, "score-shelf/publish");
        assert_eq!(config.files_bucket, "score-shelf-files");
    }

    #[test]
    fn test_rejects_missing_or_blank_values() {
        let mut values = env();
        values.insert("FILES_BUCKET", "  ");
        let err = AppConfig::from_lookup(|name| values.get(name).map(|v| v.to_string()));
        assert!(matches!(err, Err(AppError::Config(message)) if message.contains("FILES_BUCKET")));
    }

    #[test]
    fn test_rejects_non_numeric_port() {
        let mut values = env();
        values.insert("DB_PORT", "postgres");
        let err = AppConfig::from_lookup(|name| values.get(name).map(|v| v.to_string()));
        assert!(matches!(err, Err(AppError::Config(_))));
    }
}
