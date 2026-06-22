// SPDX-License-Identifier: MIT

use crate::cli::Cli;
use crate::config::Config;
use crate::error::AppError;
use crate::handlers::register_internal_routes;
use crate::middleware::AuthMiddleware;
use crate::router::Router;
use glob::Pattern;
use log::{debug, info, trace, warn};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use rustls::ServerConfig;
use std::io::BufReader;
use tokio::net::TcpStream as TokioTcpStream;

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::mem;

/// Rate limiter for basic DoS protection
#[derive(Clone)]
pub struct RateLimiter {
    connections: Arc<Vec<Mutex<HashMap<IpAddr, ConnectionInfo>>>>,
    max_requests_per_minute: u32,
    max_concurrent_per_ip: u32,
    max_connections_per_ip: u32,
}

#[derive(Debug)]
struct ConnectionInfo {
    request_count: u32,
    last_reset: Instant,
    active_connections: u32,
    last_activity: Instant,
    total_connections: u32,
}

const RATE_LIMITER_SHARDS: usize = 64;
const MAX_RATE_LIMITER_ENTRIES: usize = 100_000;
const MAX_RATE_LIMITER_ENTRIES_PER_SHARD: usize =
    MAX_RATE_LIMITER_ENTRIES.div_ceil(RATE_LIMITER_SHARDS);

impl RateLimiter {
    pub fn new(max_requests_per_minute: u32, max_concurrent_per_ip: u32) -> Self {
        let mut shards = Vec::with_capacity(RATE_LIMITER_SHARDS);
        for _ in 0..RATE_LIMITER_SHARDS {
            shards.push(Mutex::new(HashMap::new()));
        }
        Self {
            connections: Arc::new(shards),
            max_requests_per_minute,
            max_concurrent_per_ip,
            max_connections_per_ip: 1000, // Limit stored connections per IP
        }
    }

    fn shard_index(ip: IpAddr) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        ip.hash(&mut hasher);
        (hasher.finish() as usize) % RATE_LIMITER_SHARDS
    }

    pub fn check_rate_limit(&self, ip: IpAddr) -> bool {
        trace!("Checking rate limit for IP: {}", ip);
        let shard_index = Self::shard_index(ip);
        let mut connections = self.connections[shard_index].lock().unwrap();
        let now = Instant::now();

        // Check if we need to evict entries before inserting a new one
        if !connections.contains_key(&ip)
            && connections.len() >= MAX_RATE_LIMITER_ENTRIES_PER_SHARD
        {
            const EVICTION_SAMPLE: usize = 64;
            let mut best_ip: Option<IpAddr> = None;
            let mut best_last_activity: Option<Instant> = None;
            let mut best_is_idle = false;

            for (&candidate_ip, info) in connections.iter().take(EVICTION_SAMPLE) {
                let is_idle = info.active_connections == 0;
                if best_ip.is_none()
                    || (is_idle && !best_is_idle)
                    || (is_idle == best_is_idle
                        && best_last_activity
                            .map(|t| info.last_activity < t)
                            .unwrap_or(true))
                {
                    best_ip = Some(candidate_ip);
                    best_last_activity = Some(info.last_activity);
                    best_is_idle = is_idle;
                }
            }

            if let Some(victim) = best_ip {
                connections.remove(&victim);
                debug!(
                    "Evicted rate limiter entry for IP {} to make space for new IP {}",
                    victim, ip
                );
            }
        }

        let conn_info = connections.entry(ip).or_insert(ConnectionInfo {
            request_count: 0,
            last_reset: now,
            active_connections: 0,
            last_activity: now,
            total_connections: 0,
        });

        trace!(
            "IP {} current state: requests={}, active_conns={}, total_conns={}",
            ip, conn_info.request_count, conn_info.active_connections, conn_info.total_connections
        );

        // Reset counter if more than a minute has passed
        if now.duration_since(conn_info.last_reset) >= Duration::from_secs(60) {
            trace!("Resetting request counter for IP {} (minute elapsed)", ip);
            conn_info.request_count = 0;
            conn_info.last_reset = now;
        }

        // Check concurrent connections
        if conn_info.active_connections >= self.max_concurrent_per_ip {
            warn!("Rate limit exceeded for {ip}: too many concurrent connections");
            return false;
        }

        // Check request rate
        if conn_info.request_count >= self.max_requests_per_minute {
            warn!("Rate limit exceeded for {ip}: too many requests per minute");
            return false;
        }

        conn_info.request_count += 1;
        conn_info.active_connections += 1;
        conn_info.last_activity = now;
        conn_info.total_connections += 1;

        trace!(
            "Rate limit check passed for IP {}: new counts - requests={}, active_conns={}",
            ip, conn_info.request_count, conn_info.active_connections
        );

        // Check if this IP has too many stored connections
        if conn_info.total_connections > self.max_connections_per_ip {
            warn!("IP {ip} has exceeded max stored connections limit");
        }

        true
    }

    pub fn release_connection(&self, ip: IpAddr) {
        trace!("Releasing connection for IP: {}", ip);
        let shard_index = Self::shard_index(ip);
        if let Ok(mut connections) = self.connections[shard_index].lock() {
            if let Some(conn_info) = connections.get_mut(&ip) {
                let old_count = conn_info.active_connections;
                conn_info.active_connections = conn_info.active_connections.saturating_sub(1);
                conn_info.last_activity = Instant::now();
                trace!(
                    "IP {} active connections: {} -> {}",
                    ip, old_count, conn_info.active_connections
                );
            } else {
                trace!("No connection info found for IP {} during release", ip);
            }
        }
    }

    pub fn cleanup_old_entries(&self) {
        trace!("Starting manual cleanup of old rate limiter entries");
        let now = Instant::now();
        let mut cleaned_count = 0usize;

        for shard in self.connections.iter() {
            let mut connections = shard.lock().unwrap();
            let initial_count = connections.len();

            trace!("Rate limiter shard has {} entries before cleanup", initial_count);

            // Reduced retention time from 5 minutes to 2 minutes
            connections
                .retain(|_, info| now.duration_since(info.last_activity) < Duration::from_secs(120));

            cleaned_count += initial_count - connections.len();
            if initial_count > connections.len() && connections.capacity() > connections.len() * 2 {
                connections.shrink_to_fit();
            }
        }

        if cleaned_count == 0 {
            trace!("No old entries to clean up");
        } else {
            debug!("Cleaned up {} old rate limiter entries", cleaned_count);
        }
    }

    /// Perform aggressive cleanup when memory pressure is detected
    pub fn cleanup_on_memory_pressure(&self) {
        debug!("Starting aggressive cleanup due to memory pressure");
        let now = Instant::now();
        let mut cleaned_count = 0usize;

        for shard in self.connections.iter() {
            let mut connections = shard.lock().unwrap();
            let initial_count = connections.len();

            trace!(
                "Memory pressure cleanup: checking {} entries in shard",
                initial_count
            );

            // More aggressive cleanup - remove entries older than 30 seconds
            connections.retain(|_, info| {
                info.active_connections > 0
                    || now.duration_since(info.last_activity) < Duration::from_secs(30)
            });

            cleaned_count += initial_count - connections.len();
            if initial_count > connections.len() && connections.capacity() > connections.len() * 2 {
                connections.shrink_to_fit();
            }
        }

        if cleaned_count > 0 {
            warn!(
                "Memory pressure cleanup removed {} rate limiter entries",
                cleaned_count
            );
        } else {
            trace!("Memory pressure cleanup: no entries removed");
        }
    }

    /// Get rate limiter memory statistics
    pub fn get_memory_stats(&self) -> (usize, usize) {
        let mut entry_count = 0usize;
        let mut estimated_memory = 0usize;

        for shard in self.connections.iter() {
            match shard.lock() {
                Ok(connections) => {
                    entry_count += connections.len();
                    estimated_memory +=
                        connections.len() * std::mem::size_of::<(IpAddr, ConnectionInfo)>();
                }
                Err(_) => {
                    trace!("Failed to acquire rate limiter shard lock for stats");
                    return (0, 0);
                }
            }
        }

        trace!(
            "Rate limiter stats: {} entries, ~{} bytes across {} shards",
            entry_count,
            estimated_memory,
            RATE_LIMITER_SHARDS
        );
        (entry_count, estimated_memory)
    }
}

