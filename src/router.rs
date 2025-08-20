//! Simple router abstraction for registering and matching request handlers.
//!
//! This initial implementation supports:
//! - Exact path matching (e.g. "/_health")
//! - Prefix path matching (useful for static asset directories)
//! - Method filtering (GET/POST/etc.)
//!
//! Handlers are stored as boxed closures capturing any required state.
//! The router is lightweight and intended to be constructed either once
//! at startup (recommended future optimization) or ad-hoc per request
//! for now while integrating with existing code.
//!
//! Future enhancements that would be beneficial:
//! - Path parameters (e.g. /files/:id)
//! - Glob or regex based matching
//! - Middleware (before/after hooks)
//! - A fallback / not-found handler override
//! - Caching / static router built once and shared via Arc
//!
//! For current use cases we keep it intentionally small and dependency free.

use crate::error::AppError;
use crate::http::{Request, Response};
use crate::middleware::Middleware;
use log::{debug, trace};

/// Type alias for a request handler closure.
pub type Handler = Box<dyn Fn(&Request) -> Result<Response, AppError> + Send + Sync + 'static>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum MatchKind {
    Exact,
    Prefix,
}

struct RouteEntry {
    method: String,
    path: String,
    kind: MatchKind,
    handler: Handler,
}

/// A minimal router storing registered routes and resolving them for incoming requests.
#[derive(Default)]
pub struct Router {
    routes: Vec<RouteEntry>,
    middleware: Vec<Box<dyn Middleware>>, // global middleware executed in order
}

impl Router {
    /// Create a new empty router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            middleware: Vec::new(),
        }
    }

    /// Register an exact path match for the given HTTP method.
    pub fn register_exact<M, P>(&mut self, method: M, path: P, handler: Handler)
    where
        M: Into<String>,
        P: Into<String>,
    {
        self.routes.push(RouteEntry {
            method: method.into().to_uppercase(),
            path: path.into(),
            kind: MatchKind::Exact,
            handler,
        });
    }

    /// Register a prefix path match for the given HTTP method.
    /// Any request whose path starts with the provided prefix will match.
    pub fn register_prefix<M, P>(&mut self, method: M, prefix: P, handler: Handler)
    where
        M: Into<String>,
        P: Into<String>,
    {
        self.routes.push(RouteEntry {
            method: method.into().to_uppercase(),
            path: prefix.into(),
            kind: MatchKind::Prefix,
            handler,
        });
    }

    /// Add a global middleware executed before any handler.
    pub fn add_middleware(&mut self, mw: Box<dyn Middleware>) {
        self.middleware.push(mw);
    }

    /// Attempt to resolve a request to a registered route.
    /// Returns Some(Result<..>) if a route matched, or None if no route matched.
    pub fn route(&self, request: &Request) -> Option<Result<Response, AppError>> {
        debug!("Routing request: {} {}", request.method, request.path);
        trace!("Available routes: {}", self.routes.len());

        // Run middleware chain first
        for mw in &self.middleware {
            if let Err(e) = mw.handle(request) {
                debug!("Middleware rejected request: {:?}", e);
                return Some(Err(e));
            }
        }
        trace!("Middleware chain passed for request");

        let method = request.method.to_uppercase();
        // Match against the path without query string so routes like "/_irondrop/upload?x=y" work
        let path_only = if let Some(pos) = request.path.find('?') {
            &request.path[..pos]
        } else {
            request.path.as_str()
        };
        for entry in &self.routes {
            if entry.method != method {
                continue;
            }
            let is_match = match entry.kind {
                MatchKind::Exact => path_only == entry.path,
                MatchKind::Prefix => path_only.starts_with(&entry.path),
            };
            if is_match {
                debug!(
                    "Route matched: {} {} ({:?})",
                    entry.method, entry.path, entry.kind
                );
                return Some((entry.handler)(request));
            }
        }

        debug!("No route matched for: {} {}", request.method, request.path);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::ResponseBody;
    use std::collections::HashMap;

    fn dummy_request(method: &str, path: &str) -> Request {
        Request {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }

    #[test]
    fn test_exact_route_matching() {
        let mut router = Router::new();
        router.register_exact(
            "GET",
            "/health",
            Box::new(|_| {
                Ok(Response {
                    status_code: 200,
                    status_text: "OK".into(),
                    headers: HashMap::new(),
                    body: ResponseBody::Text("ok".into()),
                })
            }),
        );

        let req = dummy_request("GET", "/health");
        let resp = router.route(&req).unwrap().unwrap();
        assert_eq!(resp.status_code, 200);
    }

    #[test]
    fn test_prefix_route_matching() {
        let mut router = Router::new();
        router.register_prefix(
            "GET",
            "/static/",
            Box::new(|r| {
                Ok(Response {
                    status_code: 200,
                    status_text: r.path.clone(),
                    headers: HashMap::new(),
                    body: ResponseBody::Text("prefix".into()),
                })
            }),
        );

        let req = dummy_request("GET", "/static/app.js");
        let resp = router.route(&req).unwrap().unwrap();
        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.status_text, "/static/app.js");
    }

    #[test]
    fn test_method_is_respected() {
        let mut router = Router::new();
        router.register_exact(
            "GET",
            "/onlyget",
            Box::new(|_| {
                Ok(Response {
                    status_code: 200,
                    status_text: "GET".into(),
                    headers: HashMap::new(),
                    body: ResponseBody::Text("g".into()),
                })
            }),
        );

        let req = dummy_request("POST", "/onlyget");
        assert!(router.route(&req).is_none());
    }

    #[test]
    fn test_querystring_is_ignored_in_matching() {
        let mut router = Router::new();
        router.register_exact(
            "GET",
            "/_irondrop/upload",
            Box::new(|r| {
                Ok(Response {
                    status_code: 200,
                    status_text: r.path.clone(),
                    headers: HashMap::new(),
                    body: ResponseBody::Text("ok".into()),
                })
            }),
        );

        let req = dummy_request("GET", "/_irondrop/upload?upload_to=abcd");
        let resp = router.route(&req).unwrap().unwrap();
        assert_eq!(resp.status_code, 200);
    }
}
