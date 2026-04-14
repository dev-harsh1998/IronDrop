// SPDX-License-Identifier: MIT

//! Handles HTTP request parsing, routing, and response generation.

use crate::error::AppError;
use crate::response::create_error_response;
use crate::router::Router;
use log::{debug, error, info, trace};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Maximum size for request body (10GB) to prevent memory exhaustion attacks
const MAX_REQUEST_BODY_SIZE: usize = 10 * 1024 * 1024 * 1024;

/// Maximum size for request headers (8KB) to prevent header buffer overflow
const MAX_HEADERS_SIZE: usize = 8 * 1024;

/// Threshold for streaming request bodies to disk (64MB)
/// This ensures total memory usage stays well below 128MB
pub const STREAM_TO_DISK_THRESHOLD: usize = 64 * 1024 * 1024;
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Represents a parsed incoming HTTP request.
#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<RequestBody>,
}

/// Request body can be either in memory or streamed to disk for large uploads
#[derive(Debug)]
pub enum RequestBody {
    /// Small bodies stored in memory
    Memory(Vec<u8>),
    /// Large bodies streamed to temporary file
    File { path: PathBuf, size: u64 },
}

impl RequestBody {
    /// Get the size of the request body in bytes
    pub fn len(&self) -> usize {
        let size = match self {
            RequestBody::Memory(data) => data.len(),
            RequestBody::File { size, .. } => *size as usize,
        };
        trace!("RequestBody size: {} bytes", size);
        size
    }

    /// Check if the request body is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Represents an outgoing HTTP response.
pub struct Response {
    pub status_code: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: ResponseBody,
}

pub struct StreamBody {
    pub path: PathBuf,
    pub size: u64,
    pub chunk_size: usize,
}

pub enum ResponseBody {
    Text(String),
    StaticText(&'static str),
    Binary(Vec<u8>),
    StaticBinary(&'static [u8]),
    Stream(StreamBody),
}

impl Request {
    /// Validates if the given method is a valid HTTP method
    fn is_valid_http_method(method: &str) -> bool {
        matches!(
            method,
            "GET"
                | "POST"
                | "PUT"
                | "DELETE"
                | "HEAD"
                | "OPTIONS"
                | "PATCH"
                | "TRACE"
                | "CONNECT"
                | "PROPFIND"
                | "MKCOL"
                | "COPY"
                | "MOVE"
                | "PROPPATCH"
                | "LOCK"
                | "UNLOCK"
        )
    }

    pub async fn from_async_stream<S>(stream: &mut S) -> Result<Self, AppError>
    where
        S: tokio::io::AsyncRead + Unpin,
    {
        let (headers_data, remaining_bytes) = read_headers_with_remaining_async(stream).await?;
        let mut lines = headers_data.lines();

        let request_line = lines.next().ok_or(AppError::BadRequest)?;
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(AppError::BadRequest);
        }

        let method = parts[0].to_string();
        let raw_path = parts[1];
        let version = parts[2];

        if !Self::is_valid_http_method(&method) {
            return Err(AppError::BadRequest);
        }
        if raw_path.contains('\0') || raw_path.is_empty() {
            return Err(AppError::BadRequest);
        }

        let path = Self::decode_url(raw_path)?;
        if !version.starts_with("HTTP/1.") {
            return Err(AppError::BadRequest);
        }

        let mut headers = HashMap::new();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim().to_string();
                if let Some(existing) = headers.get(&key) {
                    headers.insert(key, format!("{existing}, {value}"));
                } else {
                    headers.insert(key, value);
                }
            }
        }

        let body = read_request_body_async(stream, &headers, remaining_bytes).await?;

        Ok(Request {
            method,
            path,
            headers,
            body,
        })
    }

    fn has_chunked_transfer_encoding(headers: &HashMap<String, String>) -> bool {
        headers
            .get("transfer-encoding")
            .map(|encoding| {
                encoding
                    .split(',')
                    .map(|token| token.trim())
                    .any(|token| token.eq_ignore_ascii_case("chunked"))
            })
            .unwrap_or(false)
    }