/// Comprehensive server statistics and monitoring
///
/// Tracks both HTTP request statistics and file upload metrics with thread-safe
/// concurrent access using Arc<Mutex<T>> for all counters.
///
/// # Request Statistics
/// - Total requests processed (successful and failed)
/// - Bytes served via downloads
/// - Server uptime tracking
///
/// # Upload Statistics
/// - Upload request counts and success rates
/// - File upload counts and total bytes uploaded
/// - Processing time metrics and concurrent upload tracking
/// - Largest upload size tracking for capacity planning
///
/// All statistics are automatically reported every 5 minutes in the background
/// and provide comprehensive insights into server usage and performance.
pub struct ServerStats {
    // Request statistics
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    error_requests: AtomicU64,
    bytes_served: AtomicU64,
    start_time: Instant,

    // Upload statistics
    total_uploads: AtomicU64,
    successful_uploads: AtomicU64,
    failed_uploads: AtomicU64,
    files_uploaded: AtomicU64,
    upload_bytes: AtomicU64,
    largest_upload: AtomicU64,
    concurrent_uploads: AtomicU64,
    upload_processing_times: Mutex<Vec<u64>>,

    // Memory statistics
    process_memory_bytes: Mutex<Option<u64>>,
    peak_memory_bytes: Mutex<Option<u64>>,
    last_memory_check: Mutex<Option<Instant>>,
    memory_available: AtomicBool,
}

impl ServerStats {
    pub fn new() -> Self {
        Self {
            // Request statistics
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            error_requests: AtomicU64::new(0),
            bytes_served: AtomicU64::new(0),
            start_time: Instant::now(),

            // Upload statistics
            total_uploads: AtomicU64::new(0),
            successful_uploads: AtomicU64::new(0),
            failed_uploads: AtomicU64::new(0),
            files_uploaded: AtomicU64::new(0),
            upload_bytes: AtomicU64::new(0),
            largest_upload: AtomicU64::new(0),
            concurrent_uploads: AtomicU64::new(0),
            upload_processing_times: Mutex::new(Vec::new()),

            // Memory statistics
            process_memory_bytes: Mutex::new(None),
            peak_memory_bytes: Mutex::new(None),
            last_memory_check: Mutex::new(None),
            memory_available: AtomicBool::new(true),
        }
    }

