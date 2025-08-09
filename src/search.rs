//! Optimized search module with caching, indexing, and parallel processing

use crate::error::AppError;
use log::{debug, info, warn};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Represents a search result file with relevance scoring
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub path: String,
    pub size: String,
    pub file_type: String,
    pub score: f32,
    pub last_modified: Option<u64>,
}

/// Search parameters
pub struct SearchParams {
    pub query: String,
    pub path: String,
    pub limit: usize,
    pub offset: usize,
    pub case_sensitive: bool,
}

/// LRU Cache for search results
pub struct SearchCache {
    cache: HashMap<String, CachedSearchResult>,
    order: VecDeque<String>,
    max_size: usize,
    stats: CacheStats,
}

#[derive(Clone)]
struct CachedSearchResult {
    results: Vec<SearchResult>,
    timestamp: Instant,
    hit_count: u32,
}

#[derive(Default)]
struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl SearchCache {
    pub fn new(max_size: usize) -> Self {
        SearchCache {
            cache: HashMap::new(),
            order: VecDeque::new(),
            max_size,
            stats: CacheStats::default(),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<Vec<SearchResult>> {
        if let Some(cached) = self.cache.get_mut(key) {
            // Check if cache is still valid (10 seconds TTL for better performance)
            if cached.timestamp.elapsed().as_secs() < 10 {
                cached.hit_count += 1;
                self.stats.hits += 1;

                // Move to front of LRU queue
                if let Some(pos) = self.order.iter().position(|k| k == key) {
                    self.order.remove(pos);
                }
                self.order.push_front(key.to_string());

                debug!("Cache hit for query: {} (hits: {})", key, cached.hit_count);
                return Some(cached.results.clone());
            } else {
                // Cache expired, remove it
                self.cache.remove(key);
                if let Some(pos) = self.order.iter().position(|k| k == key) {
                    self.order.remove(pos);
                }
                debug!("Cache expired for query: {key}");
            }
        }
        self.stats.misses += 1;
        None
    }

    pub fn put(&mut self, key: String, results: Vec<SearchResult>) {
        // Evict LRU item if cache is full
        while self.cache.len() >= self.max_size {
            if let Some(lru_key) = self.order.pop_back() {
                self.cache.remove(&lru_key);
                self.stats.evictions += 1;
                debug!("Evicted cache entry: {lru_key}");
            }
        }

        let result_count = results.len();
        self.cache.insert(
            key.clone(),
            CachedSearchResult {
                results,
                timestamp: Instant::now(),
                hit_count: 0,
            },
        );
        self.order.push_front(key.clone());
        debug!("Cached {result_count} results for query: {key}");
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.order.clear();
        info!("Search cache cleared");
    }

    pub fn get_stats(&self) -> String {
        let total = self.stats.hits + self.stats.misses;
        let hit_rate = if total > 0 {
            (self.stats.hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        format!(
            "Cache stats - Hits: {}, Misses: {}, Hit rate: {:.1}%, Evictions: {}, Size: {}/{}",
            self.stats.hits,
            self.stats.misses,
            hit_rate,
            self.stats.evictions,
            self.cache.len(),
            self.max_size
        )
    }
}

/// Directory index for fast searching
pub struct DirectoryIndex {
    entries: Vec<IndexEntry>,
    last_update: Instant,
    base_dir: PathBuf,
}

#[derive(Clone)]
struct IndexEntry {
    name: String,
    path: PathBuf,
    size: u64,
    is_dir: bool,
    modified: SystemTime,
    name_lower: String, // Pre-computed lowercase for faster searching
}

impl DirectoryIndex {
    pub fn new(base_dir: PathBuf) -> Self {
        DirectoryIndex {
            entries: Vec::new(),
            last_update: Instant::now(),
            base_dir,
        }
    }

    /// Build or update the index if it's stale
    pub fn update_if_needed(&mut self, force: bool) -> Result<(), AppError> {
        // Update index every 30 seconds or if forced
        if !force && self.last_update.elapsed().as_secs() < 30 {
            return Ok(());
        }

        info!("Building directory index for: {:?}", self.base_dir);
        let start = Instant::now();

        let mut new_entries = Vec::new();
        Self::walk_directory_for_index(&self.base_dir.clone(), &mut new_entries, 0)?;

        self.entries = new_entries;
        self.last_update = Instant::now();

        info!(
            "Directory index built: {} entries in {:.2}s",
            self.entries.len(),
            start.elapsed().as_secs_f32()
        );

        Ok(())
    }

    fn walk_directory_for_index(
        dir: &Path,
        entries: &mut Vec<IndexEntry>,
        depth: usize,
    ) -> Result<(), AppError> {
        // Limit depth to prevent excessive recursion
        if depth > 20 {
            return Ok(());
        }

        // Stop indexing if we have too many entries (prevent memory issues)
        if entries.len() > 100_000 {
            warn!("Directory index limit reached (100k entries)");
            return Ok(());
        }

        let dir_entries =
            fs::read_dir(dir).map_err(|e| AppError::InternalServerError(e.to_string()))?;

        for entry_result in dir_entries {
            let entry = match entry_result {
                Ok(e) => e,
                Err(_) => continue,
            };

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_name_lower = file_name.to_lowercase();

            entries.push(IndexEntry {
                name: file_name.clone(),
                path: entry.path(),
                size: metadata.len(),
                is_dir: metadata.is_dir(),
                modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                name_lower: file_name_lower,
            });

            // Recursively index subdirectories
            if metadata.is_dir() {
                let _ = Self::walk_directory_for_index(&entry.path(), entries, depth + 1);
            }
        }

        Ok(())
    }

    /// Search the index for matching entries
    pub fn search(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for entry in &self.entries {
            if entry.name_lower.contains(&query_lower) {
                let score = calculate_relevance_score(&entry.name, query);

                let relative_path = entry
                    .path
                    .strip_prefix(&self.base_dir)
                    .unwrap_or(&entry.path)
                    .to_string_lossy()
                    .to_string();

                results.push(SearchResult {
                    name: entry.name.clone(),
                    path: format!("/{relative_path}"),
                    size: if entry.is_dir {
                        "-".to_string()
                    } else {
                        format_file_size(entry.size)
                    },
                    file_type: if entry.is_dir {
                        "directory".to_string()
                    } else {
                        "file".to_string()
                    },
                    score,
                    last_modified: entry
                        .modified
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|d| d.as_secs()),
                });

                if results.len() >= limit * 2 {
                    break;
                }
            }
        }

        results
    }
}

/// Global search cache instance
static SEARCH_CACHE: Mutex<Option<SearchCache>> = Mutex::new(None);

/// Global directory index instance
static DIR_INDEX: RwLock<Option<DirectoryIndex>> = RwLock::new(None);

/// Initialize the search subsystem
pub fn initialize_search(base_dir: PathBuf) {
    // Initialize cache
    {
        let mut cache = SEARCH_CACHE.lock().unwrap();
        *cache = Some(SearchCache::new(1000)); // Cache up to 1000 queries
    }

    // Initialize and build directory index in background
    {
        let mut index = DIR_INDEX.write().unwrap();
        *index = Some(DirectoryIndex::new(base_dir.clone()));
    }

    // Spawn background thread to periodically update the index
    thread::spawn(move || {
        loop {
            thread::sleep(std::time::Duration::from_secs(60)); // Update every minute

            if let Ok(mut index_guard) = DIR_INDEX.write() {
                if let Some(ref mut index) = *index_guard {
                    if let Err(e) = index.update_if_needed(true) {
                        warn!("Failed to update directory index: {e:?}");
                    }
                }
            }
        }
    });

    info!("Search subsystem initialized");
}

/// Perform an optimized search with caching and indexing
pub fn perform_search(
    base_dir: &Path,
    params: &SearchParams,
) -> Result<Vec<SearchResult>, AppError> {
    let cache_key = format!("{}:{}:{}", params.query, params.path, params.case_sensitive);

    // Check cache first
    {
        let mut cache_guard = SEARCH_CACHE.lock().unwrap();
        if let Some(ref mut cache) = *cache_guard {
            if let Some(cached_results) = cache.get(&cache_key) {
                info!("Returning cached results for query: {}", params.query);
                return Ok(cached_results);
            }
        }
    }

    // Try to use the index if available
    let mut results = {
        let index_guard = DIR_INDEX.read().unwrap();
        if let Some(ref index) = *index_guard {
            info!("Using directory index for search: {}", params.query);
            index.search(&params.query, params.limit * 2)
        } else {
            Vec::new()
        }
    };

    // If index is not available or empty, fall back to filesystem search
    if results.is_empty() {
        info!("Falling back to filesystem search for: {}", params.query);
        results = perform_parallel_search(base_dir, params)?;
    }

    // Sort by relevance score
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Limit results
    results.truncate(params.limit);

    // Cache the results
    {
        let mut cache_guard = SEARCH_CACHE.lock().unwrap();
        if let Some(ref mut cache) = *cache_guard {
            cache.put(cache_key, results.clone());
        }
    }

    Ok(results)
}

/// Perform a parallel filesystem search using multiple threads
fn perform_parallel_search(
    base_dir: &Path,
    params: &SearchParams,
) -> Result<Vec<SearchResult>, AppError> {
    let (tx, rx) = mpsc::channel();
    let query = Arc::new(params.query.clone());
    let base_dir = Arc::new(base_dir.to_path_buf());
    let num_threads = 4; // Use 4 worker threads for parallel searching

    // Determine search root
    let search_root = if params.path == "/" {
        base_dir.as_ref().clone()
    } else {
        let relative_path = PathBuf::from(params.path.strip_prefix('/').unwrap_or(&params.path));
        base_dir.join(relative_path)
    };

    // Get initial directories to search
    let mut dirs_to_search = vec![search_root];
    let mut initial_dirs = Vec::new();

    // Expand to first level of subdirectories for better parallelization
    if let Ok(entries) = fs::read_dir(&dirs_to_search[0]) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                initial_dirs.push(entry.path());
            }
        }
    }

    if !initial_dirs.is_empty() {
        dirs_to_search = initial_dirs;
    }

    // Distribute directories among threads
    let chunk_size = (dirs_to_search.len() / num_threads).max(1);
    let chunks: Vec<_> = dirs_to_search
        .chunks(chunk_size)
        .map(|c| c.to_vec())
        .collect();

    let handles: Vec<_> = chunks
        .into_iter()
        .map(|chunk| {
            let tx = tx.clone();
            let query = Arc::clone(&query);
            let base_dir = Arc::clone(&base_dir);

            thread::spawn(move || {
                for dir in chunk {
                    search_directory_recursive(&dir, &query, &base_dir, &tx, 0);
                }
            })
        })
        .collect();

    // Drop the original sender so the channel closes when all threads finish
    drop(tx);

    // Collect results from all threads
    let mut results = Vec::new();
    for result in rx {
        results.push(result);
        if results.len() >= params.limit * 2 {
            break;
        }
    }

    // Wait for all threads to complete
    for handle in handles {
        let _ = handle.join();
    }

    Ok(results)
}

