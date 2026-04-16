// SPDX-License-Identifier: MIT

use crate::error::AppError;
use crate::http::{Request, RequestBody, Response, ResponseBody};
use log::{debug, trace};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct DavLock {
    token: String,
    expires_at_epoch_secs: u64,
    timeout_secs: u64,
    depth_infinity: bool,
    lockroot_href: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DavDepth {
    Zero,
    One,
    Infinity,
}

#[derive(Debug, Clone)]
enum PropfindMode {
    AllProp,
    PropName,
    Named(Vec<PropName>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyDepth {
    Zero,
    Infinity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PropPatchAction {
    Set,
    Remove,
}

#[derive(Debug, Clone)]
struct PropPatchOperation {
    action: PropPatchAction,
    props: Vec<(PropName, Option<String>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PropName {
    namespace: String,
    local_name: String,
}

static DAV_LOCKS: OnceLock<Mutex<HashMap<String, DavLock>>> = OnceLock::new();
static DAV_DEAD_PROPERTIES: OnceLock<Mutex<HashMap<String, HashMap<PropName, String>>>> =
    OnceLock::new();
static DAV_OP_GUARD: OnceLock<Mutex<()>> = OnceLock::new();
static LOCK_COUNTER: AtomicU64 = AtomicU64::new(1);
const DAV_NAMESPACE: &str = "DAV:";

pub fn handle_webdav_request(
    request: &Request,
    base_dir: &Path,
    _allowed_extensions: &[glob::Pattern],
) -> Result<Response, AppError> {
    match request.method.as_str() {
        "OPTIONS" => Ok(build_options_response()),
        "PROPFIND" => handle_propfind(request, base_dir),
        "MKCOL" => handle_mkcol(request, base_dir),
        "PUT" => handle_put(request, base_dir),
        "DELETE" => handle_delete(request, base_dir),
        "COPY" => handle_copy_or_move(request, base_dir, false),
        "MOVE" => handle_copy_or_move(request, base_dir, true),
        "PROPPATCH" => handle_proppatch(request, base_dir),
        "LOCK" => handle_lock(request, base_dir),
        "UNLOCK" => handle_unlock(request, base_dir),
        _ => Err(AppError::MethodNotAllowed),
    }
}

pub fn allow_header_value() -> &'static str {
    "OPTIONS, GET, HEAD, PROPFIND, PROPPATCH, MKCOL, PUT, DELETE, COPY, MOVE, LOCK, UNLOCK"
}

fn build_options_response() -> Response {
    let mut headers = HashMap::new();
    headers.insert("DAV".to_string(), "1,2".to_string());
    headers.insert("Allow".to_string(), allow_header_value().to_string());
    headers.insert("MS-Author-Via".to_string(), "DAV".to_string());
    Response {
        status_code: 200,
        status_text: "OK".to_string(),
        headers,
        body: ResponseBody::Text(String::new()),
    }
}

fn handle_propfind(request: &Request, base_dir: &Path) -> Result<Response, AppError> {
    let depth = parse_depth_header(&request.headers)?;
    let mode = parse_propfind_mode(request)?;
    if crate::utils::is_macos_finder_noise_path(&request.path) {
        let fast_path = resolve_request_path_without_canonicalize(base_dir, &request.path)?;
        if !fast_path.exists() {
            return Ok(status_response(404, "Not Found"));
        }
    }
    let target_path = resolve_request_path(base_dir, &request.path)?;

    if !target_path.exists() {
        return Err(AppError::NotFound);
    }

    let mut resources = vec![target_path.clone()];
    if target_path.is_dir() {
        match depth {
            DavDepth::Zero => {}
            DavDepth::One => {
                let mut entries = Vec::new();
                for entry in std::fs::read_dir(&target_path)? {
                    let entry = entry?;
                    entries.push(entry.path());
                }
                entries.sort_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
                resources.extend(entries);
            }
            DavDepth::Infinity => {
                collect_recursive_resources(&target_path, &mut resources)?;
            }
        }
    }

    let mut body = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
"#,
    );

    for resource in resources {
        append_multistatus_response(&mut body, base_dir, &resource, &mode)?;
    }
    body.push_str("</D:multistatus>\n");

    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/xml; charset=utf-8".to_string(),
    );
    headers.insert("DAV".to_string(), "1,2".to_string());
    headers.insert("Allow".to_string(), allow_header_value().to_string());

    Ok(Response {
        status_code: 207,
        status_text: "Multi-Status".to_string(),
        headers,
        body: ResponseBody::Text(body),
    })
}

fn parse_propfind_mode(request: &Request) -> Result<PropfindMode, AppError> {
    let body = request_body_bytes(request)?;
    if body.is_empty() {
        return Ok(PropfindMode::AllProp);
    }

    let body_str = std::str::from_utf8(&body).map_err(|_| AppError::BadRequest)?;
    if contains_named_element(body_str, "propname") {
        return Ok(PropfindMode::PropName);
    }
    if contains_named_element(body_str, "allprop") {
        return Ok(PropfindMode::AllProp);
    }
    let requested = parse_requested_prop_names(body_str);
    if !requested.is_empty() {
        return Ok(PropfindMode::Named(requested));
    }
    // RFC default for empty propfind body shape or unknown child shape is allprop.
    if contains_named_element(body_str, "propfind") {
        return Ok(PropfindMode::AllProp);
    }

    Err(AppError::BadRequest)
}

fn contains_named_element(xml: &str, local_name: &str) -> bool {
    let local_name = local_name.to_ascii_lowercase();
    let mut idx = 0usize;
    while let Some(lt_rel) = xml[idx..].find('<') {
        let lt = idx + lt_rel;
        let Some(gt_rel) = xml[lt..].find('>') else {
            break;
        };
        let gt = lt + gt_rel;
        let token = xml[lt + 1..gt].trim();
        idx = gt + 1;
        if token.is_empty()
            || token.starts_with('/')
            || token.starts_with('?')
            || token.starts_with('!')
        {
            continue;
        }
        let raw = token
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');
        let local = raw
            .split(':')
            .next_back()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if local == local_name {
            return true;
        }
    }
    false
}

fn parse_requested_prop_names(xml: &str) -> Vec<PropName> {
    let lowered = xml.to_ascii_lowercase();
    let prop_open = lowered.find("<d:prop").or_else(|| lowered.find("<prop"));
    let Some(start) = prop_open else {
        return Vec::new();
    };
    let Some(open_end_rel) = lowered[start..].find('>') else {
        return Vec::new();
    };
    let open_end = start + open_end_rel + 1;
    let close_idx = lowered[open_end..]
        .find("</d:prop>")
        .or_else(|| lowered[open_end..].find("</prop>"))
        .map(|idx| open_end + idx)
        .unwrap_or(xml.len());

    let mut namespace_map = parse_xmlns_mappings(xml);
    let prop_open_tag = &xml[start + 1..open_end - 1];
    for (prefix, uri) in parse_xmlns_mappings_from_tag(prop_open_tag) {
        namespace_map.insert(prefix, uri);
    }

    let inner = &xml[open_end..close_idx];
    let mut names = Vec::new();
    let mut idx = 0usize;
    while let Some(lt_rel) = inner[idx..].find('<') {
        let lt = idx + lt_rel;
        let Some(gt_rel) = inner[lt..].find('>') else {
            break;
        };
        let gt = lt + gt_rel;
        let token = inner[lt + 1..gt].trim();
        idx = gt + 1;
        if token.is_empty()
            || token.starts_with('/')
            || token.starts_with('?')
            || token.starts_with('!')
        {
            continue;
        }

        for (prefix, uri) in parse_xmlns_mappings_from_tag(token) {
            namespace_map.insert(prefix, uri);
        }
        let Some(name) = parse_prop_name_from_token(token, &namespace_map) else {
            continue;
        };
        if name.local_name == "prop" {
            continue;
        }
        names.push(name);
    }
    names
}

fn parse_xmlns_mappings(xml: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("d".to_string(), DAV_NAMESPACE.to_string());
    map.insert(String::new(), DAV_NAMESPACE.to_string());
    let mut idx = 0usize;
    while let Some(start_rel) = xml[idx..].find("xmlns") {
        let start = idx + start_rel;
        let rest = &xml[start..];
        let Some(eq_rel) = rest.find('=') else {
            break;
        };
        let key = rest[..eq_rel].trim();
        let quote_start = start + eq_rel + 1;
        let quote_char = xml[quote_start..].chars().next().unwrap_or('"');
        if quote_char != '"' && quote_char != '\'' {
            idx = quote_start + 1;
            continue;
        }
        let after_quote = quote_start + 1;
        let Some(end_rel) = xml[after_quote..].find(quote_char) else {
            break;
        };
        let value = &xml[after_quote..after_quote + end_rel];
        let prefix = if let Some(p) = key.strip_prefix("xmlns:") {
            p.trim().to_ascii_lowercase()
        } else if key == "xmlns" {
            String::new()
        } else {
            idx = after_quote + end_rel + 1;
            continue;
        };
        map.insert(prefix, value.to_string());
        idx = after_quote + end_rel + 1;
    }
    map
}

fn parse_xmlns_mappings_from_tag(tag: &str) -> HashMap<String, String> {
    parse_xmlns_mappings(tag)
}

fn parse_prop_name_from_token(
    token: &str,
    namespace_map: &HashMap<String, String>,
) -> Option<PropName> {
    let raw = token
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches('/')
        .trim();
    if raw.is_empty() {
        return None;
    }
    let (prefix, local_name) = if let Some((p, local)) = raw.split_once(':') {
        (p.to_ascii_lowercase(), local.to_ascii_lowercase())
    } else {
        (String::new(), raw.to_ascii_lowercase())
    };
    if local_name.is_empty() {
        return None;
    }
    let namespace = namespace_map
        .get(&prefix)
        .cloned()
        .unwrap_or_else(|| DAV_NAMESPACE.to_string());
    Some(PropName {
        namespace,
        local_name,
    })
}

fn collect_recursive_resources(root: &Path, resources: &mut Vec<PathBuf>) -> Result<(), AppError> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(current_dir) = stack.pop() {
        let mut children = Vec::new();
        for entry in std::fs::read_dir(&current_dir)? {
            let entry = entry?;
            children.push((entry.path(), entry.file_type()?));
        }
        children.sort_by(|(a, _), (b, _)| a.to_string_lossy().cmp(&b.to_string_lossy()));

        for (entry_path, file_type) in children {
            resources.push(entry_path.clone());
            if file_type.is_dir() {
                stack.push(entry_path);
            }
        }
    }
    Ok(())
}

fn handle_mkcol(request: &Request, base_dir: &Path) -> Result<Response, AppError> {
    let _op_guard = op_guard()
        .lock()
        .map_err(|_| AppError::InternalServerError("dav operation guard poisoned".to_string()))?;
    let target_path = resolve_request_path(base_dir, &request.path)?;
    if let Some(response) = lock_precondition_response(request, &target_path) {
        debug!(
            "WebDAV MKCOL blocked by lock target={} status={}",
            target_path.display(),
            response.status_code
        );
        return Ok(response);
    }

    if target_path.exists() {
        return Ok(status_response(405, "Method Not Allowed"));
    }
    if !request_body_bytes(request)?.is_empty() {
        // We do not implement request-body extensions for MKCOL, so reject
        // non-empty bodies instead of silently ignoring them.
        return Ok(status_response(415, "Unsupported Media Type"));
    }

    let Some(parent) = target_path.parent() else {
        return Ok(status_response(409, "Conflict"));
    };
    if !parent.exists() || !parent.is_dir() {
        return Ok(status_response(409, "Conflict"));
    }

    std::fs::create_dir(&target_path)?;
    Ok(status_response(201, "Created"))
}

fn handle_put(request: &Request, base_dir: &Path) -> Result<Response, AppError> {
    let _op_guard = op_guard()
        .lock()
        .map_err(|_| AppError::InternalServerError("dav operation guard poisoned".to_string()))?;
    let target_path = resolve_request_path(base_dir, &request.path)?;
    if let Some(response) = lock_precondition_response(request, &target_path) {
        debug!(
            "WebDAV PUT blocked by lock target={} status={}",
            target_path.display(),
            response.status_code
        );
        return Ok(response);
    }

    if target_path == base_dir {
        return Err(AppError::Forbidden);
    }

    let Some(parent) = target_path.parent() else {
        return Ok(status_response(409, "Conflict"));
    };
    if !parent.exists() || !parent.is_dir() {
        return Ok(status_response(409, "Conflict"));
    }
    if target_path.exists() && target_path.is_dir() {
        return Ok(status_response(405, "Method Not Allowed"));
    }

    let existed = target_path.exists();
    let body_bytes = request_body_bytes(request)?;

    let mut file = std::fs::File::create(&target_path)?;
    file.write_all(&body_bytes)?;
    file.sync_all()?;

    if existed {
        Ok(status_response(204, "No Content"))
    } else {
        Ok(status_response(201, "Created"))
    }
}

fn handle_delete(request: &Request, base_dir: &Path) -> Result<Response, AppError> {
    let _op_guard = op_guard()
        .lock()
        .map_err(|_| AppError::InternalServerError("dav operation guard poisoned".to_string()))?;
    let target_path = resolve_request_path(base_dir, &request.path)?;
    debug!("WebDAV DELETE target={}", target_path.display());
    if let Some(response) = lock_precondition_response(request, &target_path) {
        debug!(
            "WebDAV DELETE blocked by lock target={} status={}",
            target_path.display(),
            response.status_code
        );
        return Ok(response);
    }

    if target_path == base_dir {
        return Err(AppError::Forbidden);
    }
    if !target_path.exists() {
        return Err(AppError::NotFound);
    }
    validate_delete_depth_header(&request.headers, target_path.is_dir())?;

    if target_path.is_dir() {
        let locked_descendants = locked_descendants_without_token(request, &target_path);
        if !locked_descendants.is_empty() {
            debug!(
                "WebDAV DELETE returns multistatus target={} locked_descendants={}",
                target_path.display(),
                locked_descendants.len()
            );
            return Ok(delete_locked_multistatus_response(
                base_dir,
                &target_path,
                &locked_descendants,
            ));
        }
    }

    if target_path.is_dir() {
        std::fs::remove_dir_all(&target_path)?;
        remove_locks_for_subtree(&target_path);
        remove_dead_props_for_subtree(&target_path);
    } else {
        std::fs::remove_file(&target_path)?;
        remove_lock_for_exact_path(&target_path);
        remove_dead_prop_for_exact_path(&target_path);
    }

    debug!("WebDAV DELETE success target={}", target_path.display());
    Ok(status_response(204, "No Content"))
}

fn handle_copy_or_move(
    request: &Request,
    base_dir: &Path,
    is_move: bool,
) -> Result<Response, AppError> {
    let _op_guard = op_guard()
        .lock()
        .map_err(|_| AppError::InternalServerError("dav operation guard poisoned".to_string()))?;
    let source = resolve_request_path(base_dir, &request.path)?;
    debug!(
        "WebDAV {} source={}",
        if is_move { "MOVE" } else { "COPY" },
        source.display()
    );
    if let Some(response) = lock_precondition_response(request, &source) {
        debug!(
            "WebDAV {} blocked by source lock source={} status={}",
            if is_move { "MOVE" } else { "COPY" },
            source.display(),
            response.status_code
        );
        return Ok(response);
    }
    if source == base_dir {
        return Err(AppError::Forbidden);
    }
    if !source.exists() {
        return Err(AppError::NotFound);
    }
    if is_move {
        validate_move_depth_header(&request.headers, source.is_dir())?;
    }

    let destination_header = request
        .headers
        .get("destination")
        .ok_or(AppError::BadRequest)?;
    let destination_request_path = extract_destination_path(
        destination_header,
        request.headers.get("host").map(String::as_str),
    )?;
    let destination = resolve_request_path(base_dir, &destination_request_path)?;
    debug!(
        "WebDAV {} destination_request_path={} destination={}",
        if is_move { "MOVE" } else { "COPY" },
        destination_request_path,
        destination.display()
    );
    if let Some(response) = lock_precondition_response(request, &destination) {
        debug!(
            "WebDAV {} blocked by destination lock destination={} status={}",
            if is_move { "MOVE" } else { "COPY" },
            destination.display(),
            response.status_code
        );
        return Ok(response);
    }
    if destination == base_dir {
        return Err(AppError::Forbidden);
    }
    if source == destination || is_descendant_or_same(&destination, &source) {
        return Err(AppError::BadRequest);
    }

    let Some(parent) = destination.parent() else {
        return Ok(status_response(409, "Conflict"));
    };
    if !parent.exists() || !parent.is_dir() {
        if is_move && is_finder_archive_related_move(&source, &destination_request_path) {
            debug!(
                "WebDAV MOVE creating missing destination parent for Finder temp source={} destination_parent={}",
                source.display(),
                parent.display()
            );
            std::fs::create_dir_all(parent)?;
        } else {
            return Ok(status_response(409, "Conflict"));
        }
    }

    let overwrite = request
        .headers
        .get("overwrite")
        .map(|value| !value.trim().eq_ignore_ascii_case("F"))
        .unwrap_or(true);
    let destination_exists = destination.exists();
    if destination_exists && !overwrite {
        debug!(
            "WebDAV {} overwrite denied source={} destination={}",
            if is_move { "MOVE" } else { "COPY" },
            source.display(),
            destination.display()
        );
        return Ok(status_response(412, "Precondition Failed"));
    }

    if destination_exists {
        if destination.is_dir() {
            std::fs::remove_dir_all(&destination)?;
            remove_locks_for_subtree(&destination);
            remove_dead_props_for_subtree(&destination);
        } else {
            std::fs::remove_file(&destination)?;
            remove_lock_for_exact_path(&destination);
            remove_dead_prop_for_exact_path(&destination);
        }
    }

    if is_move {
        if std::fs::rename(&source, &destination).is_err() {
            copy_path_recursive(&source, &destination)?;
            if source.is_dir() {
                std::fs::remove_dir_all(&source)?;
            } else {
                std::fs::remove_file(&source)?;
            }
        }
        move_locks_for_subtree(&source, &destination);
        move_dead_props(&source, &destination);
    } else {
        let depth = parse_copy_depth_header(&request.headers)?;
        trace!(
            "WebDAV COPY depth={:?} source={} destination={}",
            depth,
            source.display(),
            destination.display()
        );
        if source.is_dir() && depth == CopyDepth::Zero {
            std::fs::create_dir_all(&destination)?;
        } else {
            copy_path_recursive(&source, &destination)?;
        }
        copy_dead_props(&source, &destination);
    }

    if destination_exists {
        Ok(status_response(204, "No Content"))
    } else {
        Ok(status_response(201, "Created"))
    }
}

fn parse_copy_depth_header(headers: &HashMap<String, String>) -> Result<CopyDepth, AppError> {
    let depth = headers
        .get("depth")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "infinity".to_string());
    match depth.as_str() {
        "0" => Ok(CopyDepth::Zero),
        "infinity" => Ok(CopyDepth::Infinity),
        _ => Err(AppError::BadRequest),
    }
}

fn handle_lock(request: &Request, base_dir: &Path) -> Result<Response, AppError> {
    let _op_guard = op_guard()
        .lock()
        .map_err(|_| AppError::InternalServerError("dav operation guard poisoned".to_string()))?;
    let target_path = resolve_request_path(base_dir, &request.path)?;
    let key = lock_key(&target_path);
    cleanup_expired_locks();
    let lock_body = request_body_bytes(request)?;
    debug!("WebDAV LOCK target={}", target_path.display());

    let target_is_collection = target_path.is_dir() || request.path.ends_with('/');
    let depth_infinity = parse_lock_depth(&request.headers, target_is_collection)?;
    let timeout_secs = parse_lock_timeout_secs(&request.headers).unwrap_or(600);
    let now = now_epoch_secs();
    let expires_at = now.saturating_add(timeout_secs);
    let lockroot_href = build_href(base_dir, &target_path, target_is_collection);

    let token = format!(
        "opaquelocktoken:{:x}-{:x}",
        now,
        LOCK_COUNTER.fetch_add(1, Ordering::Relaxed)
    );

    let map = locks_map();
    let mut guard = map
        .lock()
        .map_err(|_| AppError::InternalServerError("lock map poisoned".to_string()))?;
    if let Some(existing) = guard.get(&key)
        && existing.expires_at_epoch_secs > now
    {
        if token_present_for_request(request, &existing.token, &request.path) {
            debug!(
                "WebDAV LOCK refresh accepted target={}",
                target_path.display()
            );
            let refresh_lock = existing.clone();
            if let Some(existing_mut) = guard.get_mut(&key) {
                existing_mut.expires_at_epoch_secs = expires_at;
                existing_mut.timeout_secs = timeout_secs;
            }
            drop(guard);
            return Ok(lock_success_response(
                &refresh_lock.token,
                timeout_secs,
                refresh_lock.depth_infinity,
                &refresh_lock.lockroot_href,
                true,
            ));
        }
        debug!(
            "WebDAV LOCK denied existing lock target={}",
            target_path.display()
        );
        return Ok(status_response(423, "Locked"));
    }
    if let Some(invalid_response) = validate_new_lock_request(&lock_body) {
        debug!(
            "WebDAV LOCK invalid request target={} status={}",
            target_path.display(),
            invalid_response.status_code
        );
        return Ok(invalid_response);
    }
    guard.insert(
        key.clone(),
        DavLock {
            token: token.clone(),
            expires_at_epoch_secs: expires_at,
            timeout_secs,
            depth_infinity,
            lockroot_href: lockroot_href.clone(),
        },
    );
    drop(guard);
    trace!(
        "WebDAV LOCK created target={} depth_infinity={} timeout_secs={}",
        target_path.display(),
        depth_infinity,
        timeout_secs
    );

    if !target_path.exists() {
        if request.path.ends_with('/') {
            std::fs::create_dir_all(&target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let _ = std::fs::File::create(&target_path)?;
        }
    }

    Ok(lock_success_response(
        &token,
        timeout_secs,
        depth_infinity,
        &lockroot_href,
        false,
    ))
}

fn validate_new_lock_request(body: &[u8]) -> Option<Response> {
    if body.is_empty() {
        debug!("WebDAV LOCK missing lockinfo body");
        return Some(status_response(400, "Bad Request"));
    }
    let Ok(body_str) = std::str::from_utf8(body) else {
        debug!("WebDAV LOCK body is not utf-8");
        return Some(status_response(400, "Bad Request"));
    };
    let xml = body_str.to_ascii_lowercase();
    let has_lockinfo = xml.contains("<d:lockinfo") || xml.contains("<lockinfo");
    if !has_lockinfo {
        debug!("WebDAV LOCK missing lockinfo element");
        return Some(status_response(400, "Bad Request"));
    }

    let has_write = xml.contains("<d:write/>") || xml.contains("<write/>");
    let has_exclusive = xml.contains("<d:exclusive/>") || xml.contains("<exclusive/>");
    let has_shared = xml.contains("<d:shared/>") || xml.contains("<shared/>");
    if !has_write || has_shared || !has_exclusive {
        debug!(
            "WebDAV LOCK unsupported scope/type has_write={} has_exclusive={} has_shared={}",
            has_write, has_exclusive, has_shared
        );
        return Some(status_response(409, "Conflict"));
    }
    None
}

fn handle_unlock(request: &Request, base_dir: &Path) -> Result<Response, AppError> {
    let _op_guard = op_guard()
        .lock()
        .map_err(|_| AppError::InternalServerError("dav operation guard poisoned".to_string()))?;
    let target_path = resolve_request_path(base_dir, &request.path)?;
    let key = lock_key(&target_path);
    cleanup_expired_locks();
    debug!("WebDAV UNLOCK target={}", target_path.display());

    let lock_token_raw = request
        .headers
        .get("lock-token")
        .ok_or(AppError::BadRequest)?;
    let normalized_token = normalize_lock_token(lock_token_raw).ok_or(AppError::BadRequest)?;

    let map = locks_map();
    let mut guard = map
        .lock()
        .map_err(|_| AppError::InternalServerError("lock map poisoned".to_string()))?;
    match guard.get(&key) {
        Some(lock) if lock.token == normalized_token => {
            guard.remove(&key);
            debug!("WebDAV UNLOCK success target={}", target_path.display());
            Ok(status_response(204, "No Content"))
        }
        Some(_) => {
            debug!(
                "WebDAV UNLOCK token mismatch target={}",
                target_path.display()
            );
            Ok(status_response(409, "Conflict"))
        }
        None => {
            debug!(
                "WebDAV UNLOCK no active lock target={}",
                target_path.display()
            );
            Ok(status_response(409, "Conflict"))
        }
    }
}

fn extract_destination_path(
    destination_header: &str,
    request_host: Option<&str>,
) -> Result<String, AppError> {
    let trimmed = destination_header.trim();

    if let Some(scheme_sep) = trimmed.find("://") {
        let rest = &trimmed[scheme_sep + 3..];
        let path_start = rest.find('/').ok_or(AppError::BadRequest)?;
        let authority = &rest[..path_start];
        if let Some(host) = request_host
            && !authority.eq_ignore_ascii_case(host.trim())
        {
            return Err(AppError::BadRequest);
        }
        return decode_percent_path(&rest[path_start..]);
    }

    Err(AppError::BadRequest)
}

fn decode_percent_path(path: &str) -> Result<String, AppError> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(AppError::BadRequest);
            }
            let hex =
                std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|_| AppError::BadRequest)?;
            let byte = u8::from_str_radix(hex, 16).map_err(|_| AppError::BadRequest)?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| AppError::BadRequest)
}