    pub fn record_request(&self, success: bool, bytes: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.error_requests.fetch_add(1, Ordering::Relaxed);
        }
        self.bytes_served.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64, Duration) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let errors = self.error_requests.load(Ordering::Relaxed);
        let bytes = self.bytes_served.load(Ordering::Relaxed);
        let uptime = self.start_time.elapsed();

        (total, successful, errors, bytes, uptime)
    }

    /// Record an upload request and track statistics
    pub fn record_upload_request(
        &self,
        success: bool,
        files_count: u64,
        upload_bytes: u64,
        processing_time_ms: u64,
        largest_file: u64,
    ) {
        trace!(
            "Recording upload: success={}, files={}, bytes={}, time={}ms, largest={}",
            success, files_count, upload_bytes, processing_time_ms, largest_file
        );

        // Increment total uploads
        self.total_uploads.fetch_add(1, Ordering::Relaxed);

        // Track success/failure
        if success {
            self.successful_uploads.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_uploads.fetch_add(1, Ordering::Relaxed);
        }

        // Only record additional metrics for successful uploads
        if success {
            // Record number of files uploaded
            self.files_uploaded
                .fetch_add(files_count, Ordering::Relaxed);

            // Record total bytes uploaded
            self.upload_bytes.fetch_add(upload_bytes, Ordering::Relaxed);
            let mut current = self.largest_upload.load(Ordering::Relaxed);
            while largest_file > current {
                match self.largest_upload.compare_exchange_weak(
                    current,
                    largest_file,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(next) => current = next,
                }
            }

            // Record processing time (keep last 100 entries for average calculation)
            if let Ok(mut times) = self.upload_processing_times.lock() {
                times.push(processing_time_ms);
                let len = times.len();
                if len > 100 {
                    times.drain(0..len - 100);
                    if times.capacity() > 200 {
                        times.shrink_to(100);
                    }
                }
            }
        }
    }

    /// Track concurrent upload start
    pub fn start_upload(&self) {
        self.concurrent_uploads.fetch_add(1, Ordering::Relaxed);
    }

    /// Track concurrent upload completion
    pub fn finish_upload(&self) {
        let mut current = self.concurrent_uploads.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_sub(1);
            match self.concurrent_uploads.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(v) => current = v,
            }
        }
    }

    /// Get upload statistics
    pub fn get_upload_stats(&self) -> UploadStats {
        let total_uploads = self.total_uploads.load(Ordering::Relaxed);
        let successful_uploads = self.successful_uploads.load(Ordering::Relaxed);
        let failed_uploads = self.failed_uploads.load(Ordering::Relaxed);
        let files_uploaded = self.files_uploaded.load(Ordering::Relaxed);
        let upload_bytes = self.upload_bytes.load(Ordering::Relaxed);
        let largest_upload = self.largest_upload.load(Ordering::Relaxed);
        let concurrent_uploads = self.concurrent_uploads.load(Ordering::Relaxed);

        let processing_times = self
            .upload_processing_times
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let average_processing_time = if processing_times.is_empty() {
            0.0
        } else {
            processing_times.iter().sum::<u64>() as f64 / processing_times.len() as f64
        };

        UploadStats {
            total_uploads,
            successful_uploads,
            failed_uploads,
            files_uploaded,
            upload_bytes,
            average_upload_size: if files_uploaded > 0 {
                upload_bytes / files_uploaded
            } else {
                0
            },
            largest_upload,
            concurrent_uploads,
            average_processing_time,
            success_rate: if total_uploads > 0 {
                (successful_uploads as f64 / total_uploads as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    /// Check if memory pressure is detected (memory usage > 150MB)
    /// This triggers aggressive cleanup in rate limiter and other components
    pub fn check_memory_pressure(&self, rate_limiter: Option<&RateLimiter>) -> bool {
        trace!("Checking memory pressure");
        let (current_memory, _, available) = self.get_memory_usage();

        if available {
            if let Some(memory) = current_memory {
                // Trigger cleanup if memory exceeds 150MB (baseline is ~30MB)
                let memory_mb = memory / (1024 * 1024);
                trace!("Current memory usage: {}MB", memory_mb);

                if memory_mb > 150 {
                    warn!("Memory pressure detected: {}MB usage", memory_mb);

                    // Trigger rate limiter cleanup if provided
                    if let Some(limiter) = rate_limiter {
                        debug!("Triggering rate limiter cleanup due to memory pressure");
                        limiter.cleanup_on_memory_pressure();
                    }

                    // Clear search cache if available
                    debug!("Clearing search cache due to memory pressure");
                    crate::search::clear_cache();

                    return true;
                }
                trace!("Memory usage within normal limits: {}MB", memory_mb);
            } else {
                trace!("Memory usage unavailable but tracking is enabled");
            }
        } else {
            trace!("Memory tracking not available");
        }
        false
    }

    /// Get current process memory usage in bytes
    ///
    /// This function implements cross-platform memory reading with caching
    /// to avoid frequent expensive syscalls. Memory is cached for 5 seconds.
    /// Returns (current_memory, peak_memory, available) where memory values
    /// are None if memory tracking is unavailable.
    pub fn get_memory_usage(&self) -> (Option<u64>, Option<u64>, bool) {
        let now = Instant::now();

        // Check if we need to refresh the memory reading (cache for 5 seconds)
        let should_refresh = {
            let last_check = self.last_memory_check.lock().unwrap();
            match *last_check {
                Some(last) => {
                    let elapsed = now.duration_since(last);
                    let should_refresh = elapsed >= Duration::from_secs(5);
                    trace!(
                        "Memory cache check: elapsed={}s, should_refresh={}",
                        elapsed.as_secs(),
                        should_refresh
                    );
                    should_refresh
                }
                None => {
                    trace!("First memory check, refreshing");
                    true
                }
            }
        };

        if should_refresh {
            trace!("Refreshing memory statistics");
            let current_memory_opt = get_process_memory_bytes();

            // Update availability status
            let is_available = current_memory_opt.is_some();
            let previous = self.memory_available.swap(is_available, Ordering::Relaxed);
            if !previous && is_available {
                info!("Memory tracking is now available");
            } else if previous && !is_available {
                info!("Memory tracking is no longer available");
            }

            // Update current memory
            if let Ok(mut mem) = self.process_memory_bytes.lock() {
                *mem = current_memory_opt;
            }

            // Update peak memory if this is higher
            if let (Some(current_memory), Ok(mut peak)) =
                (current_memory_opt, self.peak_memory_bytes.lock())
            {
                match *peak {
                    Some(peak_val) => {
                        if current_memory > peak_val {
                            *peak = Some(current_memory);
                        }
                    }
                    None => {
                        *peak = Some(current_memory);
                    }
                }
            }

            // Update last check time
            if let Ok(mut last_check) = self.last_memory_check.lock() {
                *last_check = Some(now);
            }
        }

        // Return current and peak memory with availability
        let current = *self
            .process_memory_bytes
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let peak = *self
            .peak_memory_bytes
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let available = self.memory_available.load(Ordering::Relaxed);
        (current, peak, available)
    }

    /// Force refresh memory statistics (bypasses cache)
    pub fn refresh_memory_stats(&self) {
        let current_memory_opt = get_process_memory_bytes();

        // Update availability status
        let is_available = current_memory_opt.is_some();
        self.memory_available.store(is_available, Ordering::Relaxed);

        if let Ok(mut mem) = self.process_memory_bytes.lock() {
            *mem = current_memory_opt;
        }

        if let (Some(current_memory), Ok(mut peak)) =
            (current_memory_opt, self.peak_memory_bytes.lock())
        {
            match *peak {
                Some(peak_val) => {
                    if current_memory > peak_val {
                        *peak = Some(current_memory);
                    }
                }
                None => {
                    *peak = Some(current_memory);
                }
            }
        }

        if let Ok(mut last_check) = self.last_memory_check.lock() {
            *last_check = Some(Instant::now());
        }
    }
}

impl Default for ServerStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Cross-platform process memory reading
///
/// Returns current process memory usage in bytes, or None if unavailable.
/// Prioritizes Linux /proc/self/status, with fallbacks for other platforms.
/// Returns None when memory tracking is restricted (e.g., containers, CI environments).
fn get_process_memory_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        match fs::read_to_string("/proc/self/status") {
            Ok(status) => {
                for line in status.lines() {
                    if line.starts_with("VmRSS:")
                        && let Some(kb_str) = line.split_whitespace().nth(1)
                        && let Ok(kb) = kb_str.parse::<u64>()
                    {
                        return Some(kb * 1024); // Convert KB to bytes
                    }
                }
                // Parsing succeeded but VmRSS not found - unusual but possible
                debug!("VmRSS not found in /proc/self/status");
                None
            }
            Err(e) => {
                // Log different error types with appropriate levels
                match e.kind() {
                    std::io::ErrorKind::NotFound => {
                        debug!("Memory tracking unavailable: /proc/self/status not found");
                    }
                    std::io::ErrorKind::PermissionDenied => {
                        debug!("Memory tracking unavailable: /proc/self/status access denied");
                    }
                    _ => {
                        warn!("Memory tracking unavailable: failed to read /proc/self/status: {e}");
                    }
                }
                None
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_void;

        // time_value_t (seconds + microseconds) used in mach_task_basic_info
        #[repr(C)]
        #[allow(non_camel_case_types)]
        struct time_value_t {
            seconds: i32,
            microseconds: i32,
        }

        // Layout aligned with <mach/task_info.h> (basic flavor)
        #[repr(C)]
        #[allow(non_camel_case_types)]
        struct mach_task_basic_info {
            virtual_size: u64,
            resident_size: u64,
            resident_size_max: u64,
            user_time: time_value_t,
            system_time: time_value_t,
            policy: i32,
            suspend_count: i32,
        }

        // TASK_VM_INFO fallback struct (subset; order matters)
        #[repr(C)]
        #[allow(non_camel_case_types)]
        struct task_vm_info {
            virtual_size: u64,
            region_count: i32,
            page_size: i32,
            resident_size: u64,
            resident_size_peak: u64,
            device: u64,
            device_peak: u64,
            internal: u64,
            internal_peak: u64,
            external: u64,
            external_peak: u64,
            reusable: u64,
            reusable_peak: u64,
            purgeable_volatile_pmap: u64,
            purgeable_volatile_resident: u64,
            purgeable_volatile_virtual: u64,
            compressed: u64,
            compressed_peak: u64,
            compressed_lifetime: u64,
            phys_footprint: u64,
            min_address: u64,
            max_address: u64,
        }

        unsafe extern "C" {
            fn mach_task_self() -> u32;
            fn task_info(
                target_task: u32,
                flavor: u32,
                task_info_out: *mut c_void,
                task_info_outCnt: *mut u32,
            ) -> i32;
        }

        const MACH_TASK_BASIC_INFO: u32 = 20; // flavor constant
        const TASK_VM_INFO: u32 = 22; // fallback flavor
        // natural_t == u32; express counts in u32 units
        const MACH_TASK_BASIC_INFO_COUNT: u32 =
            (std::mem::size_of::<mach_task_basic_info>() / std::mem::size_of::<u32>()) as u32;
        const TASK_VM_INFO_COUNT: u32 =
            (std::mem::size_of::<task_vm_info>() / std::mem::size_of::<u32>()) as u32;

        unsafe {
            let mut basic: mach_task_basic_info = mem::zeroed();
            let mut count = MACH_TASK_BASIC_INFO_COUNT;
            let r_basic = task_info(
                mach_task_self(),
                MACH_TASK_BASIC_INFO,
                &mut basic as *mut _ as *mut c_void,
                &mut count,
            );
            if r_basic == 0 && basic.resident_size > 0 {
                return Some(basic.resident_size as u64);
            }
            debug!("mach_task_basic_info failed (code {r_basic}), trying TASK_VM_INFO");
            let mut vm: task_vm_info = mem::zeroed();
            let mut vm_count = TASK_VM_INFO_COUNT;
            let r_vm = task_info(
                mach_task_self(),
                TASK_VM_INFO,
                &mut vm as *mut _ as *mut c_void,
                &mut vm_count,
            );
            if r_vm == 0 && vm.resident_size > 0 {
                return Some(vm.resident_size as u64);
            }
            debug!("TASK_VM_INFO failed (code {r_vm}) on macOS");
        }
        debug!("Memory tracking unavailable: macOS APIs failed");
        None
    }

    #[cfg(target_os = "windows")]
    {
        use std::ffi::c_void;

        #[repr(C)]
        #[allow(non_snake_case)]
        struct PROCESS_MEMORY_COUNTERS {
            cb: u32,
            PageFaultCount: u32,
            PeakWorkingSetSize: usize,
            WorkingSetSize: usize,
            QuotaPeakPagedPoolUsage: usize,
            QuotaPagedPoolUsage: usize,
            QuotaPeakNonPagedPoolUsage: usize,
            QuotaNonPagedPoolUsage: usize,
            PagefileUsage: usize,
            PeakPagefileUsage: usize,
        }

        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut c_void;
        }

        #[link(name = "psapi")]
        unsafe extern "system" {
            fn GetProcessMemoryInfo(
                hProcess: *mut c_void,
                ppsmemCounters: *mut PROCESS_MEMORY_COUNTERS,
                cb: u32,
            ) -> i32;
        }

        unsafe {
            let mut pmc: PROCESS_MEMORY_COUNTERS = mem::zeroed();
            pmc.cb = mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

            let result = GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb);

            if result != 0 {
                return Some(pmc.WorkingSetSize as u64);
            }
        }
        debug!("Memory tracking unavailable: failed to get memory info on Windows");
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        debug!("Memory tracking not supported on this platform");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;
    use std::thread;

    #[test]
    fn test_upload_statistics_tracking() {
        let stats = ServerStats::new();

        // Verify initial state
        let initial_stats = stats.get_upload_stats();
        assert_eq!(initial_stats.total_uploads, 0);
        assert_eq!(initial_stats.successful_uploads, 0);
        assert_eq!(initial_stats.failed_uploads, 0);
        assert_eq!(initial_stats.files_uploaded, 0);
        assert_eq!(initial_stats.upload_bytes, 0);
        assert_eq!(initial_stats.success_rate, 0.0);

        // Test concurrent upload tracking
        stats.start_upload();
        stats.start_upload();
        let concurrent_stats = stats.get_upload_stats();
        assert_eq!(concurrent_stats.concurrent_uploads, 2);

        // Test successful upload recording
        stats.record_upload_request(true, 3, 1024, 150, 512);
        stats.finish_upload();

        stats.record_upload_request(true, 2, 2048, 200, 1024);
        stats.finish_upload();

        let success_stats = stats.get_upload_stats();
        assert_eq!(success_stats.total_uploads, 2);
        assert_eq!(success_stats.successful_uploads, 2);
        assert_eq!(success_stats.failed_uploads, 0);
        assert_eq!(success_stats.files_uploaded, 5);
        assert_eq!(success_stats.upload_bytes, 3072);
        assert_eq!(success_stats.largest_upload, 1024);
        assert_eq!(success_stats.success_rate, 100.0);
        assert_eq!(success_stats.average_upload_size, 3072 / 5);
        assert_eq!(success_stats.average_processing_time, 175.0); // (150 + 200) / 2

        // Test failed upload recording
        stats.record_upload_request(false, 0, 0, 0, 0);

        let final_stats = stats.get_upload_stats();
        assert_eq!(final_stats.total_uploads, 3);
        assert_eq!(final_stats.successful_uploads, 2);
        assert_eq!(final_stats.failed_uploads, 1);
        assert!((final_stats.success_rate - (200.0 / 3.0)).abs() < 0.01);
    }

    #[test]
    fn test_memory_tracking() {
        let stats = ServerStats::new();

        // Test initial memory state
        let (current, peak, available) = stats.get_memory_usage();

        // Test behavior based on availability
        if available {
            // When memory tracking is available, we should have Some values
            assert!(current.is_some());
            assert!(peak.is_some());
            assert!(current.unwrap() <= peak.unwrap());

            // Test forced refresh
            stats.refresh_memory_stats();
            let (current2, peak2, available2) = stats.get_memory_usage();

            assert!(available2);
            assert!(current2.is_some());
            assert!(peak2.is_some());

            // Peak should never decrease
            assert!(peak2.unwrap() >= peak.unwrap());
            // Current might change but should be reasonable
            assert!(current2.unwrap() <= peak2.unwrap());
        } else {
            // When memory tracking is unavailable, values should be None
            assert!(current.is_none());
            assert!(peak.is_none());

            // Test forced refresh
            stats.refresh_memory_stats();
            let (current2, peak2, available2) = stats.get_memory_usage();

            // Should remain unavailable
            assert!(!available2);
            assert!(current2.is_none());
            assert!(peak2.is_none());
        }
    }

    #[test]
    fn test_memory_caching() {
        let stats = ServerStats::new();

        // First call should set the cache
        let (current1, peak1, available1) = stats.get_memory_usage();

        // Immediate second call should use cache (values should be identical)
        let (current2, peak2, available2) = stats.get_memory_usage();
        assert_eq!(current1, current2);
        assert_eq!(peak1, peak2);
        assert_eq!(available1, available2);

        // Verify cache timestamp was set
        let last_check = stats.last_memory_check.lock().unwrap();
        assert!(last_check.is_some());
    }

    #[test]
    fn test_memory_unavailable_scenario() {
        let stats = ServerStats::new();

        // First check the actual system state
        let (initial_current, initial_peak, initial_available) = stats.get_memory_usage();

        if initial_available {
            // System has memory tracking available, so let's manually simulate unavailable state
            // Set memory as unavailable for testing
            stats.memory_available.store(false, Ordering::Relaxed);
            if let Ok(mut mem) = stats.process_memory_bytes.lock() {
                *mem = None;
            }
            if let Ok(mut peak) = stats.peak_memory_bytes.lock() {
                *peak = None;
            }

            // Now the get_memory_usage should return the cached unavailable state
            // without refreshing (since we didn't change the timestamp)
            let (current, peak, available) = (
                *stats.process_memory_bytes.lock().unwrap(),
                *stats.peak_memory_bytes.lock().unwrap(),
                stats.memory_available.load(Ordering::Relaxed),
            );

            // Should indicate unavailable memory based on what we set
            assert!(!available);
            assert!(current.is_none());
            assert!(peak.is_none());
        } else {
            // System doesn't have memory tracking, verify the behavior
            assert!(!initial_available);
            assert!(initial_current.is_none());
            assert!(initial_peak.is_none());

            // Test refresh maintains unavailable state
            stats.refresh_memory_stats();
            let (current2, peak2, available2) = stats.get_memory_usage();

            // Should remain unavailable if system doesn't support it
            assert!(!available2);
            assert!(current2.is_none());
            assert!(peak2.is_none());
        }
    }

    #[test]
    #[ignore]
    fn perf_rate_limiter_sharded_unique_ips() {
        let limiter = RateLimiter::new(10_000, 32);
        let workers = 8u32;
        let ips_per_worker = 5_000u32;
        let start = Instant::now();

        thread::scope(|scope| {
            for worker in 0..workers {
                let limiter = limiter.clone();
                scope.spawn(move || {
                    for i in 1..=ips_per_worker {
                        let ordinal = worker * ips_per_worker + (i - 1);
                        let second = ((ordinal >> 16) & 0xff) as u8;
                        let third = ((ordinal >> 8) & 0xff) as u8;
                        let fourth = (ordinal & 0xff) as u8;
                        let ip = IpAddr::V4(Ipv4Addr::new(10, second, third, fourth));
                        assert!(limiter.check_rate_limit(ip));
                    }
                });
            }
        });

        let elapsed_ms = start.elapsed().as_millis();
        let (entries, memory_bytes) = limiter.get_memory_stats();
        println!(
            "PERF rate_limiter_sharded workers={} total_ops={} elapsed_ms={} entries={} memory_bytes={}",
            workers,
            workers as u64 * ips_per_worker as u64,
            elapsed_ms,
            entries,
            memory_bytes
        );
        assert!(entries > 0);
    }
}

/// Upload statistics structure for reporting
#[derive(Debug, Clone)]
pub struct UploadStats {
    pub total_uploads: u64,
    pub successful_uploads: u64,
    pub failed_uploads: u64,
    pub files_uploaded: u64,
    pub upload_bytes: u64,
    pub average_upload_size: u64,
    pub largest_upload: u64,
    pub concurrent_uploads: u64,
    pub average_processing_time: f64,
    pub success_rate: f64,
}

/// Load TLS certificates from a PEM file
fn load_tls_certs(
    path: &std::path::Path,
) -> Result<Vec<rustls::pki_types::CertificateDer<'static>>, AppError> {
    let file = std::fs::File::open(path).map_err(|e| {
        AppError::InvalidConfiguration(format!(
            "Failed to open SSL certificate file {}: {}",
            path.display(),
            e
        ))
    })?;
    let mut reader = BufReader::new(file);
    let certs: Vec<rustls::pki_types::CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            AppError::InvalidConfiguration(format!(
                "Failed to parse SSL certificate file {}: {}",
                path.display(),
                e
            ))
        })?;
    if certs.is_empty() {
        return Err(AppError::InvalidConfiguration(format!(
            "No certificates found in {}",
            path.display()
        )));
    }
    info!(
        "Loaded {} certificate(s) from {}",
        certs.len(),
        path.display()
    );
    Ok(certs)
}

