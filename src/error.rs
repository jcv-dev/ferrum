//! Application error types and handling.
//!
//! Provides structured error responses for the API.

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::Serialize;

/// API error response body.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error type/code.
    pub error: String,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    /// Create a new error response.
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Add details to the error response.
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Application error types.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Resource not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Authentication required or failed.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Permission denied.
    #[error("Forbidden: {0}")]
    Forbidden(String),

    /// Validation error.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Resource already exists.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Bad request.
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Internal server error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl AppError {
    /// Get the error code string.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "NOT_FOUND",
            Self::Unauthorized(_) => "UNAUTHORIZED",
            Self::Forbidden(_) => "FORBIDDEN",
            Self::Validation(_) => "VALIDATION_ERROR",
            Self::Conflict(_) => "CONFLICT",
            Self::BadRequest(_) => "BAD_REQUEST",
            Self::Internal(_) => "INTERNAL_ERROR",
            Self::Io(_) => "IO_ERROR",
            Self::Json(_) => "JSON_ERROR",
        }
    }

    /// Create an unauthorized error for invalid credentials.
    pub fn invalid_credentials() -> Self {
        Self::Unauthorized("Invalid username or password".to_string())
    }

    /// Create an unauthorized error for invalid token.
    pub fn invalid_token() -> Self {
        Self::Unauthorized("Invalid or expired token".to_string())
    }

    /// Create a not found error for a song.
    pub fn song_not_found(filename: &str) -> Self {
        Self::NotFound(format!("Song not found: {}", filename))
    }

    /// Create a validation error for path traversal attempt.
    pub fn path_traversal() -> Self {
        Self::BadRequest("Invalid path: path traversal not allowed".to_string())
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Internal(_) | Self::Io(_) | Self::Json(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_response = ErrorResponse::new(self.error_code(), self.to_string());

        tracing::error!(
            error_code = %self.error_code(),
            status = %status.as_u16(),
            message = %self.to_string(),
            "API error"
        );

        HttpResponse::build(status).json(error_response)
    }
}

/// Result type alias using AppError.
pub type AppResult<T> = Result<T, AppError>;

/// Extension trait for converting Option to AppResult.
pub trait OptionExt<T> {
    /// Convert None to NotFound error.
    fn ok_or_not_found(self, msg: impl Into<String>) -> AppResult<T>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_not_found(self, msg: impl Into<String>) -> AppResult<T> {
        self.ok_or_else(|| AppError::NotFound(msg.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(AppError::NotFound("test".into()).error_code(), "NOT_FOUND");
        assert_eq!(
            AppError::Unauthorized("test".into()).error_code(),
            "UNAUTHORIZED"
        );
        assert_eq!(AppError::Forbidden("test".into()).error_code(), "FORBIDDEN");
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            AppError::NotFound("test".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::Unauthorized("test".into()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            AppError::Internal("test".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse::new("TEST_ERROR", "Test message");
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("TEST_ERROR"));
        assert!(json.contains("Test message"));
    }
}
