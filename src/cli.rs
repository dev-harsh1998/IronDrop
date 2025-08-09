use crate::error::AppError;
use clap::Parser;
use log::warn;
use std::path::PathBuf;

// Defines the command-line interface using clap. 🎉
// This struct represents the structure of arguments you can pass when running the server.
#[derive(Parser, Clone)]
#[command(
     author = "Harshit Jain",
     version = "2.5.0", //  Version of our IronDrop - feels like we're shipping software! 🚢
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

    /// Enable file upload functionality - Allows clients to upload files to the server. Upload endpoint will be available at /upload. 📤
    #[arg(long)]
    pub enable_upload: Option<bool>,

    /// Maximum upload file size in MB - Limits the size of files that can be uploaded to prevent abuse and manage storage. 📏
    #[arg(long, value_parser = validate_upload_size)]
    pub max_upload_size: Option<u64>,

    /// Configuration file path - Specify a custom configuration file (INI format). If not provided, looks for irondrop.ini in current directory or ~/.config/irondrop/config.ini 🛠️
    #[arg(long, value_parser = validate_config_file)]
    pub config_file: Option<String>,
}

/// Validate upload size is within safe bounds (1-10240 MB)
fn validate_upload_size(s: &str) -> Result<u64, String> {
    let size: u64 = s
        .parse()
        .map_err(|_| "Upload size must be a positive number".to_string())?;

    if size == 0 {
        return Err("Upload size must be greater than 0 MB".to_string());
    }

    if size > 10240 {
        return Err(
            "Upload size must not exceed 10240 MB (10 GB) for security reasons".to_string(),
        );
    }

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
    pub fn validate(&self) -> Result<(), AppError> {
        // Validate upload configuration consistency
        if self.enable_upload.unwrap_or(false) {
            // Warn if upload size is very large
            let max_size = self.max_upload_size.unwrap_or(10240);
            if max_size > 2048 {
                warn!(
                    "Large upload size limit configured: {max_size} MB. Ensure adequate server resources."
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
    pub fn max_upload_size_bytes(&self) -> u64 {
        // Safe conversion from u64 MB to u64 bytes
        // Since we limit to 10240 MB max, this can't overflow
        self.max_upload_size.unwrap_or(10240) * 1024 * 1024
    }

    /// Get the resolved upload directory, using OS defaults if not specified
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
        assert!(validate_upload_size("10241").is_err());
        assert!(validate_upload_size("18446744073709551615").is_err()); // u64::MAX
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