    /// Simple URL decoding for percent-encoded paths
    fn decode_url(path: &str) -> Result<String, AppError> {
        let mut decoded = String::with_capacity(path.len());
        let mut chars = path.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '%' {
                // Try to decode percent-encoded character
                let hex1 = chars.next().ok_or(AppError::BadRequest)?;
                let hex2 = chars.next().ok_or(AppError::BadRequest)?;

                if let Ok(byte_val) = u8::from_str_radix(&format!("{hex1}{hex2}"), 16) {
                    if let Some(decoded_char) = char::from_u32(byte_val as u32) {
                        decoded.push(decoded_char);
                    } else {
                        // Invalid character, keep as-is
                        decoded.push(ch);
                        decoded.push(hex1);
                        decoded.push(hex2);
                    }
                } else {
                    // Invalid hex, keep as-is
                    decoded.push(ch);
                    decoded.push(hex1);
                    decoded.push(hex2);
                }
            } else {
                decoded.push(ch);
            }
        }

        Ok(decoded)
    }
}

// Static asset, favicon, upload, and health handlers moved to handlers.rs

/// Determines the correct response based on the request.
#[allow(clippy::too_many_arguments)]
fn route_request(
    request: &Request,
    base_dir: &Arc<PathBuf>,
    allowed_extensions: &Arc<Vec<glob::Pattern>>,
    _username: &Arc<Option<String>>,
    _password: &Arc<Option<String>>,
    chunk_size: usize,
    cli_config: Option<&crate::cli::Cli>,
    _stats: Option<&crate::server::ServerStats>,
    router: &Arc<Router>,
) -> Result<Response, AppError> {
    trace!("Routing {} {} through router", request.method, request.path);
    // Authentication is now handled by middleware in the router
    // First check if router handles the request (internal routes)
    if let Some(router_response) = router.route(request) {
        debug!(
            "Route found in router for {} {}",
            request.method, request.path
        );
        trace!("Router handler execution starting");
        return router_response;
    }

    // All non-internal paths (not starting with /_irondrop/) are treated as file / directory lookup
    if request.path.starts_with("/_irondrop/") {
        debug!("Internal path {} not found in router", request.path);
        return Err(AppError::NotFound);
    }

    debug!("Handling file request for path: {}", request.path);
    trace!("Using file handler for non-internal path");
    // Handle file and directory serving via dedicated handler
    crate::handlers::handle_file_request(
        request,
        base_dir,
        allowed_extensions,
        chunk_size,
        cli_config,
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn handle_client_async<S>(
    mut stream: S,
    peer_addr: std::net::SocketAddr,
    base_dir: Arc<PathBuf>,
    allowed_extensions: Arc<Vec<glob::Pattern>>,
    username: Arc<Option<String>>,
    password: Arc<Option<String>>,
    chunk_size: usize,
    cli_config: Option<Arc<crate::cli::Cli>>,
    stats: Option<Arc<crate::server::ServerStats>>,
    router: Arc<Router>,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send,
{
    let log_prefix = format!("[{}]", peer_addr);

    let request = match Request::from_async_stream(&mut stream).await {
        Ok(req) => req,
        Err(e) => {
            send_error_response_async(&mut stream, e, &log_prefix).await;
            if let Some(stats) = stats {
                stats.record_request(false, 0);
            }
            return;
        }
    };

    let cleanup_path = match &request.body {
        Some(RequestBody::File { path, .. }) => Some(path.clone()),
        _ => None,
    };

    let request_method = request.method.clone();
    let request_path = request.path.clone();

    let response_result = tokio::task::spawn_blocking({
        let base_dir = base_dir.clone();
        let allowed_extensions = allowed_extensions.clone();
        let username = username.clone();
        let password = password.clone();
        let cli_config = cli_config.clone();
        let router = router.clone();
        move || {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                route_request(
                    &request,
                    &base_dir,
                    &allowed_extensions,
                    &username,
                    &password,
                    chunk_size,
                    cli_config.as_deref(),
                    None,
                    &router,
                )
            }))
            .unwrap_or_else(|_| {
                Err(AppError::InternalServerError(
                    "Client handler panicked".into(),
                ))
            })
        }
    })
    .await;

    let response_result = match response_result {
        Ok(result) => result,
        Err(_) => Err(AppError::InternalServerError("Join error".into())),
    };

    match response_result {
        Ok(response) => {
            info!(
                "{} {} {} -> {}",
                log_prefix, request_method, request_path, response.status_code
            );
            match send_response_async(&mut stream, response, &log_prefix).await {
                Ok(body_bytes) => {
                    if let Some(stats) = stats {
                        stats.record_request(true, body_bytes);
                    }
                }
                Err(_) => {
                    if let Some(stats) = stats {
                        stats.record_request(false, 0);
                    }
                }
            }
        }
        Err(e) => {
            send_error_response_async(&mut stream, e, &log_prefix).await;
            if let Some(stats) = stats {
                stats.record_request(false, 0);
            }
        }
    }

    if let Some(path) = cleanup_path {
        let _ = tokio::fs::remove_file(path).await;
    }
}