fn copy_path_recursive(source: &Path, destination: &Path) -> Result<(), AppError> {
    if source.is_dir() {
        std::fs::create_dir_all(destination)?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = destination.join(entry.file_name());
            copy_path_recursive(&src_path, &dst_path)?;
        }
        Ok(())
    } else {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, destination)?;
        Ok(())
    }
}

fn request_body_bytes(request: &Request) -> Result<Vec<u8>, AppError> {
    match &request.body {
        Some(RequestBody::Memory(data)) => Ok(data.clone()),
        Some(RequestBody::File { path, .. }) => std::fs::read(path).map_err(AppError::from),
        None => Ok(Vec::new()),
    }
}

fn dead_props_map() -> &'static Mutex<HashMap<String, HashMap<PropName, String>>> {
    DAV_DEAD_PROPERTIES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn dead_props_for_path(path: &Path) -> HashMap<PropName, String> {
    let key = lock_key(path);
    if let Ok(guard) = dead_props_map().lock() {
        return guard.get(&key).cloned().unwrap_or_default();
    }
    HashMap::new()
}

fn copy_dead_props(source: &Path, destination: &Path) {
    if let Ok(mut guard) = dead_props_map().lock() {
        if let Some(props) = guard.get(&lock_key(source)).cloned() {
            guard.insert(lock_key(destination), props);
        }
        if !source.is_dir() {
            return;
        }
        let entries: Vec<(String, HashMap<PropName, String>)> = guard
            .iter()
            .filter_map(|(key, props)| {
                let current = Path::new(key);
                current
                    .strip_prefix(source)
                    .ok()
                    .map(|rel| (lock_key(&destination.join(rel)), props.clone()))
            })
            .collect();
        for (new_key, props) in entries {
            guard.insert(new_key, props);
        }
    }
}