/// Load TLS private key from a PEM file
fn load_tls_key(
    path: &std::path::Path,
) -> Result<rustls::pki_types::PrivateKeyDer<'static>, AppError> {
    let file = std::fs::File::open(path).map_err(|e| {
        AppError::InvalidConfiguration(format!(
            "Failed to open SSL key file {}: {}",
            path.display(),
            e
        ))
    })?;
    let mut reader = BufReader::new(file);
    let key = rustls_pemfile::private_key(&mut reader)
        .map_err(|e| {
            AppError::InvalidConfiguration(format!(
                "Failed to parse SSL key file {}: {}",
                path.display(),
                e
            ))
        })?
        .ok_or_else(|| {
            AppError::InvalidConfiguration(format!("No private key found in {}", path.display()))
        })?;
    info!("Loaded private key from {}", path.display());
    Ok(key)
}

/// Build TLS server configuration from certificate and key paths
fn build_tls_config(
    cert_path: &std::path::Path,
    key_path: &std::path::Path,
) -> Result<Arc<ServerConfig>, AppError> {
    let certs = load_tls_certs(cert_path)?;
    let key = load_tls_key(key_path)?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| {
            AppError::InvalidConfiguration(format!("Failed to build TLS configuration: {}", e))
        })?;

    info!("TLS configuration built successfully");
    Ok(Arc::new(config))
}

