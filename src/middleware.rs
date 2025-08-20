// SPDX-License-Identifier: MIT

//! Middleware system for request preprocessing (e.g. authentication).
//!
//! Provides a Basic Auth middleware that validates the `Authorization` header
//! when username & password are configured. If credentials are not configured
//! the middleware is a no-op.

use crate::error::AppError;
use crate::http::Request;
use base64::Engine;
use log::{debug, trace, warn};

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
            trace!("Authentication disabled - allowing request");
            return true; // auth disabled
        };

        debug!("Authentication required - checking credentials");

        let header = match auth_header {
            Some(h) => {
                trace!("Authorization header found");
                h
            }
            None => {
                debug!("No Authorization header provided");
                return false;
            }
        };
        let credentials = match header.strip_prefix("Basic ") {
            Some(c) => {
                trace!("Basic authentication scheme detected");
                c
            }
            None => {
                debug!("Invalid authentication scheme (not Basic)");
                return false;
            }
        };
        let decoded = match base64::engine::general_purpose::STANDARD.decode(credentials) {
            Ok(d) => {
                trace!("Successfully decoded base64 credentials");
                d
            }
            Err(_) => {
                debug!("Failed to decode base64 credentials");
                return false;
            }
        };
        let decoded_str = match String::from_utf8(decoded) {
            Ok(s) => {
                trace!("Successfully converted credentials to UTF-8");
                s
            }
            Err(_) => {
                debug!("Invalid UTF-8 in decoded credentials");
                return false;
            }
        };
        if let Some((provided_user, provided_pass)) = decoded_str.split_once(':') {
            trace!("Parsed username from credentials: '{}'", provided_user);
            let auth_result = provided_user == user && provided_pass == pass;
            if auth_result {
                debug!("Authentication successful for user: '{}'", provided_user);
            } else {
                warn!("Authentication failed for user: '{}'", provided_user);
            }
            auth_result
        } else {
            debug!("Invalid credential format (missing colon separator)");
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