fn move_dead_props(source: &Path, destination: &Path) {
    if let Ok(mut guard) = dead_props_map().lock() {
        if let Some(props) = guard.remove(&lock_key(source)) {
            guard.insert(lock_key(destination), props);
        }
        if !source.is_dir() {
            return;
        }
        let keys: Vec<String> = guard.keys().cloned().collect();
        let mut moved = Vec::new();
        for key in keys {
            let current = Path::new(&key);
            if let Ok(rel) = current.strip_prefix(source)
                && let Some(props) = guard.remove(&key)
            {
                moved.push((lock_key(&destination.join(rel)), props));
            }
        }
        for (new_key, props) in moved {
            guard.insert(new_key, props);
        }
    }
}

fn remove_lock_for_exact_path(path: &Path) {
    if let Ok(mut guard) = locks_map().lock() {
        guard.remove(&lock_key(path));
    }
}

fn remove_locks_for_subtree(root: &Path) {
    if let Ok(mut guard) = locks_map().lock() {
        let keys: Vec<String> = guard
            .keys()
            .filter_map(|key| {
                let current = Path::new(key);
                if current == root || current.strip_prefix(root).is_ok() {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        for key in keys {
            guard.remove(&key);
        }
    }
}

fn move_locks_for_subtree(source: &Path, destination: &Path) {
    if let Ok(mut guard) = locks_map().lock() {
        let keys: Vec<String> = guard.keys().cloned().collect();
        let mut moved = Vec::new();
        for key in keys {
            let current = Path::new(&key);
            if let Ok(rel) = current.strip_prefix(source)
                && let Some(lock) = guard.remove(&key)
            {
                moved.push((lock_key(&destination.join(rel)), lock));
            }
        }
        for (new_key, lock) in moved {
            guard.insert(new_key, lock);
        }
    }
}

fn remove_dead_prop_for_exact_path(path: &Path) {
    if let Ok(mut guard) = dead_props_map().lock() {
        guard.remove(&lock_key(path));
    }
}

fn remove_dead_props_for_subtree(root: &Path) {
    if let Ok(mut guard) = dead_props_map().lock() {
        let keys: Vec<String> = guard
            .keys()
            .filter_map(|key| {
                let current = Path::new(key);
                if current == root || current.strip_prefix(root).is_ok() {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        for key in keys {
            guard.remove(&key);
        }
    }
}

fn handle_proppatch(request: &Request, base_dir: &Path) -> Result<Response, AppError> {
    let _op_guard = op_guard()
        .lock()
        .map_err(|_| AppError::InternalServerError("dav operation guard poisoned".to_string()))?;
    let target_path = resolve_request_path(base_dir, &request.path)?;
    if !target_path.exists() {
        return Err(AppError::NotFound);
    }
    if let Some(response) = lock_precondition_response(request, &target_path) {
        debug!(
            "WebDAV PROPPATCH blocked by lock target={} status={}",
            target_path.display(),
            response.status_code
        );
        return Ok(response);
    }

    let body = request_body_bytes(request)?;
    let body_str = std::str::from_utf8(&body).map_err(|_| AppError::BadRequest)?;

    let operations = parse_propertyupdate_operations(body_str);
    if operations.is_empty() {
        return Err(AppError::BadRequest);
    }

    let key = lock_key(&target_path);
    let mut ok_or_dependent_props: Vec<(PropName, Option<String>)> = Vec::new();
    let mut missing_props: Vec<(PropName, Option<String>)> = Vec::new();
    let mut forbidden_props: Vec<(PropName, Option<String>)> = Vec::new();
    let mut has_failure = false;

    let mut guard = dead_props_map()
        .lock()
        .map_err(|_| AppError::InternalServerError("dead properties map poisoned".to_string()))?;
    let current_props = guard.get(&key).cloned().unwrap_or_default();
    let mut working_props = current_props.clone();

    for operation in operations {
        for (name, value) in operation.props {
            if is_protected_property(&name) {
                forbidden_props.push((name, None));
                has_failure = true;
                continue;
            }
            match operation.action {
                PropPatchAction::Set => {
                    working_props.insert(name.clone(), value.unwrap_or_default());
                    ok_or_dependent_props.push((name, None));
                }
                PropPatchAction::Remove => {
                    if working_props.remove(&name).is_some() {
                        ok_or_dependent_props.push((name, None));
                    } else {
                        missing_props.push((name, None));
                        has_failure = true;
                    }
                }
            }
        }
    }
    if !has_failure {
        guard.insert(key, working_props);
    }
    drop(guard);

    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
  <D:response>
"#,
    );
    let href = build_href(base_dir, &target_path, target_path.is_dir());
    xml.push_str("    <D:href>");
    xml.push_str(&xml_escape(&href));
    xml.push_str("</D:href>\n");
    if !ok_or_dependent_props.is_empty() {
        if has_failure {
            append_propstat(
                &mut xml,
                &ok_or_dependent_props,
                "HTTP/1.1 424 Failed Dependency",
            );
        } else {
            append_propstat(&mut xml, &ok_or_dependent_props, "HTTP/1.1 200 OK");
        }
    }
    if !missing_props.is_empty() {
        append_propstat(&mut xml, &missing_props, "HTTP/1.1 404 Not Found");
    }
    if !forbidden_props.is_empty() {
        append_propstat(&mut xml, &forbidden_props, "HTTP/1.1 403 Forbidden");
    }
    xml.push_str("  </D:response>\n</D:multistatus>\n");

    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/xml; charset=utf-8".to_string(),
    );
    Ok(Response {
        status_code: 207,
        status_text: "Multi-Status".to_string(),
        headers,
        body: ResponseBody::Text(xml),
    })
}

fn parse_propertyupdate_operations(xml: &str) -> Vec<PropPatchOperation> {
    let lowered = xml.to_ascii_lowercase();
    let global_namespace_map = parse_xmlns_mappings(xml);
    let mut operations = Vec::new();
    let mut idx = 0usize;

    while idx < lowered.len() {
        let next_set = lowered[idx..]
            .find("<d:set")
            .or_else(|| lowered[idx..].find("<set"))
            .map(|v| idx + v);
        let next_remove = lowered[idx..]
            .find("<d:remove")
            .or_else(|| lowered[idx..].find("<remove"))
            .map(|v| idx + v);

        let (action, op_start) = match (next_set, next_remove) {
            (Some(s), Some(r)) if s < r => (PropPatchAction::Set, s),
            (Some(_), Some(r)) => (PropPatchAction::Remove, r),
            (Some(s), None) => (PropPatchAction::Set, s),
            (None, Some(r)) => (PropPatchAction::Remove, r),
            (None, None) => break,
        };

        let (close_tag, close_tag_plain) = match action {
            PropPatchAction::Set => ("</d:set>", "</set>"),
            PropPatchAction::Remove => ("</d:remove>", "</remove>"),
        };

        let Some(open_end_rel) = lowered[op_start..].find('>') else {
            break;
        };
        let open_end = op_start + open_end_rel + 1;
        let Some(op_end) = lowered[open_end..]
            .find(close_tag)
            .or_else(|| lowered[open_end..].find(close_tag_plain))
            .map(|v| open_end + v)
        else {
            break;
        };

        let props = parse_prop_elements(&xml[open_end..op_end], &global_namespace_map);
        if !props.is_empty() {
            operations.push(PropPatchOperation { action, props });
        }
        idx = if lowered[op_end..].starts_with(close_tag) {
            op_end + close_tag.len()
        } else {
            op_end + close_tag_plain.len()
        };
    }

    operations
}

fn is_protected_property(name: &PropName) -> bool {
    if name.namespace != DAV_NAMESPACE {
        return false;
    }
    matches!(
        name.local_name.as_str(),
        "displayname"
            | "resourcetype"
            | "getcontentlength"
            | "getlastmodified"
            | "getcontenttype"
            | "creationdate"
            | "getetag"
            | "supportedlock"
            | "lockdiscovery"
    )
}

fn parse_prop_elements(
    fragment: &str,
    base_namespace_map: &HashMap<String, String>,
) -> Vec<(PropName, Option<String>)> {
    let mut items = Vec::new();
    let mut namespace_map = base_namespace_map.clone();
    for (prefix, uri) in parse_xmlns_mappings(fragment) {
        namespace_map.insert(prefix, uri);
    }
    let mut idx = 0usize;
    while let Some(lt_rel) = fragment[idx..].find('<') {
        let lt = idx + lt_rel;
        let Some(gt_rel) = fragment[lt..].find('>') else {
            break;
        };
        let gt = lt + gt_rel;
        let token = fragment[lt + 1..gt].trim();
        idx = gt + 1;

        if token.is_empty()
            || token.starts_with('/')
            || token.starts_with('?')
            || token.starts_with('!')
            || token.to_ascii_lowercase().contains(":prop")
        {
            continue;
        }

        for (prefix, uri) in parse_xmlns_mappings_from_tag(token) {
            namespace_map.insert(prefix, uri);
        }
        let Some(name) = parse_prop_name_from_token(token, &namespace_map) else {
            continue;
        };

        if token.ends_with('/') {
            items.push((name, None));
            continue;
        }
        let raw = token
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('/');

        let close_tag = format!("</{raw}>");
        if let Some(close_rel) = fragment[idx..]
            .to_ascii_lowercase()
            .find(&close_tag.to_ascii_lowercase())
        {
            let value = fragment[idx..idx + close_rel].trim().to_string();
            idx += close_rel + close_tag.len();
            items.push((name, Some(value)));
        } else {
            items.push((name, None));
        }
    }
    items
}

fn parse_depth_header(headers: &HashMap<String, String>) -> Result<DavDepth, AppError> {
    let depth = headers
        .get("depth")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "infinity".to_string());
    match depth.as_str() {
        "0" => Ok(DavDepth::Zero),
        "1" => Ok(DavDepth::One),
        "infinity" => Ok(DavDepth::Infinity),
        _ => Err(AppError::BadRequest),
    }
}

fn locked_descendants_without_token(request: &Request, target_path: &Path) -> Vec<PathBuf> {
    cleanup_expired_locks();
    let mut locked = Vec::new();
    let Ok(guard) = locks_map().lock() else {
        return locked;
    };
    for (path_key, lock) in guard.iter() {
        let locked_path = Path::new(path_key);
        if locked_path != target_path
            && is_descendant_or_same(locked_path, target_path)
            && !token_present_for_request(request, &lock.token, &request.path)
        {
            locked.push(PathBuf::from(path_key));
        }
    }
    locked.sort();
    locked.dedup();
    locked
}

fn delete_locked_multistatus_response(
    base_dir: &Path,
    target_path: &Path,
    locked_descendants: &[PathBuf],
) -> Response {
    let mut body = String::from(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:">
"#,
    );

    for locked in locked_descendants {
        body.push_str("  <D:response>\n");
        body.push_str("    <D:href>");
        body.push_str(&xml_escape(&build_href(base_dir, locked, locked.is_dir())));
        body.push_str("</D:href>\n");
        body.push_str("    <D:status>HTTP/1.1 423 Locked</D:status>\n");
        body.push_str("  </D:response>\n");
    }

    body.push_str("  <D:response>\n");
    body.push_str("    <D:href>");
    body.push_str(&xml_escape(&build_href(
        base_dir,
        target_path,
        target_path.is_dir(),
    )));
    body.push_str("</D:href>\n");
    body.push_str("    <D:status>HTTP/1.1 424 Failed Dependency</D:status>\n");
    body.push_str("  </D:response>\n");
    body.push_str("</D:multistatus>\n");

    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/xml; charset=utf-8".to_string(),
    );
    Response {
        status_code: 207,
        status_text: "Multi-Status".to_string(),
        headers,
        body: ResponseBody::Text(body),
    }
}

fn lock_precondition_response(request: &Request, target_path: &Path) -> Option<Response> {
    cleanup_expired_locks();
    let map = locks_map();
    let Ok(guard) = map.lock() else {
        return Some(status_response(500, "Internal Server Error"));
    };
    for (key, lock) in guard.iter() {
        let lock_path = Path::new(key);
        let applies = lock_path == target_path
            || (lock.depth_infinity && is_same_or_ancestor(lock_path, target_path));
        if applies && !token_present_for_request(request, &lock.token, &request.path) {
            trace!(
                "WebDAV precondition failed target={} lock_path={} depth_infinity={}",
                target_path.display(),
                lock_path.display(),
                lock.depth_infinity
            );
            return Some(status_response(423, "Locked"));
        }
    }
    None
}

fn token_present_for_request(request: &Request, expected_token: &str, request_path: &str) -> bool {
    if request
        .headers
        .get("lock-token")
        .and_then(|v| normalize_lock_token(v))
        .map(|t| t == expected_token)
        .unwrap_or(false)
    {
        return true;
    }

    let if_header = request.headers.get("if").map(String::as_str).unwrap_or("");
    if_header_matches_lock_token(if_header, expected_token, request_path)
}

fn if_header_matches_lock_token(if_header: &str, expected_token: &str, request_path: &str) -> bool {
    if if_header.trim().is_empty() {
        return false;
    }

    let request_path = request_path_only(request_path);
    let mut idx = 0usize;
    while let Some(open_rel) = if_header[idx..].find('(') {
        let open = idx + open_rel;
        let Some(close_rel) = if_header[open + 1..].find(')') else {
            break;
        };
        let close = open + 1 + close_rel;
        let list = &if_header[open + 1..close];
        let tag_matches = if_header_resource_tag_matches(&if_header[idx..open], &request_path);
        idx = close + 1;
        if !tag_matches {
            continue;
        }

        let tokens: Vec<&str> = list.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let mut list_ok = true;
        let mut has_positive_expected = false;
        let mut i = 0usize;
        while i < tokens.len() {
            let mut not = false;
            if tokens[i].eq_ignore_ascii_case("Not") {
                not = true;
                i += 1;
                if i >= tokens.len() {
                    list_ok = false;
                    break;
                }
            }

            let term = tokens[i];
            i += 1;
            if term.starts_with('<') && term.ends_with('>') {
                let token = term.trim_matches(|c| c == '<' || c == '>');
                if token == expected_token {
                    if not {
                        list_ok = false;
                        break;
                    }
                    has_positive_expected = true;
                } else if !not {
                    list_ok = false;
                    break;
                }
            } else {
                // Ignore ETag/state token terms for lock-token authorization.
                continue;
            }
        }

        if list_ok && has_positive_expected {
            return true;
        }
    }
    false
}

fn request_path_only(path: &str) -> String {
    let only_path = path.split('?').next().unwrap_or(path);
    format!("/{}", only_path.trim_start_matches('/'))
}

fn if_header_resource_tag_matches(prefix: &str, request_path: &str) -> bool {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return true;
    }
    let Some(start) = trimmed.rfind('<') else {
        return true;
    };
    let Some(end) = trimmed.rfind('>') else {
        return true;
    };
    if end <= start + 1 {
        return true;
    }
    let tag = &trimmed[start + 1..end];
    if let Some(path_start) = tag
        .find("://")
        .and_then(|sep| tag[sep + 3..].find('/').map(|v| sep + 3 + v))
    {
        let tagged_path = &tag[path_start..];
        return tagged_path == request_path;
    }
    if tag.starts_with('/') {
        return tag == request_path;
    }
    true
}

fn is_descendant_or_same(path: &Path, ancestor: &Path) -> bool {
    path == ancestor || path.strip_prefix(ancestor).is_ok()
}

fn is_same_or_ancestor(ancestor: &Path, path: &Path) -> bool {
    path == ancestor || path.strip_prefix(ancestor).is_ok()
}

fn lock_success_response(
    token: &str,
    timeout_secs: u64,
    depth_infinity: bool,
    lockroot_href: &str,
    is_refresh: bool,
) -> Response {
    let timeout_header = format!("Second-{timeout_secs}");
    let lock_body = lockdiscovery_xml(token, &timeout_header, depth_infinity, lockroot_href);
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Type".to_string(),
        "application/xml; charset=utf-8".to_string(),
    );
    headers.insert("Lock-Token".to_string(), format!("<{token}>"));
    headers.insert("Timeout".to_string(), timeout_header);
    Response {
        status_code: if is_refresh { 200 } else { 201 },
        status_text: if is_refresh { "OK" } else { "Created" }.to_string(),
        headers,
        body: ResponseBody::Text(lock_body),
    }
}

