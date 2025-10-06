// SPDX-License-Identifier: MIT

use crate::error::AppError;
use clap::Parser;
use log::info;
use std::path::PathBuf;

// Defines the command-line interface using clap. 🎉
// This struct represents the structure of arguments you can pass when running the server.
#[derive(Parser, Clone)]
#[command(
     author = "Harshit Jain",
     version = crate::VERSION, //  Version of our IronDrop - feels like we're shipping software! 🚢
     long_about = "This is a simple configurable download server that serves files from a directory with sophisticated error reporting and handling.\n It can be used to share files with others or to download files from a remote server.\n The server can be configured to serve only specific file extensions and can be run on a specific host and port.\n If the requested path is a directory, the server will generate an HTML page with a list of files and subdirectories in the directory.\n The server will respond with detailed error logs for various scenarios, enhancing operational visibility.\n The server can be configured to serve only specific file extensions and can be run on a specific host and port.\n The server will respond with a 403 Forbidden error if the requested file extension is not allowed.\n The server will respond with a 404 Not Found error if the requested file or directory does not exist.\n The server will respond with a 400 Bad Request error if the request is invalid.\n Follow & conribute with devlopment efforts at: git.harsh1998.dev \n Author: Harshit Jain, UI Design by: Sonu Kr. Saw\n",
     about = "A simple configurable download server with sophisticated error reporting." // Short description for `irondrop --help`.
 )]
pub struct Cli {
    /// Directory path to serve, mandatory -  This is the *only* required argument. 📂
    #[arg(short, long, required = true)]
    pub directory: PathBuf,

    /// Host address to listen on (e.g., "127.0.0.1" for local, "0.0.0.0" for everyone on the network). 👂
    #[arg(short, long)]
    pub listen: Option<String>,

    /// Port number to listen on -  Like a door number for the server to receive requests. 🚪
    #[arg(short, long)]
    pub port: Option<u16>,

    /// Allowed file extensions for download (comma-separated, supports wildcards like *.zip, *.txt) -  Security measure to only share certain file types. 🔒
    #[arg(short, long)]
    pub allowed_extensions: Option<String>,

    /// Number of threads in the thread pool -  More threads = handle more downloads at once, up to a point. 🧵🧵🧵
    #[arg(short, long)]
    pub threads: Option<usize>,

    /// Chunk size for reading files (in bytes) -  How much data we read from a file at a time when sending it. Smaller chunks are gentler on memory. 📦
    /// This is the size of the buffer used to read files in chunks
    #[arg(short, long)]
    pub chunk_size: Option<usize>,

    /// Enable verbose logging for debugging (log level: debug) -  For super detailed logs, useful when things go wrong or you're developing. 🐛
    #[arg(short, long)]
    pub verbose: Option<bool>,

    /// Enable more detailed logging (log level: info if verbose=false, debug if verbose=true) -  More logs than usual, but not *too* much. Good for general monitoring. ℹ️
    #[arg(long)]
    pub detailed_logging: Option<bool>,

    /// Username for basic authentication.
    #[arg(long)]
    pub username: Option<String>,

    /// Password for basic authentication.
    #[arg(long)]
    pub password: Option<String>,

    /// Enable direct streaming upload functionality - Allows clients to upload files using efficient direct disk streaming. Upload endpoint available at /_irondrop/upload. 📤
    #[arg(long)]
    pub enable_upload: Option<bool>,

    /// Optional upload size limit in MB - Set a maximum file size for uploads. If not specified, uploads are unlimited using direct streaming. 📏
    #[arg(long, value_parser = validate_upload_size)]
    pub max_upload_size: Option<u64>,

    /// Configuration file path - Specify a custom configuration file (INI format). If not provided, looks for irondrop.ini in current directory or ~/.config/irondrop/config.ini 🛠️
    #[arg(long, value_parser = validate_config_file)]
    pub config_file: Option<String>,

    /// Log directory path - Directory where timestamped log files will be created. If not provided, logs go to stdout 📝
    #[arg(long, value_parser = validate_log_dir)]
    pub log_dir: Option<PathBuf>,
}

/// Validate upload size (minimum 1 MB, no upper limit for direct streaming)
fn validate_upload_size(s: &str) -> Result<u64, String> {
    let size: u64 = s
        .parse()
        .map_err(|_| "Upload size must be a positive number".to_string())?;

    if size == 0 {
        return Err("Upload size must be greater than 0 MB".to_string());
    }

    // No upper limit since we're using direct disk streaming
    Ok(size)
}

/// Validate config file path exists and is readable
fn validate_config_file(s: &str) -> Result<String, String> {
    if s.is_empty() {
        return Err("Config file path cannot be empty".to_string());
    }

    let path = PathBuf::from(s);

    // Check if file exists
    if !path.exists() {
        return Err(format!("Config file does not exist: {s}"));
    }

    // Check if it's a file (not a directory)
    if !path.is_file() {
        return Err(format!("Config path is not a file: {s}"));
    }

    // Check if we can read the file
    match std::fs::File::open(&path) {
        Ok(_) => Ok(s.to_string()),
        Err(e) => Err(format!("Cannot read config file {s}: {e}")),
    }
}

