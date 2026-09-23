use std::borrow::Cow;

use ahara_lambda_http::{HttpError, PublicHttpError};
use lambda_http::http::StatusCode;

pub type AppResult<T> = Result<T, AppError>;

/// Application errors. Private detail stays in the variant; only the public
/// status, code and message reach clients.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration: {0}")]
    Config(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database: {0}")]
    Database(String),
    #[error("{service}: {message}")]
    ExternalService {
        service: &'static str,
        message: String,
    },
    #[error(transparent)]
    Http(#[from] HttpError),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Database(err.to_string())
    }
}

impl PublicHttpError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Http(err) => err.status_code(),
            Self::Config(_) | Self::Database(_) | Self::ExternalService { .. } => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn code(&self) -> Cow<'_, str> {
        match self {
            Self::Unauthorized(_) => "unauthorized".into(),
            Self::Forbidden(_) => "forbidden".into(),
            Self::Validation(_) => "validation_error".into(),
            Self::NotFound(_) => "not_found".into(),
            Self::Conflict(_) => "conflict".into(),
            Self::Http(err) => err.code(),
            Self::Config(_) | Self::Database(_) | Self::ExternalService { .. } => {
                "internal_error".into()
            }
        }
    }

    fn message(&self) -> Cow<'_, str> {
        match self {
            Self::Validation(message) | Self::Conflict(message) => message.as_str().into(),
            Self::Unauthorized(_) => "authentication required".into(),
            Self::Forbidden(_) => "not allowed".into(),
            Self::NotFound(_) => "not found".into(),
            Self::Http(err) => err.message(),
            Self::Config(_) | Self::Database(_) | Self::ExternalService { .. } => {
                "internal error".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_errors_expose_their_message() {
        let err = AppError::Validation("label is required".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.code(), "validation_error");
        assert_eq!(err.message(), "label is required");
    }

    #[test]
    fn test_internal_errors_hide_detail() {
        let err = AppError::Database("connection refused at 10.0.0.1".to_string());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message(), "internal error");
    }

    #[test]
    fn test_not_found_hides_which_resource() {
        let err = AppError::NotFound("piece boreal_pocket".to_string());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(err.message(), "not found");
    }

    #[test]
    fn test_auth_errors_map_to_401_and_403() {
        assert_eq!(
            AppError::Unauthorized(String::new()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::Forbidden(String::new()).status_code(),
            StatusCode::FORBIDDEN
        );
    }
}