/// Run server with new configuration system
pub fn run_server_with_config(config: Config) -> Result<(), AppError> {
    // Convert Config back to Cli for compatibility with existing code
    // This is a transitional approach - eventually we could refactor to use Config throughout
    let cli = Cli {
        directory: config.directory,
        listen: Some(config.listen),
        port: Some(config.port),
        allowed_extensions: Some(config.allowed_extensions.join(",")),
        threads: Some(config.threads),
        chunk_size: Some(config.chunk_size),
        verbose: Some(config.verbose),
        detailed_logging: Some(config.detailed_logging),
        username: config.username,
        password: config.password,
        enable_upload: Some(config.enable_upload),
        max_upload_size: Some(config.max_upload_size / (1024 * 1024)), // Convert bytes back to MB
        enable_webdav: Some(config.enable_webdav),
        disable_rate_limit: Some(config.disable_rate_limit),
        config_file: None, // Not needed for server execution
        log_dir: config.log_dir,
        ssl_cert: config.ssl_cert,
        ssl_key: config.ssl_key,
        base_path: if config.base_path.is_empty() {
            None
        } else {
            Some(config.base_path)
        },
    };

    run_server(cli, None, None)
}

pub fn run_server(
    cli: Cli,
    shutdown_rx: Option<mpsc::Receiver<()>>,
    addr_tx: Option<mpsc::Sender<SocketAddr>>,
) -> Result<(), AppError> {
    let worker_threads = cli.threads.unwrap_or(8);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .worker_threads(worker_threads)
        .max_blocking_threads(worker_threads.saturating_mul(8).max(64))
        .build()
        .map_err(|e| AppError::InternalServerError(e.to_string()))?;

    runtime.block_on(run_server_async(cli, shutdown_rx, addr_tx))
}