fn locks_map() -> &'static Mutex<HashMap<String, DavLock>> {
    DAV_LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn op_guard() -> &'static Mutex<()> {
    DAV_OP_GUARD.get_or_init(|| Mutex::new(()))
}

fn current_lock_for_path(path: &Path) -> Option<DavLock> {
    cleanup_expired_locks();
    let guard = locks_map().lock().ok()?;
    guard.get(&lock_key(path)).cloned()
}

fn lock_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn cleanup_expired_locks() {
    let now = now_epoch_secs();
    if let Ok(mut guard) = locks_map().lock() {
        guard.retain(|_, lock| lock.expires_at_epoch_secs > now);
    }
}

fn parse_lock_timeout_secs(headers: &HashMap<String, String>) -> Option<u64> {
    let timeout = headers.get("timeout")?;
    for token in timeout.split(',') {
        let token = token.trim();
        if token.eq_ignore_ascii_case("infinite") {
            return Some(3600);
        }
        if let Some(seconds) = token.strip_prefix("Second-")
            && let Ok(v) = seconds.parse::<u64>()
        {
            return Some(v.clamp(1, 3600));
        }
    }
    None
}

fn parse_lock_depth(
    headers: &HashMap<String, String>,
    is_collection: bool,
) -> Result<bool, AppError> {
    let depth = headers
        .get("depth")
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| {
            if is_collection {
                "infinity".to_string()
            } else {
                "0".to_string()
            }
        });
    match depth.as_str() {
        "0" => Ok(false),
        "infinity" if is_collection => Ok(true),
        _ => Err(AppError::BadRequest),
    }
}

