use crate::cli::Cli;
use crate::config::Config;
use crate::error::AppError;
use crate::handlers::register_internal_routes;
use crate::http::handle_client;
use crate::middleware::AuthMiddleware;
use crate::router::Router;
use glob::Pattern;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::mem;

/// Rate limiter for basic DoS protection
#[derive(Clone)]
pub struct RateLimiter {
    connections: Arc<Mutex<HashMap<IpAddr, ConnectionInfo>>>,
    max_requests_per_minute: u32,
    max_concurrent_per_ip: u32,
}

#[derive(Debug)]
struct ConnectionInfo {
    request_count: u32,
    last_reset: Instant,
    active_connections: u32,
}

impl RateLimiter {
    pub fn new(max_requests_per_minute: u32, max_concurrent_per_ip: u32) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            max_requests_per_minute,
            max_concurrent_per_ip,
        }
    }

    pub fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let mut connections = self.connections.lock().unwrap();
        let now = Instant::now();

        let conn_info = connections.entry(ip).or_insert(ConnectionInfo {
            request_count: 0,
            last_reset: now,
            active_connections: 0,
        });

        // Reset counter if more than a minute has passed
        if now.duration_since(conn_info.last_reset) >= Duration::from_secs(60) {
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
        true
    }

    pub fn release_connection(&self, ip: IpAddr) {
        if let Ok(mut connections) = self.connections.lock() {
            if let Some(conn_info) = connections.get_mut(&ip) {
                conn_info.active_connections = conn_info.active_connections.saturating_sub(1);
            }
        }
    }

    pub fn cleanup_old_entries(&self) {
        let mut connections = self.connections.lock().unwrap();
        let now = Instant::now();

        connections.retain(|_, info| {
            now.duration_since(info.last_reset) < Duration::from_secs(300) // Keep for 5 minutes
        });
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
#[derive(Default, Clone)]
pub struct ServerStats {
    // Request statistics
    pub total_requests: Arc<Mutex<u64>>,
    pub successful_requests: Arc<Mutex<u64>>,
    pub error_requests: Arc<Mutex<u64>>,
    pub bytes_served: Arc<Mutex<u64>>,
    pub start_time: Arc<Mutex<Option<Instant>>>,

    // Upload statistics
    pub total_uploads: Arc<Mutex<u64>>,
    pub successful_uploads: Arc<Mutex<u64>>,
    pub failed_uploads: Arc<Mutex<u64>>,
    pub files_uploaded: Arc<Mutex<u64>>,
    pub upload_bytes: Arc<Mutex<u64>>,
    pub largest_upload: Arc<Mutex<u64>>,
    pub concurrent_uploads: Arc<Mutex<u64>>,
    pub upload_processing_times: Arc<Mutex<Vec<u64>>>,

    // Memory statistics
    pub process_memory_bytes: Arc<Mutex<Option<u64>>>,
    pub peak_memory_bytes: Arc<Mutex<Option<u64>>>,
    pub last_memory_check: Arc<Mutex<Option<Instant>>>,
    pub memory_available: Arc<Mutex<bool>>,
}

impl ServerStats {
    pub fn new() -> Self {
        Self {
            // Request statistics
            total_requests: Arc::new(Mutex::new(0)),
            successful_requests: Arc::new(Mutex::new(0)),
            error_requests: Arc::new(Mutex::new(0)),
            bytes_served: Arc::new(Mutex::new(0)),
            start_time: Arc::new(Mutex::new(Some(Instant::now()))),

            // Upload statistics
            total_uploads: Arc::new(Mutex::new(0)),
            successful_uploads: Arc::new(Mutex::new(0)),
            failed_uploads: Arc::new(Mutex::new(0)),
            files_uploaded: Arc::new(Mutex::new(0)),
            upload_bytes: Arc::new(Mutex::new(0)),
            largest_upload: Arc::new(Mutex::new(0)),
            concurrent_uploads: Arc::new(Mutex::new(0)),
            upload_processing_times: Arc::new(Mutex::new(Vec::new())),

            // Memory statistics
            process_memory_bytes: Arc::new(Mutex::new(None)),
            peak_memory_bytes: Arc::new(Mutex::new(None)),
            last_memory_check: Arc::new(Mutex::new(None)),
            memory_available: Arc::new(Mutex::new(true)), // Assume available until proven otherwise
        }
    }

    pub fn record_request(&self, success: bool, bytes: u64) {
        if let Ok(mut total) = self.total_requests.lock() {
            *total += 1;
        }

        if success {
            if let Ok(mut successful) = self.successful_requests.lock() {
                *successful += 1;
            }
        } else if let Ok(mut errors) = self.error_requests.lock() {
            *errors += 1;
        }

        if let Ok(mut total_bytes) = self.bytes_served.lock() {
            *total_bytes += bytes;
        }
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64, Duration) {
        let total = *self
            .total_requests
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let successful = *self
            .successful_requests
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let errors = *self
            .error_requests
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let bytes = *self
            .bytes_served
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let uptime = self
            .start_time
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"))
            .map(|start| start.elapsed())
            .unwrap_or_default();

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
        // Increment total uploads
        if let Ok(mut total) = self.total_uploads.lock() {
            *total += 1;
        }

        // Track success/failure
        if success {
            if let Ok(mut successful) = self.successful_uploads.lock() {
                *successful += 1;
            }
        } else if let Ok(mut failed) = self.failed_uploads.lock() {
            *failed += 1;
        }

        // Only record additional metrics for successful uploads
        if success {
            // Record number of files uploaded
            if let Ok(mut files) = self.files_uploaded.lock() {
                *files += files_count;
            }

            // Record total bytes uploaded
            if let Ok(mut bytes) = self.upload_bytes.lock() {
                *bytes += upload_bytes;
            }

            // Update largest upload if this is bigger
            if let Ok(mut largest) = self.largest_upload.lock() {
                if largest_file > *largest {
                    *largest = largest_file;
                }
            }

            // Record processing time (keep last 100 entries for average calculation)
            if let Ok(mut times) = self.upload_processing_times.lock() {
                times.push(processing_time_ms);
                if times.len() > 100 {
                    times.remove(0);
                }
            }
        }
    }

    /// Track concurrent upload start
    pub fn start_upload(&self) {
        if let Ok(mut concurrent) = self.concurrent_uploads.lock() {
            *concurrent += 1;
        }
    }

    /// Track concurrent upload completion
    pub fn finish_upload(&self) {
        if let Ok(mut concurrent) = self.concurrent_uploads.lock() {
            *concurrent = concurrent.saturating_sub(1);
        }
    }

    /// Get upload statistics
    pub fn get_upload_stats(&self) -> UploadStats {
        let total_uploads = *self
            .total_uploads
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let successful_uploads = *self
            .successful_uploads
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let failed_uploads = *self
            .failed_uploads
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let files_uploaded = *self
            .files_uploaded
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let upload_bytes = *self
            .upload_bytes
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let largest_upload = *self
            .largest_upload
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        let concurrent_uploads = *self
            .concurrent_uploads
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));

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
                Some(last) => now.duration_since(last) >= Duration::from_secs(5),
                None => true,
            }
        };

        if should_refresh {
            let current_memory_opt = get_process_memory_bytes();

            // Update availability status
            let is_available = current_memory_opt.is_some();
            if let Ok(mut available) = self.memory_available.lock() {
                if !*available && is_available {
                    info!("Memory tracking is now available");
                } else if *available && !is_available {
                    info!("Memory tracking is no longer available");
                }
                *available = is_available;
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
        let available = *self
            .memory_available
            .lock()
            .unwrap_or_else(|_| panic!("Stats lock poisoned"));
        (current, peak, available)
    }

    /// Force refresh memory statistics (bypasses cache)
    pub fn refresh_memory_stats(&self) {
        let current_memory_opt = get_process_memory_bytes();

        // Update availability status
        let is_available = current_memory_opt.is_some();
        if let Ok(mut available) = self.memory_available.lock() {
            *available = is_available;
        }

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
                    if line.starts_with("VmRSS:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return Some(kb * 1024); // Convert KB to bytes
                            }
                        }
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

        extern "C" {
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

        extern "system" {
            fn GetCurrentProcess() -> *mut c_void;
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
            if let Ok(mut available) = stats.memory_available.lock() {
                *available = false;
            }
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
                *stats.memory_available.lock().unwrap(),
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

/// Simple native thread pool implementation
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        if let Some(ref sender) = self.sender {
            if sender.send(job).is_err() {
                warn!("Failed to send job to thread pool");
            }
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                if thread.join().is_err() {
                    warn!("Worker thread {} panicked", worker.id);
                }
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    job();
                }
                Err(_) => {
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
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
        config_file: None, // Not needed for server execution
    };

    run_server(cli, None, None)
}

pub fn run_server(
    cli: Cli,
    shutdown_rx: Option<mpsc::Receiver<()>>,
    addr_tx: Option<mpsc::Sender<SocketAddr>>,
) -> Result<(), AppError> {
    let base_dir = Arc::new(cli.directory.canonicalize()?);

    if !base_dir.is_dir() {
        return Err(AppError::DirectoryNotFound(
            cli.directory.to_string_lossy().into_owned(),
        ));
    }

    // Initialize the search subsystem with caching and indexing
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
    let listener = TcpListener::bind(&bind_address)?;
    let local_addr = listener.local_addr()?;
    listener.set_nonblocking(true)?;

    // Initialize security and monitoring systems
    let rate_limiter = Arc::new(RateLimiter::new(120, 10)); // 120 req/min, 10 concurrent per IP
    let stats = Arc::new(ServerStats::new());

    if let Some(tx) = addr_tx {
        if tx.send(local_addr).is_err() {
            return Err(AppError::InternalServerError(
                "Failed to send server address to test thread".to_string(),
            ));
        }
    }

    info!(
        "🚀 Server listening on {} for directory '{}' (allowed extensions: {:?})",
        local_addr,
        base_dir.display(),
        allowed_extensions
    );
    info!("⚡ Security: Rate limiting enabled (120 req/min, 10 concurrent per IP)");
    info!("📊 Monitoring: Statistics collection enabled");

    let pool = ThreadPool::new(cli.threads.unwrap_or(8));
    let username = Arc::new(cli.username.clone());
    let password = Arc::new(cli.password.clone());
    let cli_arc = Arc::new(cli);

    // Build shared internal router once (with middleware)
    let mut router = Router::new();
    if cli_arc.username.is_some() && cli_arc.password.is_some() {
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

    // Start background cleanup task for rate limiter
    let rate_limiter_cleanup = rate_limiter.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(300)); // Cleanup every 5 minutes
            rate_limiter_cleanup.cleanup_old_entries();
        }
    });

    // Start background stats reporting
    let stats_reporter = stats.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(300)); // Report every 5 minutes
            let (total, successful, errors, bytes, uptime) = stats_reporter.get_stats();
            let upload_stats = stats_reporter.get_upload_stats();
            let (current_memory, peak_memory, memory_available) = stats_reporter.get_memory_usage();

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
                info!("🧠 Memory Stats: {current_mb:.2} MB current, {peak_mb:.2} MB peak");
            } else {
                debug!("🧠 Memory Stats: unavailable");
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
    });

    'server_loop: loop {
        if let Some(ref rx) = shutdown_rx {
            if rx.try_recv().is_ok() {
                info!("🛑 Shutdown signal received. Shutting down gracefully.");
                break 'server_loop;
            }
        }

        match listener.accept() {
            Ok((stream, peer_addr)) => {
                let client_ip = peer_addr.ip();

                // Check rate limits
                if !rate_limiter.check_rate_limit(client_ip) {
                    warn!("🚫 Connection from {client_ip} rejected due to rate limiting");
                    drop(stream); // Close connection immediately
                    continue;
                }

                // Ensure the accepted stream is in blocking mode
                if let Err(e) = stream.set_nonblocking(false) {
                    error!("Failed to set stream to blocking mode: {e}");
                    rate_limiter.release_connection(client_ip);
                    continue;
                }

                let (
                    base_dir,
                    allowed_extensions,
                    username,
                    password,
                    chunk_size,
                    rate_limiter,
                    stats,
                    cli_ref,
                    router,
                ) = (
                    base_dir.clone(),
                    allowed_extensions.clone(),
                    username.clone(),
                    password.clone(),
                    cli_arc.chunk_size.unwrap_or(1024),
                    rate_limiter.clone(),
                    stats.clone(),
                    cli_arc.clone(),
                    shared_router.clone(),
                );

                pool.execute(move || {
                    let result = handle_client_with_stats(
                        stream,
                        peer_addr,
                        &base_dir,
                        &allowed_extensions,
                        &username,
                        &password,
                        chunk_size,
                        &stats,
                        Some(cli_ref.as_ref()),
                        &router,
                    );

                    // Release rate limit connection
                    rate_limiter.release_connection(client_ip);

                    // Log any errors
                    if let Err(e) = result {
                        warn!("⚠️  Client handling error: {e}");
                    }
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                error!("❌ Error accepting connection: {e}");
            }
        }
    }

    // Final stats report
    let (total, successful, errors, bytes, uptime) = stats.get_stats();
    let upload_stats = stats.get_upload_stats();
    let (current_memory, peak_memory, memory_available) = stats.get_memory_usage();

    info!(
        "📊 Final Request Stats: {} total ({} successful, {} errors), {:.2} MB served, uptime: {}s",
        total,
        successful,
        errors,
        bytes as f64 / 1024.0 / 1024.0,
        uptime.as_secs()
    );

    if memory_available {
        let current_mb = current_memory.unwrap_or(0) as f64 / 1024.0 / 1024.0;
        let peak_mb = peak_memory.unwrap_or(0) as f64 / 1024.0 / 1024.0;
        info!("🧠 Final Memory Stats: {current_mb:.2} MB current, {peak_mb:.2} MB peak");
    } else {
        info!("🧠 Final Memory Stats: unavailable");
    }

    if upload_stats.total_uploads > 0 {
        info!(
            "📤 Final Upload Stats: {} uploads ({:.1}% success), {} files, {:.2} MB uploaded, largest: {:.2} MB",
            upload_stats.total_uploads,
            upload_stats.success_rate,
            upload_stats.files_uploaded,
            upload_stats.upload_bytes as f64 / 1024.0 / 1024.0,
            upload_stats.largest_upload as f64 / 1024.0 / 1024.0
        );
    }

    info!("✅ Server shut down gracefully.");
    Ok(())
}

