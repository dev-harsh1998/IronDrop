// SPDX-License-Identifier: MIT

//! Middleware system for request preprocessing (e.g. authentication).
//!
//! Provides a Basic Auth middleware that validates the `Authorization` header
//! when username & password are configured. If credentials are not configured
//! the middleware is a no-op.

use crate::error::AppError;
use crate::http::Request;
use base64::Engine;
use log::{trace, warn};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

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

        let Some(header) = auth_header else {
            auth_failure_rate_limited("missing authorization header");
            return false;
        };

        if constant_time_eq_bytes(header.as_bytes(), expected) {
            true
        } else {
            auth_failure_rate_limited("invalid credentials");
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

fn auth_failure_rate_limited(reason: &'static str) {
    static STATE: OnceLock<Mutex<(Instant, u64)>> = OnceLock::new();
    let state = STATE.get_or_init(|| Mutex::new((Instant::now() - Duration::from_secs(3600), 0)));
    if let Ok(mut st) = state.lock() {
        st.1 += 1;
        if st.0.elapsed() >= Duration::from_secs(20) {
            warn!(
                "Authentication failed ({reason}). failures_since_last_log={}",
                st.1
            );
            st.0 = Instant::now();
            st.1 = 0;
        }
    } else {
        warn!("Authentication failed ({reason}).");
    }
}