fn validate_delete_depth_header(
    headers: &HashMap<String, String>,
    is_collection: bool,
) -> Result<(), AppError> {
    if !is_collection {
        return Ok(());
    }
    match headers.get("depth") {
        None => Ok(()),
        Some(value) if value.trim().eq_ignore_ascii_case("infinity") => Ok(()),
        Some(_) => Err(AppError::BadRequest),
    }
}

fn validate_move_depth_header(
    headers: &HashMap<String, String>,
    is_collection: bool,
) -> Result<(), AppError> {
    if !is_collection {
        return Ok(());
    }
    match headers.get("depth") {
        None => Ok(()),
        Some(value) if value.trim().eq_ignore_ascii_case("infinity") => Ok(()),
        Some(_) => Err(AppError::BadRequest),
    }
}

fn normalize_lock_token(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let without_brackets = trimmed
        .strip_prefix('<')
        .and_then(|v| v.strip_suffix('>'))
        .unwrap_or(trimmed);
    Some(without_brackets.to_string())
}

fn lockdiscovery_xml(
    token: &str,
    timeout: &str,
    depth_infinity: bool,
    lockroot_href: &str,
) -> String {
    let depth_value = if depth_infinity { "Infinity" } else { "0" };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<D:prop xmlns:D="DAV:">
  <D:lockdiscovery>
    <D:activelock>
      <D:locktype><D:write/></D:locktype>
      <D:lockscope><D:exclusive/></D:lockscope>
      <D:depth>{depth_value}</D:depth>
      <D:timeout>{timeout}</D:timeout>
      <D:lockroot><D:href>{lockroot_href}</D:href></D:lockroot>
      <D:locktoken><D:href>{token}</D:href></D:locktoken>
    </D:activelock>
  </D:lockdiscovery>
</D:prop>"#
    )
}