async fn send_error_response_async<S>(stream: &mut S, error: AppError, log_prefix: &str)
where
    S: tokio::io::AsyncWrite + Unpin,
{
    let (status_code, status_text) = match error {
        AppError::NotFound => (404, "Not Found"),
        AppError::Forbidden => (403, "Forbidden"),
        AppError::BadRequest => (400, "Bad Request"),
        AppError::Unauthorized => (401, "Unauthorized"),
        AppError::MethodNotAllowed => (405, "Method Not Allowed"),
        AppError::PayloadTooLarge(_) => (413, "Payload Too Large"),
        AppError::InvalidFilename(_) => (400, "Bad Request"),
        AppError::UploadDiskFull(_) => (507, "Insufficient Storage"),
        AppError::UnsupportedMediaType(_) => (415, "Unsupported Media Type"),
        AppError::UploadDisabled => (403, "Forbidden"),
        _ => (500, "Internal Server Error"),
    };

    info!("{log_prefix} {status_code} {status_text}");

    let http_response = create_error_response(status_code, status_text);
    let mut headers = HashMap::new();
    for (k, v) in http_response.headers {
        headers.insert(k, v);
    }

    let response = Response {
        status_code: http_response.status_code,
        status_text: http_response.status_text,
        headers,
        body: ResponseBody::Binary(http_response.body),
    };

    let _ = send_response_async(stream, response, log_prefix).await;
}

async fn send_response_async<S>(
    stream: &mut S,
    response: Response,
    log_prefix: &str,
) -> Result<u64, std::io::Error>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    let mut response_str = format!(
        "HTTP/1.1 {} {}
",
        response.status_code, response.status_text
    );

    response_str.push_str(&format!(
        "Server: irondrop/{}
",
        crate::VERSION
    ));
    response_str.push_str(
        "Connection: close
",
    );

    for (key, value) in &response.headers {
        response_str.push_str(&format!(
            "{key}: {value}
"
        ));
    }

    let has_content_length = response
        .headers
        .keys()
        .any(|k| k.to_lowercase() == "content-length");
    if !has_content_length {
        let length = match &response.body {
            ResponseBody::Text(text) => text.len(),
            ResponseBody::StaticText(text) => text.len(),
            ResponseBody::Binary(bytes) => bytes.len(),
            ResponseBody::StaticBinary(bytes) => bytes.len(),
            ResponseBody::Stream(stream_body) => stream_body.size as usize,
        };
        response_str.push_str(&format!(
            "Content-Length: {length}
"
        ));
    }

    response_str.push_str("\r\n");
    stream.write_all(response_str.as_bytes()).await?;

    let mut body_sent: u64 = 0;
    match response.body {
        ResponseBody::Text(text) => {
            let bytes = text.as_bytes();
            stream.write_all(bytes).await?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::StaticText(text) => {
            let bytes = text.as_bytes();
            stream.write_all(bytes).await?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::Binary(bytes) => {
            stream.write_all(&bytes).await?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::StaticBinary(bytes) => {
            stream.write_all(bytes).await?;
            body_sent += bytes.len() as u64;
        }
        ResponseBody::Stream(stream_body) => {
            let mut file = tokio::fs::File::open(&stream_body.path).await?;
            let mut buffer = vec![0; stream_body.chunk_size];
            loop {
                let bytes_read = file.read(&mut buffer).await?;
                if bytes_read == 0 {
                    break;
                }
                stream.write_all(&buffer[..bytes_read]).await?;
                body_sent += bytes_read as u64;
            }
        }
    }

    stream.flush().await?;
    trace!("{log_prefix} sent {body_sent} bytes");
    Ok(body_sent)
}

async fn read_with_timeout<S>(stream: &mut S, buf: &mut [u8]) -> Result<usize, AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    match tokio::time::timeout(Duration::from_secs(30), stream.read(buf)).await {
        Ok(result) => result.map_err(AppError::Io),
        Err(_) => Err(AppError::Io(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "read timeout",
        ))),
    }
}

