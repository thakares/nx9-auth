use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// Central application error type.
/// All handlers return `Result<T, AppError>`, which Axum maps to HTTP responses.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("resource not found")]
    NotFound,

    #[error("invalid credentials")]
    Unauthorized,

    #[error("insufficient permissions")]
    Forbidden,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("too many requests")]
    RateLimited,

    #[error("internal error")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            AppError::InvalidInput(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_input"),
            AppError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            AppError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error"),
        };

        // Log server-side errors for visibility
        match &self {
            AppError::Database(e) => {
                tracing::error!(error = %e, "database error");
            }
            AppError::Internal => {
                tracing::error!("internal error");
            }
            _ => {}
        }

        let body = json!({
            "error": self.to_string(),
            "code": code,
        });

        (status, Json(body)).into_response()
    }
}

/// Convenience alias used throughout the codebase.
pub type Result<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_mapping() {
        let err_not_found = AppError::NotFound;
        let resp = err_not_found.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let err_unauthorized = AppError::Unauthorized;
        let resp = err_unauthorized.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let err_forbidden = AppError::Forbidden;
        let resp = err_forbidden.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        let err_conflict = AppError::Conflict("already exists".into());
        let resp = err_conflict.into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);

        let err_invalid = AppError::InvalidInput("bad value".into());
        let resp = err_invalid.into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let err_rate = AppError::RateLimited;
        let resp = err_rate.into_response();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
