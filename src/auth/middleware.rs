//! Authentication middleware and extractors.

use actix_web::{dev::Payload, FromRequest, HttpRequest};
use std::future::{ready, Ready};
use uuid::Uuid;

use super::jwt::{decode_token, Claims};
use crate::error::AppError;

/// Authenticated user extractor.
///
/// Use this as a parameter in route handlers to require authentication.
///
/// # Example
/// ```ignore
/// async fn protected_route(user: AuthenticatedUser) -> impl Responder {
///     format!("Hello, {}!", user.username)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// User ID.
    pub id: Uuid,
    /// Username.
    pub username: String,
    /// Whether the user is an admin.
    pub is_admin: bool,
}

impl AuthenticatedUser {
    /// Create from JWT claims.
    pub fn from_claims(claims: Claims) -> Self {
        Self {
            id: claims.sub,
            username: claims.username,
            is_admin: claims.is_admin,
        }
    }

    /// Check if the user has admin privileges.
    pub fn require_admin(&self) -> Result<(), AppError> {
        if self.is_admin {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "Admin privileges required".to_string(),
            ))
        }
    }
}

impl FromRequest for AuthenticatedUser {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(extract_user(req))
    }
}

/// Extract the authenticated user from request headers.
fn extract_user(req: &HttpRequest) -> Result<AuthenticatedUser, AppError> {
    // Get Authorization header
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            AppError::Unauthorized("Missing Authorization header".to_string())
        })?;

    // Parse Bearer token
    let token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .ok_or_else(|| {
            AppError::Unauthorized("Invalid Authorization header format. Expected: Bearer <token>".to_string())
        })?;

    // Decode and validate token
    let claims = decode_token(token)?;

    // Check expiration
    if claims.is_expired() {
        return Err(AppError::invalid_token());
    }

    Ok(AuthenticatedUser::from_claims(claims))
}

/// Optional authenticated user extractor.
///
/// Use this when authentication is optional - will return None if no valid token is present.
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<AuthenticatedUser>);

impl FromRequest for OptionalUser {
    type Error = AppError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        ready(Ok(OptionalUser(extract_user(req).ok())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    #[test]
    fn test_missing_auth_header() {
        let req = TestRequest::default().to_http_request();
        let result = extract_user(&req);

        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }

    #[test]
    fn test_invalid_auth_header_format() {
        let req = TestRequest::default()
            .insert_header(("Authorization", "Basic abc123"))
            .to_http_request();
        let result = extract_user(&req);

        assert!(matches!(result, Err(AppError::Unauthorized(_))));
    }
}