async fn read_headers_with_remaining_async<S>(stream: &mut S) -> Result<(String, Vec<u8>), AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let mut buffer = vec![0; MAX_HEADERS_SIZE];
    let mut total_read = 0;

    loop {
        let bytes_read = read_with_timeout(stream, &mut buffer[total_read..]).await?;
        if bytes_read == 0 {
            if total_read == 0 {
                return Err(AppError::BadRequest);
            }
            break;
        }
        total_read += bytes_read;

        let double_crlf = b"\r\n\r\n";
        let double_lf = b"\n\n";

        if let Some(pos) = buffer[0..total_read]
            .windows(4)
            .position(|window| window == double_crlf)
        {
            let headers_end = pos;
            let body_start = pos + 4;
            let headers_data =
                std::str::from_utf8(&buffer[0..headers_end]).map_err(|_| AppError::BadRequest)?;
            let remaining_bytes = buffer[body_start..total_read].to_vec();
            return Ok((headers_data.to_string(), remaining_bytes));
        }

        if let Some(pos) = buffer[0..total_read]
            .windows(2)
            .position(|window| window == double_lf)
        {
            let headers_end = pos;
            let body_start = pos + 2;
            let headers_data =
                std::str::from_utf8(&buffer[0..headers_end]).map_err(|_| AppError::BadRequest)?;
            let remaining_bytes = buffer[body_start..total_read].to_vec();
            return Ok((headers_data.to_string(), remaining_bytes));
        }

        if total_read >= buffer.len() {
            return Err(AppError::BadRequest);
        }
    }

    let data = std::str::from_utf8(&buffer[0..total_read]).map_err(|_| AppError::BadRequest)?;
    Ok((data.to_string(), Vec::new()))
}

async fn read_request_body_async<S>(
    stream: &mut S,
    headers: &HashMap<String, String>,
    remaining_bytes: Vec<u8>,
) -> Result<Option<RequestBody>, AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let has_content_length = headers.contains_key("content-length");
    let has_chunked_transfer = Request::has_chunked_transfer_encoding(headers);

    if has_content_length && has_chunked_transfer {
        return Err(AppError::BadRequest);
    }

    if has_chunked_transfer {
        let body = read_chunked_body_async(stream, remaining_bytes).await?;
        return Ok(Some(body));
    }

    let content_length = match headers.get("content-length") {
        Some(length_str) => length_str
            .parse::<usize>()
            .map_err(|_| AppError::BadRequest)?,
        None => return Ok(None),
    };

    if content_length == 0 {
        return Ok(Some(RequestBody::Memory(Vec::new())));
    }

    if content_length > MAX_REQUEST_BODY_SIZE {
        return Err(AppError::PayloadTooLarge(MAX_REQUEST_BODY_SIZE as u64));
    }

    if content_length <= STREAM_TO_DISK_THRESHOLD {
        let body = read_body_to_memory_async(stream, content_length, remaining_bytes).await?;
        Ok(Some(RequestBody::Memory(body)))
    } else {
        let (path, size) = read_body_to_disk_async(stream, content_length, remaining_bytes).await?;
        Ok(Some(RequestBody::File { path, size }))
    }
}