impl Cli {
    /// Validate the CLI configuration for security and consistency
    /// # Errors
    ///
    /// Returns an error if configuration validation fails, including invalid
    /// paths, missing directories, or out-of-range parameter values.
    pub fn validate(&self) -> Result<(), AppError> {
        // Validate upload configuration consistency
        if self.enable_upload.unwrap_or(false) {
            // Warn if upload size is very large
            let max_size = self.max_upload_size.unwrap_or(u64::MAX / (1024 * 1024));
            if max_size > 2048 {
                info!(
                    "Large upload size limit configured: {max_size} MB. Using direct disk streaming for efficient memory usage."
                );
            }
        }

        // Validate main serving directory
        if !self.directory.exists() {
            return Err(AppError::DirectoryNotFound(
                self.directory.to_string_lossy().to_string(),
            ));
        }

        if !self.directory.is_dir() {
            return Err(AppError::InvalidPath);
        }

        Ok(())
    }

    /// Convert upload size from MB to bytes with overflow checking
    #[must_use]
    pub fn max_upload_size_bytes(&self) -> u64 {
        // Safe conversion from u64 MB to u64 bytes
        // No upper limit - direct streaming handles any size efficiently
        self.max_upload_size
            .map_or(u64::MAX, |size| size * 1024 * 1024)
    }

    /// Get the resolved upload directory, using OS defaults if not specified
    /// # Errors
    ///
    /// Returns an error if the configured upload directory does not exist
    /// or cannot be created, or if permissions are insufficient.
    pub fn get_upload_directory(&self) -> Result<PathBuf, AppError> {
        // Always return an error since we no longer support pre-configured upload directories
        // Upload directories are now determined dynamically from the current URL
        Err(AppError::InvalidPath)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_upload_size() {
        // Valid sizes
        assert_eq!(validate_upload_size("1").unwrap(), 1);
        assert_eq!(validate_upload_size("100").unwrap(), 100);
        assert_eq!(validate_upload_size("1024").unwrap(), 1024);
        assert_eq!(validate_upload_size("10240").unwrap(), 10240);

        // Invalid sizes
        assert!(validate_upload_size("0").is_err());
        // No upper limit anymore - direct streaming handles any size
        assert!(validate_upload_size("10241").is_ok());
        assert!(validate_upload_size("18446744073709551615").is_ok()); // u64::MAX is valid now
        assert!(validate_upload_size("-1").is_err());
        assert!(validate_upload_size("abc").is_err());
    }

    #[test]
    fn test_max_upload_size_bytes() {
        let mut cli = Cli {
            directory: PathBuf::from("."),
            listen: Some("127.0.0.1".to_string()),
            port: Some(8080),
            allowed_extensions: Some("*".to_string()),
            threads: Some(4),
            chunk_size: Some(1024),
            verbose: Some(false),
            detailed_logging: Some(false),
            username: None,
            password: None,
            enable_upload: Some(false),
            max_upload_size: Some(100),
            config_file: None,
            log_dir: None,
        };

        // Test conversion
        assert_eq!(cli.max_upload_size_bytes(), 100 * 1024 * 1024);

        cli.max_upload_size = Some(1);
        assert_eq!(cli.max_upload_size_bytes(), 1024 * 1024);

        cli.max_upload_size = Some(1024);
        assert_eq!(cli.max_upload_size_bytes(), 1024 * 1024 * 1024);

        cli.max_upload_size = Some(10240);
        assert_eq!(cli.max_upload_size_bytes(), 10240 * 1024 * 1024);
    }

    #[test]
    fn test_cli_validate() {
        let temp_dir = TempDir::new().unwrap();

        // Valid configuration
        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            listen: Some("127.0.0.1".to_string()),
            port: Some(8080),
            allowed_extensions: Some("*".to_string()),
            threads: Some(4),
            chunk_size: Some(1024),
            verbose: Some(false),
            detailed_logging: Some(false),
            username: None,
            password: None,
            enable_upload: Some(true),
            max_upload_size: Some(100),
            config_file: None,
            log_dir: None,
        };

        assert!(cli.validate().is_ok());

        // Invalid serving directory
        let mut invalid_cli = cli.clone();
        invalid_cli.directory = PathBuf::from("/nonexistent/directory/path");
        assert!(invalid_cli.validate().is_err());

        // File instead of directory
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test").unwrap();
        let mut file_cli = cli.clone();
        file_cli.directory = file_path;
        assert!(file_cli.validate().is_err());
    }
}

/// Validate log directory path and ensure it exists and is writable
fn validate_log_dir(s: &str) -> Result<PathBuf, String> {
    if s.is_empty() {
        return Err("Log directory path cannot be empty".to_string());
    }

    let path = PathBuf::from(s);

    // Check if directory exists
    if !path.exists() {
        return Err(format!("Log directory does not exist: {}", path.display()));
    }

    // Check if it's a directory
    if !path.is_dir() {
        return Err(format!(
            "Log directory path is not a directory: {}",
            path.display()
        ));
    }

    // Test write permissions by creating a temporary file
    let test_file = path.join(".irondrop_write_test");
    match std::fs::File::create(&test_file) {
        Ok(_) => {
            // Remove the test file
            let _ = std::fs::remove_file(&test_file);
            Ok(path)
        }
        Err(e) => Err(format!(
            "Cannot write to log directory {}: {}",
            path.display(),
            e
        )),
    }
}