/// Enhanced client handler with statistics tracking
#[allow(clippy::too_many_arguments)]
fn handle_client_with_stats(
    stream: std::net::TcpStream,
    peer_addr: SocketAddr,
    base_dir: &Arc<std::path::PathBuf>,
    allowed_extensions: &Arc<Vec<glob::Pattern>>,
    username: &Arc<Option<String>>,
    password: &Arc<Option<String>>,
    chunk_size: usize,
    stats: &ServerStats,
    cli_config: Option<&crate::cli::Cli>,
    router: &Arc<crate::router::Router>,
) -> Result<(), AppError> {
    let start = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_client(
            stream,
            base_dir,
            allowed_extensions,
            username,
            password,
            chunk_size,
            cli_config,
            Some(stats),
            router,
        );
    }));

    let success = result.is_ok();
    let processing_time = start.elapsed();

    // On panic, record failure (normal success/failure & bytes recorded in handle_client)
    if !success {
        stats.record_request(false, 0);
    }

    if processing_time > Duration::from_millis(1000) {
        warn!(
            "⏱️  Slow request from {}: {}ms",
            peer_addr.ip(),
            processing_time.as_millis()
        );
    }

    if result.is_err() {
        return Err(AppError::InternalServerError(
            "Client handler panicked".to_string(),
        ));
    }

    Ok(())
}