fn status_response(status_code: u16, status_text: &str) -> Response {
    if status_code == 423 {
        let mut headers = HashMap::new();
        headers.insert(
            "Content-Type".to_string(),
            "application/xml; charset=utf-8".to_string(),
        );
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:error xmlns:D="DAV:">
  <D:lock-token-submitted/>
</D:error>"#
            .to_string();
        return Response {
            status_code,
            status_text: status_text.to_string(),
            headers,
            body: ResponseBody::Text(body),
        };
    }

    let mut headers = HashMap::new();
    headers.insert("Content-Length".to_string(), "0".to_string());
    Response {
        status_code,
        status_text: status_text.to_string(),
        headers,
        body: ResponseBody::Text(String::new()),
    }
}

fn is_finder_archive_temp_path(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    name.starts_with(".AU.") || name.starts_with(".ArchiveServiceTemp")
}

fn is_finder_archive_related_move(source: &Path, destination_request_path: &str) -> bool {
    if is_finder_archive_temp_path(source) {
        return true;
    }
    if destination_request_path.contains(".sb-") {
        return true;
    }
    if destination_request_path.contains("/.AU.")
        || destination_request_path.contains("/.ArchiveServiceTemp")
    {
        return true;
    }
    destination_request_path.contains("AUHelperService")
        || destination_request_path.contains("A%20Document%20Being%20Saved%20By%20AUHelperService")
}