async fn read_chunked_body_async<S>(
    stream: &mut S,
    mut pending: Vec<u8>,
) -> Result<RequestBody, AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    const CHUNK_LINE_LIMIT: usize = 8 * 1024;

    let mut total_size: usize = 0;
    let mut memory_body: Vec<u8> = Vec::new();
    let mut file_sink: Option<(PathBuf, tokio::fs::File)> = None;

    loop {
        let line = read_crlf_line_async(stream, &mut pending, CHUNK_LINE_LIMIT).await?;
        let line_str = std::str::from_utf8(&line).map_err(|_| AppError::BadRequest)?;
        let size_token = line_str
            .split(';')
            .next()
            .ok_or(AppError::BadRequest)?
            .trim();
        if size_token.is_empty() {
            return Err(AppError::BadRequest);
        }

        let chunk_size = usize::from_str_radix(size_token, 16).map_err(|_| AppError::BadRequest)?;
        if chunk_size == 0 {
            consume_chunked_trailers_async(stream, &mut pending).await?;
            break;
        }

        let next_total = total_size
            .checked_add(chunk_size)
            .ok_or(AppError::PayloadTooLarge(MAX_REQUEST_BODY_SIZE as u64))?;
        if next_total > MAX_REQUEST_BODY_SIZE {
            return Err(AppError::PayloadTooLarge(MAX_REQUEST_BODY_SIZE as u64));
        }

        let chunk_data = read_exact_from_buffer_async(stream, &mut pending, chunk_size).await?;
        consume_expected_crlf_async(stream, &mut pending).await?;

        if file_sink.is_none() && next_total <= STREAM_TO_DISK_THRESHOLD {
            memory_body.extend_from_slice(&chunk_data);
        } else {
            if file_sink.is_none() {
                let (temp_path, mut temp_file) = create_temp_body_file_async().await?;
                if !memory_body.is_empty() {
                    temp_file.write_all(&memory_body).await.map_err(|e| {
                        let _ = std::fs::remove_file(&temp_path);
                        AppError::from(e)
                    })?;
                    memory_body.clear();
                }
                file_sink = Some((temp_path, temp_file));
            }
            if let Some((temp_path, temp_file)) = file_sink.as_mut() {
                temp_file.write_all(&chunk_data).await.map_err(|e| {
                    let _ = std::fs::remove_file(temp_path);
                    AppError::from(e)
                })?;
            }
        }

        total_size = next_total;
    }

    if let Some((temp_path, temp_file)) = file_sink.as_mut() {
        temp_file.sync_all().await.map_err(|e| {
            let _ = std::fs::remove_file(temp_path);
            AppError::from(e)
        })?;
    }

    if let Some((temp_path, _)) = file_sink {
        Ok(RequestBody::File {
            path: temp_path,
            size: total_size as u64,
        })
    } else {
        Ok(RequestBody::Memory(memory_body))
    }
}

async fn create_temp_body_file_async() -> Result<(PathBuf, tokio::fs::File), AppError> {
    let temp_filename = format!(
        "irondrop_request_{}_{:x}_{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let temp_path = std::env::temp_dir().join(temp_filename);
    let temp_file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .await
        .map_err(|e| {
            error!("Failed to create temporary file {temp_path:?}: {e}");
            AppError::from(e)
        })?;
    Ok((temp_path, temp_file))
}

async fn read_crlf_line_async<S>(
    stream: &mut S,
    pending: &mut Vec<u8>,
    max_line_len: usize,
) -> Result<Vec<u8>, AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    loop {
        if let Some(pos) = pending.windows(2).position(|w| w == b"\r\n") {
            let line = pending[..pos].to_vec();
            pending.drain(0..pos + 2);
            return Ok(line);
        }

        if pending.len() > max_line_len + 2 {
            return Err(AppError::BadRequest);
        }

        let mut buffer = [0u8; 8192];
        let n = read_with_timeout(stream, &mut buffer).await?;
        if n == 0 {
            return Err(AppError::BadRequest);
        }
        pending.extend_from_slice(&buffer[..n]);
    }
}