/// Recursively search a directory
fn search_directory_recursive(
    dir: &Path,
    query: &str,
    base_dir: &Path,
    tx: &mpsc::Sender<SearchResult>,
    depth: usize,
) {
    if depth > 10 {
        return; // Limit recursion depth
    }

    let query_lower = query.to_lowercase();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let file_name_lower = file_name.to_lowercase();

            if file_name_lower.contains(&query_lower) {
                if let Ok(metadata) = entry.metadata() {
                    let relative_path = entry
                        .path()
                        .strip_prefix(base_dir)
                        .unwrap_or(&entry.path())
                        .to_string_lossy()
                        .to_string();

                    let result = SearchResult {
                        name: file_name.clone(),
                        path: format!("/{relative_path}"),
                        size: if metadata.is_dir() {
                            "-".to_string()
                        } else {
                            format_file_size(metadata.len())
                        },
                        file_type: if metadata.is_dir() {
                            "directory".to_string()
                        } else {
                            "file".to_string()
                        },
                        score: calculate_relevance_score(&file_name, query),
                        last_modified: metadata
                            .modified()
                            .ok()
                            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                            .map(|d| d.as_secs()),
                    };

                    let _ = tx.send(result);
                }
            }

            // Recursively search subdirectories
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                search_directory_recursive(&entry.path(), query, base_dir, tx, depth + 1);
            }
        }
    }
}

