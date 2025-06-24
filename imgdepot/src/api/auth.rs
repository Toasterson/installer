use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum::extract::Query;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tracing::{error, info};

use crate::error::{AppError, Result};
use crate::storage::Storage;
use super::routes::AppState;

// JWT Claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub iss: String,
    pub aud: Option<String>,
    pub scope: Option<String>,
}

// Authentication configuration
pub struct AuthConfig {
    pub realm: String,
    pub service: String,
    pub issuer: String,
    pub token_expiration: Duration,
    pub signing_key: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            realm: "https://auth.example.com/token".to_string(),
            service: "registry.example.com".to_string(),
            issuer: "registry-auth".to_string(),
            token_expiration: Duration::hours(1),
            signing_key: "secret".to_string(), // In production, use a secure key
        }
    }
}

// Authentication middleware
pub async fn auth_middleware(
    State((_, _)): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    // Check if the request has an Authorization header
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        // Parse the Authorization header
        let auth_header = match auth_header.to_str() {
            Ok(header) => header,
            Err(_) => {
                return AppError::Unauthorized("Invalid Authorization header".to_string()).into_response();
            }
        };

        // Handle Bearer token
        if auth_header.starts_with("Bearer ") {
            let token = &auth_header["Bearer ".len()..];
            
            // Validate the token
            return match validate_token(token) {
                Ok(claims) => {
                    // Add claims to request extensions for later use
                    request.extensions_mut().insert(claims);
                    next.run(request).await
                }
                Err(err) => {
                    error!("Token validation failed: {}", err);
                    AppError::Unauthorized("Invalid token".to_string()).into_response()
                }
            }
        }
        // Handle Basic auth
        else if auth_header.starts_with("Basic ") {
            let credentials = &auth_header["Basic ".len()..];
            // TODO validate credentials against a configuration.
            
            // In a real implementation, validate the credentials against a database
            // For now, we'll accept any credentials
            info!("Basic auth accepted");
            return next.run(request).await;
        }
    }

    // If no Authorization header or invalid auth, return a WWW-Authenticate challenge
    let config = AuthConfig::default();
    let challenge = format!(
        r#"Bearer realm="{}", service="{}""#,
        config.realm, config.service
    );

    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, challenge)
        .body(axum::body::Body::empty())
        .unwrap()
}

// Generate a token for a user
pub fn generate_token(username: &str, scope: Option<String>) -> Result<String> {
    let config = AuthConfig::default();
    
    let now = OffsetDateTime::now_utc();
    let expiration = now + config.token_expiration;
    
    let claims = Claims {
        sub: username.to_string(),
        exp: expiration.unix_timestamp(),
        iat: now.unix_timestamp(),
        iss: config.issuer,
        aud: Some(config.service),
        scope,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.signing_key.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Token generation failed: {}", e)))?;
    
    Ok(token)
}

// Validate a token
fn validate_token(token: &str) -> Result<Claims> {
    let config = AuthConfig::default();
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.signing_key.as_bytes()),
        &Validation::default(),
    )
    .map_err(|e| AppError::Unauthorized(format!("Token validation failed: {}", e)))?;
    
    Ok(token_data.claims)
}

// Token endpoint handler
pub async fn token_handler(
    State((_, _)): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<TokenParams>,
) -> Result<impl IntoResponse> {
    // Check if the request has an Authorization header for basic auth
    let username = if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        let auth_header = auth_header.to_str().map_err(|_| AppError::Unauthorized("Invalid Authorization header".to_string()))?;
        
        if auth_header.starts_with("Basic ") {
            let credentials = &auth_header["Basic ".len()..];
            let decoded = base64::decode(credentials)
                .map_err(|_| AppError::Unauthorized("Invalid Basic auth".to_string()))?;
            
            let credentials_str = String::from_utf8(decoded)
                .map_err(|_| AppError::Unauthorized("Invalid Basic auth".to_string()))?;
            
            if let Some(colon_pos) = credentials_str.find(':') {
                credentials_str[..colon_pos].to_string()
            } else {
                return Err(AppError::Unauthorized("Invalid Basic auth".to_string()));
            }
        } else {
            return Err(AppError::Unauthorized("Basic auth required".to_string()));
        }
    } else {
        // Anonymous access
        "anonymous".to_string()
    };
    
    // Generate a token with the requested scope
    let token = generate_token(&username, params.scope)?;
    
    // Return the token response
    Ok(axum::Json(TokenResponse {
        token,
        expires_in: 3600, // 1 hour
        issued_at: chrono::Utc::now().to_rfc3339(),
    }))
}

// Token request parameters
#[derive(Debug, Deserialize)]
pub struct TokenParams {
    pub service: Option<String>,
    pub scope: Option<String>,
}

// Token response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub expires_in: u64,
    pub issued_at: String,
}