async fn run_server_async(
    cli: Cli,
    shutdown_rx: Option<mpsc::Receiver<()>>,
    addr_tx: Option<mpsc::Sender<SocketAddr>>,
) -> Result<(), AppError> {
    debug!(
        "Starting server with configuration: verbose={:?}, detailed_logging={:?}",
        cli.verbose, cli.detailed_logging
    );

    let base_dir = Arc::new(cli.directory.canonicalize()?);
    trace!("Base directory resolved to: {:?}", base_dir);

    if !base_dir.is_dir() {
        return Err(AppError::DirectoryNotFound(
            cli.directory.to_string_lossy().into_owned(),
        ));
    }

    crate::search::initialize_search(base_dir.as_ref().clone());

    let allowed_extensions = Arc::new(
        cli.allowed_extensions
            .as_ref()
            .unwrap_or(&"*".to_string())
            .split(',')
            .map(|ext| Pattern::new(ext.trim()))
            .collect::<Result<Vec<Pattern>, _>>()?,
    );

    let bind_address = format!(
        "{}:{}",
        cli.listen.as_ref().unwrap_or(&"127.0.0.1".to_string()),
        cli.port.unwrap_or(8080)
    );

    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    let local_addr = listener.local_addr()?;

    let tls_config: Option<Arc<ServerConfig>> =
        if let (Some(cert_path), Some(key_path)) = (&cli.ssl_cert, &cli.ssl_key) {
            Some(build_tls_config(cert_path, key_path)?)
        } else {
            None
        };
    let tls_acceptor = tls_config
        .as_ref()
        .map(|cfg| tokio_rustls::TlsAcceptor::from(cfg.clone()));
    let is_https = tls_acceptor.is_some();

    let webdav_enabled = cli.enable_webdav.unwrap_or(false);
    let disable_rate_limit_requested = cli.disable_rate_limit.unwrap_or(false);
    let rate_limit_disabled = webdav_enabled && disable_rate_limit_requested;
    if disable_rate_limit_requested && !webdav_enabled {
        warn!(
            "Ignoring --disable-rate-limit because WebDAV is disabled. Enable WebDAV for it to take effect."
        );
    }
    if rate_limit_disabled {
        info!("WebDAV rate limiting is disabled by configuration.");
    }
    let (rate_limit_per_minute, concurrent_per_ip) = if webdav_enabled {
        (3500, 128)
    } else {
        (120, 10)
    };
    let rate_limiter = Arc::new(RateLimiter::new(rate_limit_per_minute, concurrent_per_ip));
    let stats = Arc::new(ServerStats::new());

    if let Some(tx) = addr_tx
        && tx.send(local_addr).is_err()
    {
        return Err(AppError::InternalServerError(
            "Failed to send server address to test thread".to_string(),
        ));
    }

    let protocol = if is_https { "https" } else { "http" };
    info!(
        "🚀 Server listening on {}://{} for directory '{}' (allowed extensions: {:?})",
        protocol,
        local_addr,
        base_dir.display(),
        allowed_extensions
    );

    let username = Arc::new(cli.username.clone());
    let password = Arc::new(cli.password.clone());
    let chunk_size = cli.chunk_size.unwrap_or(1024);
    let cli_arc = Arc::new(cli);

    // Initialize the global base path for reverse proxy sub-path support
    crate::templates::init_base_path(cli_arc.base_path.clone().unwrap_or_default());

    let mut router = Router::new();
    if cli_arc.username.is_some() && cli_arc.password.is_some() {
        crate::templates::AUTH_ENABLED.store(true, std::sync::atomic::Ordering::SeqCst);
        router.add_middleware(Box::new(AuthMiddleware::new(
            cli_arc.username.clone(),
            cli_arc.password.clone(),
        )));
    }
    register_internal_routes(
        &mut router,
        Some(cli_arc.clone()),
        Some(stats.clone()),
        Some(base_dir.clone()),
    );
    let shared_router = Arc::new(router);

    tokio::spawn({
        let rate_limiter = rate_limiter.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                rate_limiter.cleanup_old_entries();
            }
        }
    });

    tokio::spawn({
        let stats_reporter = stats.clone();
        let rate_limiter_monitor = rate_limiter.clone();
        async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                let (total, successful, errors, bytes, uptime) = stats_reporter.get_stats();
                let upload_stats = stats_reporter.get_upload_stats();
                let (current_memory, peak_memory, memory_available) =
                    stats_reporter.get_memory_usage();
                let memory_pressure =
                    stats_reporter.check_memory_pressure(Some(&rate_limiter_monitor));

                info!(
                    "📊 Request Stats: {} total ({} successful, {} errors), {:.2} MB served, uptime: {}s",
                    total,
                    successful,
                    errors,
                    bytes as f64 / 1024.0 / 1024.0,
                    uptime.as_secs()
                );

                if memory_available {
                    let current_mb = current_memory.unwrap_or(0) as f64 / 1024.0 / 1024.0;
                    let peak_mb = peak_memory.unwrap_or(0) as f64 / 1024.0 / 1024.0;
                    let pressure_indicator = if memory_pressure {
                        " ⚠️ PRESSURE"
                    } else {
                        ""
                    };
                    info!(
                        "🧠 Memory Stats: {current_mb:.2} MB current, {peak_mb:.2} MB peak{pressure_indicator}"
                    );
                }

                if upload_stats.total_uploads > 0 {
                    info!(
                        "📤 Upload Stats: {} uploads ({:.1}% success), {} files, {:.2} MB uploaded, avg: {:.2} MB/file, {:.0}ms/upload, {} concurrent",
                        upload_stats.total_uploads,
                        upload_stats.success_rate,
                        upload_stats.files_uploaded,
                        upload_stats.upload_bytes as f64 / 1024.0 / 1024.0,
                        upload_stats.average_upload_size as f64 / 1024.0 / 1024.0,
                        upload_stats.average_processing_time,
                        upload_stats.concurrent_uploads
                    );
                }
            }
        }
    });

    let mut shutdown_task = shutdown_rx.map(|rx| {
        tokio::task::spawn_blocking(move || {
            let _ = rx.recv();
        })
    });

    loop {
        if let Some(ref mut shutdown_task) = shutdown_task {
            tokio::select! {
                _ = shutdown_task => {
                    break;
                }
                res = listener.accept() => {
                    let (stream, peer_addr) = res?;
                    handle_connection(
                        stream,
                        peer_addr,
                        base_dir.clone(),
                        allowed_extensions.clone(),
                        username.clone(),
                        password.clone(),
                        chunk_size,
                        rate_limiter.clone(),
                        rate_limit_disabled,
                        stats.clone(),
                        cli_arc.clone(),
                        shared_router.clone(),
                        tls_acceptor.clone(),
                    );
                }
            }
        } else {
            let (stream, peer_addr) = listener.accept().await?;
            handle_connection(
                stream,
                peer_addr,
                base_dir.clone(),
                allowed_extensions.clone(),
                username.clone(),
                password.clone(),
                chunk_size,
                rate_limiter.clone(),
                rate_limit_disabled,
                stats.clone(),
                cli_arc.clone(),
                shared_router.clone(),
                tls_acceptor.clone(),
            );
        }
    }

    info!("✅ Server shut down gracefully.");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_connection(
    stream: TokioTcpStream,
    peer_addr: SocketAddr,
    base_dir: Arc<std::path::PathBuf>,
    allowed_extensions: Arc<Vec<glob::Pattern>>,
    username: Arc<Option<String>>,
    password: Arc<Option<String>>,
    chunk_size: usize,
    rate_limiter: Arc<RateLimiter>,
    rate_limit_disabled: bool,
    stats: Arc<ServerStats>,
    cli_config: Arc<Cli>,
    router: Arc<Router>,
    tls_acceptor: Option<tokio_rustls::TlsAcceptor>,
) {
    let client_ip = peer_addr.ip();
    if !rate_limit_disabled && !rate_limiter.check_rate_limit(client_ip) {
        return;
    }

    tokio::spawn(async move {
        let result = if let Some(acceptor) = tls_acceptor {
            match acceptor.accept(stream).await {
                Ok(tls_stream) => {
                    crate::http::handle_client_async(
                        tls_stream,
                        peer_addr,
                        base_dir,
                        allowed_extensions,
                        username,
                        password,
                        chunk_size,
                        Some(cli_config),
                        Some(stats.clone()),
                        router,
                    )
                    .await;
                    Ok(())
                }
                Err(_) => Err(()),
            }
        } else {
            crate::http::handle_client_async(
                stream,
                peer_addr,
                base_dir,
                allowed_extensions,
                username,
                password,
                chunk_size,
                Some(cli_config),
                Some(stats.clone()),
                router,
            )
            .await;
            Ok(())
        };

        if !rate_limit_disabled {
            rate_limiter.release_connection(client_ip);
        }
        let _ = result;
    });
}
