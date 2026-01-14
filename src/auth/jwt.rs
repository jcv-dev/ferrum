//! JWT token handling.

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config;
use crate::error::{AppError, AppResult};

/// JWT claims payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID).
    pub sub: Uuid,
    /// Username.
    pub username: String,
    /// Whether user is admin.
    pub is_admin: bool,
    /// Expiration time (Unix timestamp).
    pub exp: i64,
    /// Issued at time (Unix timestamp).
    pub iat: i64,
}

impl Claims {
    /// Create new claims for a user.
    pub fn new(user_id: Uuid, username: String, is_admin: bool, expiry_days: i64) -> Self {
        let now = Utc::now();
        let exp = now + Duration::days(expiry_days);

        Self {
            sub: user_id,
            username,
            is_admin,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        }
    }

    /// Check if the token has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }
}

/// Token pair (for future refresh token support).
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    /// Access token.
    pub access_token: String,
    /// Token type (always "Bearer").
    pub token_type: String,
    /// Expiration time in seconds.
    pub expires_in: i64,
}

/// Encode a JWT token.
pub fn encode_token(claims: &Claims) -> AppResult<String> {
    let config = config::get();
    let key = EncodingKey::from_secret(config.jwt_secret.as_bytes());

    encode(&Header::default(), claims, &key).map_err(|e| {
        tracing::error!(error = %e, "Failed to encode JWT");
        AppError::Internal("Failed to generate token".to_string())
    })
}

/// Decode and validate a JWT token.
pub fn decode_token(token: &str) -> AppResult<Claims> {
    let config = config::get();
    let key = DecodingKey::from_secret(config.jwt_secret.as_bytes());
    let validation = Validation::default();

    decode::<Claims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| {
            tracing::debug!(error = %e, "Failed to decode JWT");
            AppError::invalid_token()
        })
}

/// Create a new token pair for a user.
pub fn create_token_pair(
    user_id: Uuid,
    username: String,
    is_admin: bool,
) -> AppResult<TokenPair> {
    let config = config::get();
    let expiry_days = config.jwt_expiry_days;

    let claims = Claims::new(user_id, username, is_admin, expiry_days);
    let access_token = encode_token(&claims)?;

    Ok(TokenPair {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: expiry_days * 24 * 60 * 60, // Convert days to seconds
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_config() {
        std::env::set_var("JWT_SECRET", "test-secret-key-for-testing-purposes-only");
        std::env::set_var("MUSIC_FOLDER", ".");
        let _ = config::init();
    }

    #[test]
    fn test_claims_creation() {
        let claims = Claims::new(
            Uuid::new_v4(),
            "testuser".to_string(),
            false,
            7,
        );

        assert!(!claims.is_expired());
        assert_eq!(claims.username, "testuser");
        assert!(!claims.is_admin);
    }

    #[test]
    fn test_token_roundtrip() {
        init_test_config();

        let user_id = Uuid::new_v4();
        let claims = Claims::new(user_id, "testuser".to_string(), true, 7);
        let token = encode_token(&claims).unwrap();
        let decoded = decode_token(&token).unwrap();

        assert_eq!(decoded.sub, user_id);
        assert_eq!(decoded.username, "testuser");
        assert!(decoded.is_admin);
    }
}