fn resolve_request_path_without_canonicalize(
    base_dir: &Path,
    request_path: &str,
) -> Result<PathBuf, AppError> {
    let path_only = request_path.split('?').next().unwrap_or(request_path);
    let requested_path = PathBuf::from(path_only.strip_prefix('/').unwrap_or(path_only));
    let safe_path = normalize_relative_path(&requested_path)?;
    let full_path = base_dir.join(safe_path);
    if !full_path.starts_with(base_dir) {
        return Err(AppError::Forbidden);
    }
    Ok(full_path)
}

fn resolve_request_path(base_dir: &Path, request_path: &str) -> Result<PathBuf, AppError> {
    let path_only = request_path.split('?').next().unwrap_or(request_path);
    let requested_path = PathBuf::from(path_only.strip_prefix('/').unwrap_or(path_only));
    let safe_path = normalize_relative_path(&requested_path)?;
    let full_path = base_dir.join(safe_path);
    if !full_path.starts_with(base_dir) {
        return Err(AppError::Forbidden);
    }
    let base_canonical = std::fs::canonicalize(base_dir)?;
    if full_path.exists() {
        let resolved = std::fs::canonicalize(&full_path)?;
        if !resolved.starts_with(&base_canonical) {
            return Err(AppError::Forbidden);
        }
    } else {
        let mut current = full_path.parent();
        while let Some(parent) = current {
            if parent.exists() {
                let resolved_parent = std::fs::canonicalize(parent)?;
                if !resolved_parent.starts_with(&base_canonical) {
                    return Err(AppError::Forbidden);
                }
                break;
            }
            current = parent.parent();
        }
    }
    Ok(full_path)
}

fn normalize_relative_path(path: &Path) -> Result<PathBuf, AppError> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => components.push(name),
            Component::ParentDir => {
                if components.pop().is_none() {
                    return Err(AppError::Forbidden);
                }
            }
            _ => {}
        }
    }
    Ok(components.iter().collect())
}