/// Calculate relevance score for search results
pub fn calculate_relevance_score(filename: &str, query: &str) -> f32 {
    let filename_lower = filename.to_lowercase();
    let query_lower = query.to_lowercase();

    let mut score = 0.0f32;

    // Exact match gets highest score
    if filename_lower == query_lower {
        score += 100.0;
    }
    // Starts with query gets high score
    else if filename_lower.starts_with(&query_lower) {
        score += 75.0;
    }
    // Ends with query (useful for extensions)
    else if filename_lower.ends_with(&query_lower) {
        score += 50.0;
    }
    // Contains query gets moderate score
    else if filename_lower.contains(&query_lower) {
        score += 25.0;

        // Bonus for word boundary matches
        if filename_lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word == query_lower)
        {
            score += 25.0;
        }
    }

    // Fuzzy match for typos
    let distance = levenshtein_distance(&filename_lower, &query_lower);
    if distance <= 2 && distance > 0 {
        score += 10.0 / (1.0 + distance as f32);
    }

    // Bonus for shorter filenames (more relevant)
    score += 5.0 / (1.0 + filename.len() as f32 * 0.1);

    // Penalty for deep nesting (prefer files closer to search root)
    let path_depth = filename.matches('/').count() as f32;
    score -= path_depth * 2.0;

    score.max(0.0)
}

/// Simple Levenshtein distance for fuzzy matching
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row = vec![0; len2 + 1];

    for i in 1..=len1 {
        curr_row[0] = i;
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            curr_row[j] = std::cmp::min(
                std::cmp::min(prev_row[j] + 1, curr_row[j - 1] + 1),
                prev_row[j - 1] + cost,
            );
        }
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[len2]
}

/// Format file size in human-readable format
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;

    if size == 0 {
        return "0 B".to_string();
    }

    let mut size_f = size as f64;
    let mut unit_index = 0;

    while size_f >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size_f /= THRESHOLD as f64;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size_f, UNITS[unit_index])
    }
}

/// Clear the search cache (useful for testing or manual cache invalidation)
pub fn clear_cache() {
    let mut cache_guard = SEARCH_CACHE.lock().unwrap();
    if let Some(ref mut cache) = *cache_guard {
        cache.clear();
    }
}

/// Get cache statistics
pub fn get_cache_stats() -> String {
    let cache_guard = SEARCH_CACHE.lock().unwrap();
    if let Some(ref cache) = *cache_guard {
        cache.get_stats()
    } else {
        "Cache not initialized".to_string()
    }
}
