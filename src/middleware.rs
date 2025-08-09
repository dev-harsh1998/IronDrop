//! Middleware system for request preprocessing (e.g. authentication).
//!
//! Provides a Basic Auth middleware that validates the `Authorization` header
//! when username & password are configured. If credentials are not configured
//! the middleware is a no-op.

use crate::error::AppError;
use crate::http::Request;
use base64::Engine;

/// Middleware trait – middlewares can inspect a request before it reaches a handler.
/// Returning `Ok(())` continues the chain; returning `Err(AppError)` aborts processing.
pub trait Middleware: Send + Sync + 'static {
    fn handle(&self, request: &Request) -> Result<(), AppError>;
}

/// Basic authentication middleware.
pub struct AuthMiddleware {
    pub username: Option<String>,
    pub password: Option<String>,
}

impl AuthMiddleware {
    pub fn new(username: Option<String>, password: Option<String>) -> Self {
        Self { username, password }
    }

    fn is_authenticated(&self, auth_header: Option<&String>) -> bool {
        let (Some(user), Some(pass)) = (&self.username, &self.password) else {
            return true; // auth disabled
        };

        let header = match auth_header {
            Some(h) => h,
            None => return false,
        };
        let credentials = match header.strip_prefix("Basic ") {
            Some(c) => c,
            None => return false,
        };
        let decoded = match base64::engine::general_purpose::STANDARD.decode(credentials) {
            Ok(d) => d,
            Err(_) => return false,
        };
        let decoded_str = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(_) => return false,
        };
        if let Some((provided_user, provided_pass)) = decoded_str.split_once(':') {
            provided_user == user && provided_pass == pass
        } else {
            false
        }
    }
}

impl Middleware for AuthMiddleware {
    fn handle(&self, request: &Request) -> Result<(), AppError> {
        if self.username.is_some()
            && self.password.is_some()
            && !self.is_authenticated(request.headers.get("authorization"))
        {
            return Err(AppError::Unauthorized);
        }
        Ok(())
    }
}
