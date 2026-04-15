// SPDX-License-Identifier: MIT

//! Middleware system for request preprocessing (e.g. authentication).
//!
//! Provides a Basic Auth middleware that validates the `Authorization` header
//! when username & password are configured. If credentials are not configured
//! the middleware is a no-op.

use crate::error::AppError;
use crate::http::Request;
use base64::Engine;
use log::{debug, trace};

/// Middleware trait – middlewares can inspect a request before it reaches a handler.
/// Returning `Ok(())` continues the chain; returning `Err(AppError)` aborts processing.
pub trait Middleware: Send + Sync + 'static {
    fn handle(&self, request: &Request) -> Result<(), AppError>;
}

/// Basic authentication middleware.
pub struct AuthMiddleware {
    pub username: Option<String>,
    pub password: Option<String>,
    expected_authorization: Option<Vec<u8>>,
}

impl AuthMiddleware {
    pub fn new(username: Option<String>, password: Option<String>) -> Self {
        let expected_authorization = match (&username, &password) {
            (Some(user), Some(pass)) => {
                let raw = format!("{user}:{pass}");
                let encoded = base64::engine::general_purpose::STANDARD.encode(raw.as_bytes());
                Some(format!("Basic {encoded}").into_bytes())
            }
            _ => None,
        };
        Self {
            username,
            password,
            expected_authorization,
        }
    }

    fn is_authenticated(&self, auth_header: Option<&String>) -> bool {
        let Some(expected) = &self.expected_authorization else {
            trace!("Authentication disabled - allowing request");
            return true; // auth disabled
        };

        debug!("Authentication required - checking credentials");

        let Some(header) = auth_header else {
            debug!("No Authorization header provided");
            return false;
        };

        constant_time_eq_bytes(header.as_bytes(), expected)
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

fn constant_time_eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