fn append_multistatus_response(
    xml: &mut String,
    base_dir: &Path,
    resource: &Path,
    mode: &PropfindMode,
) -> Result<(), AppError> {
    let metadata = std::fs::metadata(resource)?;
    let is_dir = metadata.is_dir();
    let href = build_href(base_dir, resource, is_dir);
    let displayname = if resource == base_dir {
        "/".to_string()
    } else {
        resource
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    };

    xml.push_str("  <D:response>\n");
    xml.push_str("    <D:href>");
    xml.push_str(&xml_escape(&href));
    xml.push_str("</D:href>\n");
    let live_props = build_live_props(resource, is_dir, &displayname, &metadata);
    match mode {
        PropfindMode::AllProp => {
            let mut merged = live_props.clone();
            let dead = dead_props_for_path(resource);
            for (name, value) in dead {
                if !merged.iter().any(|(live_name, _)| live_name == &name) {
                    merged.push((name, Some(xml_escape(&value))));
                }
            }
            append_propstat(xml, &merged, "HTTP/1.1 200 OK");
        }
        PropfindMode::PropName => {
            let mut names: Vec<(PropName, Option<String>)> = live_props
                .iter()
                .map(|(name, _)| (name.clone(), None))
                .collect();
            let dead = dead_props_for_path(resource);
            for dead_name in dead.keys() {
                if !names.iter().any(|(name, _)| name == dead_name) {
                    names.push((dead_name.clone(), None));
                }
            }
            append_propstat(xml, &names, "HTTP/1.1 200 OK");
        }
        PropfindMode::Named(requested) => {
            let mut ok_props: Vec<(PropName, Option<String>)> = Vec::new();
            let mut not_found_props: Vec<(PropName, Option<String>)> = Vec::new();
            let dead = dead_props_for_path(resource);

            for prop in requested {
                if let Some((_, value)) = live_props.iter().find(|(name, _)| name == prop) {
                    ok_props.push((prop.clone(), value.clone()));
                } else if let Some(value) = dead.get(prop) {
                    ok_props.push((prop.clone(), Some(xml_escape(value))));
                } else {
                    not_found_props.push((prop.clone(), None));
                }
            }

            if !ok_props.is_empty() {
                append_propstat(xml, &ok_props, "HTTP/1.1 200 OK");
            }
            if !not_found_props.is_empty() {
                append_propstat(xml, &not_found_props, "HTTP/1.1 404 Not Found");
            }
        }
    }
    xml.push_str("  </D:response>\n");

    Ok(())
}

fn append_propstat(xml: &mut String, props: &[(PropName, Option<String>)], status: &str) {
    xml.push_str("    <D:propstat>\n");
    xml.push_str("      <D:prop>\n");
    for (name, value) in props {
        let (prefix, xmlns_attr) = prop_render_prefix_and_xmlns(name);
        xml.push_str("        <");
        xml.push_str(&prefix);
        xml.push(':');
        xml.push_str(&name.local_name);
        if !xmlns_attr.is_empty() {
            xml.push(' ');
            xml.push_str(&xmlns_attr);
        }
        if let Some(v) = value {
            xml.push('>');
            xml.push_str(v);
            xml.push_str("</");
            xml.push_str(&prefix);
            xml.push(':');
            xml.push_str(&name.local_name);
            xml.push_str(">\n");
        } else {
            xml.push_str("/>\n");
        }
    }
    xml.push_str("      </D:prop>\n");
    xml.push_str("      <D:status>");
    xml.push_str(status);
    xml.push_str("</D:status>\n");
    xml.push_str("    </D:propstat>\n");
}

fn dav_prop_name(local_name: &str) -> PropName {
    PropName {
        namespace: DAV_NAMESPACE.to_string(),
        local_name: local_name.to_ascii_lowercase(),
    }
}

fn prop_render_prefix_and_xmlns(name: &PropName) -> (String, String) {
    if name.namespace == DAV_NAMESPACE {
        return ("D".to_string(), String::new());
    }
    (
        "X".to_string(),
        format!(r#"xmlns:X="{}""#, xml_escape(&name.namespace)),
    )
}

fn build_live_props(
    resource: &Path,
    is_dir: bool,
    displayname: &str,
    metadata: &std::fs::Metadata,
) -> Vec<(PropName, Option<String>)> {
    let mut props = Vec::new();
    props.push((dav_prop_name("displayname"), Some(xml_escape(displayname))));

    if is_dir {
        props.push((
            dav_prop_name("resourcetype"),
            Some("<D:collection/>".to_string()),
        ));
    } else {
        props.push((dav_prop_name("resourcetype"), Some(String::new())));
        props.push((
            dav_prop_name("getcontentlength"),
            Some(metadata.len().to_string()),
        ));
        props.push((
            dav_prop_name("getcontenttype"),
            Some("application/octet-stream".to_string()),
        ));
    }

    if let Ok(modified) = metadata.modified()
        && let Some(http_date) = format_http_date(modified)
    {
        props.push((
            dav_prop_name("getlastmodified"),
            Some(xml_escape(&http_date)),
        ));
    }
    if let Ok(modified) = metadata.modified()
        && let Some(creation_date) = format_iso8601_utc(modified)
    {
        props.push((
            dav_prop_name("creationdate"),
            Some(xml_escape(&creation_date)),
        ));
    }
    props.push((
        dav_prop_name("getetag"),
        Some(etag_for_resource(resource, metadata)),
    ));
    props.push((
        dav_prop_name("supportedlock"),
        Some("<D:lockentry><D:lockscope><D:exclusive/></D:lockscope><D:locktype><D:write/></D:locktype></D:lockentry>".to_string()),
    ));
    if let Some(lock) = current_lock_for_path(resource) {
        props.push((
            dav_prop_name("lockdiscovery"),
            Some(format!(
                "<D:activelock><D:locktype><D:write/></D:locktype><D:lockscope><D:exclusive/></D:lockscope><D:depth>{}</D:depth><D:timeout>Second-{}</D:timeout><D:lockroot><D:href>{}</D:href></D:lockroot><D:locktoken><D:href>{}</D:href></D:locktoken></D:activelock>",
                if lock.depth_infinity { "Infinity" } else { "0" },
                lock.timeout_secs,
                xml_escape(&lock.lockroot_href),
                xml_escape(&lock.token)
            )),
        ));
    } else {
        props.push((dav_prop_name("lockdiscovery"), Some(String::new())));
    }

    props
}

fn etag_for_resource(path: &Path, metadata: &std::fs::Metadata) -> String {
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let seed = path.to_string_lossy();
    format!("\"{:x}-{:x}-{:x}\"", metadata.len(), modified, seed.len())
}

fn build_href(base_dir: &Path, resource: &Path, is_dir: bool) -> String {
    if resource == base_dir {
        return "/".to_string();
    }

    let relative = resource.strip_prefix(base_dir).unwrap_or(resource);
    let mut href = String::from("/");
    let mut first = true;
    for component in relative.components() {
        if let Component::Normal(segment) = component {
            if !first {
                href.push('/');
            }
            href.push_str(&percent_encode(segment.to_string_lossy().as_ref()));
            first = false;
        }
    }
    if is_dir && !href.ends_with('/') {
        href.push('/');
    }
    href
}

fn percent_encode(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        let is_unreserved = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~');
        if is_unreserved {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn format_http_date(time: SystemTime) -> Option<String> {
    const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    let total_seconds = time.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let days = total_seconds.div_euclid(86_400);
    let secs_of_day = total_seconds.rem_euclid(86_400);

    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day % 3600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;

    let weekday_idx = (days.rem_euclid(7)) as usize;
    let weekday = WEEKDAYS[weekday_idx];

    let (year, month, day) = civil_from_days(days);
    let month_name = MONTHS[(month - 1) as usize];

    Some(format!(
        "{weekday}, {day:02} {month_name} {year:04} {hour:02}:{minute:02}:{second:02} GMT"
    ))
}

fn format_iso8601_utc(time: SystemTime) -> Option<String> {
    let total_seconds = time.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    let days = total_seconds.div_euclid(86_400);
    let secs_of_day = total_seconds.rem_euclid(86_400);

    let hour = (secs_of_day / 3600) as u32;
    let minute = ((secs_of_day % 3600) / 60) as u32;
    let second = (secs_of_day % 60) as u32;
    let (year, month, day) = civil_from_days(days);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

fn civil_from_days(days_since_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = (yoe as i32) + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month as u32, day as u32)
}