async fn read_exact_from_buffer_async<S>(
    stream: &mut S,
    pending: &mut Vec<u8>,
    count: usize,
) -> Result<Vec<u8>, AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    while pending.len() < count {
        let mut buffer = [0u8; 8192];
        let n = read_with_timeout(stream, &mut buffer).await?;
        if n == 0 {
            return Err(AppError::BadRequest);
        }
        pending.extend_from_slice(&buffer[..n]);
    }
    Ok(pending.drain(0..count).collect())
}

async fn consume_expected_crlf_async<S>(
    stream: &mut S,
    pending: &mut Vec<u8>,
) -> Result<(), AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let crlf = read_exact_from_buffer_async(stream, pending, 2).await?;
    if crlf != b"\r\n" {
        return Err(AppError::BadRequest);
    }
    Ok(())
}

async fn consume_chunked_trailers_async<S>(
    stream: &mut S,
    pending: &mut Vec<u8>,
) -> Result<(), AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let mut total_trailer_size = 0usize;
    loop {
        let line = read_crlf_line_async(stream, pending, MAX_HEADERS_SIZE).await?;
        total_trailer_size += line.len() + 2;
        if total_trailer_size > MAX_HEADERS_SIZE {
            return Err(AppError::BadRequest);
        }
        if line.is_empty() {
            break;
        }
    }
    Ok(())
}

async fn read_body_to_memory_async<S>(
    stream: &mut S,
    content_length: usize,
    remaining_bytes: Vec<u8>,
) -> Result<Vec<u8>, AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let mut body = Vec::with_capacity(content_length);
    let bytes_from_headers = remaining_bytes.len().min(content_length);
    body.extend_from_slice(&remaining_bytes[..bytes_from_headers]);
    let bytes_needed = content_length - bytes_from_headers;

    if bytes_needed > 0 {
        let mut bytes_read = 0;
        let chunk_size = 8192;
        let mut buffer = vec![0; chunk_size];
        while bytes_read < bytes_needed {
            let to_read = (bytes_needed - bytes_read).min(chunk_size);
            let n = read_with_timeout(stream, &mut buffer[..to_read]).await?;
            if n == 0 {
                return Err(AppError::BadRequest);
            }
            body.extend_from_slice(&buffer[..n]);
            bytes_read += n;
        }
    }

    if body.len() != content_length {
        return Err(AppError::BadRequest);
    }
    Ok(body)
}

async fn read_body_to_disk_async<S>(
    stream: &mut S,
    content_length: usize,
    remaining_bytes: Vec<u8>,
) -> Result<(PathBuf, u64), AppError>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let temp_filename = format!(
        "irondrop_request_{}_{:x}_{}.tmp",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    );

    let temp_path = std::env::temp_dir().join(&temp_filename);
    let mut temp_file = tokio::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .await
        .map_err(|e| {
            error!("Failed to create temporary file {temp_path:?}: {e}");
            AppError::from(e)
        })?;

    let mut total_written: usize = 0;
    if !remaining_bytes.is_empty() {
        let bytes_to_write = remaining_bytes.len().min(content_length);
        temp_file
            .write_all(&remaining_bytes[..bytes_to_write])
            .await
            .map_err(|e| {
                let _ = std::fs::remove_file(&temp_path);
                AppError::from(e)
            })?;
        total_written += bytes_to_write;
    }

    let bytes_needed = content_length - total_written;
    if bytes_needed > 0 {
        let mut bytes_read = 0usize;
        let chunk_size = 64 * 1024;
        let mut buffer = vec![0; chunk_size];
        while bytes_read < bytes_needed {
            let to_read = (bytes_needed - bytes_read).min(chunk_size);
            let n = read_with_timeout(stream, &mut buffer[..to_read]).await?;
            if n == 0 {
                let _ = std::fs::remove_file(&temp_path);
                return Err(AppError::BadRequest);
            }
            temp_file.write_all(&buffer[..n]).await.map_err(|e| {
                let _ = std::fs::remove_file(&temp_path);
                AppError::from(e)
            })?;
            bytes_read += n;
            total_written += n;
        }
    }

    if total_written != content_length {
        let _ = std::fs::remove_file(&temp_path);
        return Err(AppError::BadRequest);
    }

    temp_file.sync_all().await.map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        AppError::from(e)
    })?;

    Ok((temp_path, total_written as u64))
}
