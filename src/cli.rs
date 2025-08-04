use crate::error::AppError;
use clap::Parser;
use log::{error, warn};
use std::fs;
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
    #[arg(short, long, default_value = "127.0.0.1")]
    pub listen: String,

    /// Port number to listen on -  Like a door number for the server to receive requests. 🚪
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,

    /// Allowed file extensions for download (comma-separated, supports wildcards like *.zip, *.txt) -  Security measure to only share certain file types. 🔒
    #[arg(short, long, default_value = "*.zip,*.txt")]
    pub allowed_extensions: String,

    /// Number of threads in the thread pool -  More threads = handle more downloads at once, up to a point. 🧵🧵🧵
    #[arg(short, long, default_value_t = 8)]
    pub threads: usize,

    /// Chunk size for reading files (in bytes) -  How much data we read from a file at a time when sending it. Smaller chunks are gentler on memory. 📦
    /// This is the size of the buffer used to read files in chunks
    #[arg(short, long, default_value_t = 1024)]
    pub chunk_size: usize,

    /// Enable verbose logging for debugging (log level: debug) -  For super detailed logs, useful when things go wrong or you're developing. 🐛
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// Enable more detailed logging (log level: info if verbose=false, debug if verbose=true) -  More logs than usual, but not *too* much. Good for general monitoring. ℹ️
    #[arg(long, default_value_t = false)]
    pub detailed_logging: bool,

    /// Username for basic authentication.
    #[arg(long)]
    pub username: Option<String>,

    /// Password for basic authentication.
    #[arg(long)]
    pub password: Option<String>,

    /// Enable file upload functionality - Allows clients to upload files to the server. Upload endpoint will be available at /upload. 📤
    #[arg(long, default_value_t = false)]
    pub enable_upload: bool,

    /// Maximum upload file size in MB - Limits the size of files that can be uploaded to prevent abuse and manage storage. 📏
    #[arg(long, default_value_t = 10240, value_parser = validate_upload_size)]
    pub max_upload_size: u64,

    /// Upload target directory - Directory where uploaded files will be stored. If not specified, uses the OS default download directory. 📁
    #[arg(long, value_parser = validate_upload_dir)]
    pub upload_dir: Option<PathBuf>,
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

/// Validate upload directory path for security
fn validate_upload_dir(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);

    // Check for empty path
    if s.is_empty() {
        return Err("Upload directory path cannot be empty".to_string());
    }

    // Canonicalize the path to resolve any .. or . components
    let canonical_path = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            // If path doesn't exist yet, try to canonicalize the parent
            if let Some(parent) = path.parent() {
                if parent.exists() {
                    match parent.canonicalize() {
                        Ok(parent_canonical) => parent_canonical
                            .join(path.file_name().ok_or("Invalid path components")?),
                        Err(e) => return Err(format!("Cannot resolve parent directory: {e}")),
                    }
                } else {
                    return Err("Parent directory does not exist".to_string());
                }
            } else {
                return Err("Invalid path: no parent directory".to_string());
            }
        }
    };

    // Ensure the path is absolute
    if !canonical_path.is_absolute() {
        return Err("Upload directory must be an absolute path".to_string());
    }

    // Check for suspicious patterns that might indicate path traversal
    let path_str = canonical_path.to_string_lossy();
    if path_str.contains("..") {
        return Err("Path traversal patterns detected in resolved path".to_string());
    }

    // On Linux systems, check if trying to write to system directories
    #[cfg(target_os = "linux")]
    {
        let forbidden_prefixes = ["/etc", "/sys", "/proc", "/dev", "/boot"];
        for prefix in &forbidden_prefixes {
            if path_str.starts_with(prefix) {
                return Err(format!(
                    "Cannot use system directory {prefix} as upload directory"
                ));
            }
        }
    }

    // On Windows, check for system directories
    #[cfg(windows)]
    {
        let path_to_check = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
        let forbidden_patterns = [
            "C:\\Windows",
            "C:\\Program Files",
            "C:\\Program Files (x86)",
        ];
        for pattern in &forbidden_patterns {
            if path_to_check.len() >= pattern.len()
                && path_to_check[..pattern.len()].eq_ignore_ascii_case(pattern)
            {
                return Err("Cannot use Windows system directory as upload directory".to_string());
            }
        }
    }

    Ok(canonical_path)
}

impl Cli {
    /// Validate the CLI configuration for security and consistency
    pub fn validate(&self) -> Result<(), AppError> {
        // Validate upload configuration consistency
        if self.enable_upload {
            // Additional runtime validation for upload directory
            if let Some(ref upload_dir) = self.upload_dir {
                // Check if directory exists or can be created
                if !upload_dir.exists() {
                    // Try to create it
                    if let Err(e) = fs::create_dir_all(upload_dir) {
                        error!("Failed to create upload directory {upload_dir:?}: {e}");
                        return Err(AppError::DirectoryNotFound(format!(
                            "Cannot create upload directory: {e}"
                        )));
                    }
                }

                // Verify it's a directory
                if !upload_dir.is_dir() {
                    return Err(AppError::InvalidPath);
                }

                // Check write permissions by attempting to create a test file
                let test_file = upload_dir.join(".irondrop_test");
                match fs::File::create(&test_file) {
                    Ok(_) => {
                        let _ = fs::remove_file(&test_file);
                    }
                    Err(e) => {
                        error!("Upload directory {upload_dir:?} is not writable: {e}");
                        return Err(AppError::InternalServerError(format!(
                            "Upload directory is not writable: {e}"
                        )));
                    }
                }
            }

            // Warn if upload size is very large
            if self.max_upload_size > 2048 {
                warn!(
                    "Large upload size limit configured: {} MB. Ensure adequate server resources.",
                    self.max_upload_size
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
        self.max_upload_size * 1024 * 1024
    }

    /// Get the resolved upload directory, using OS defaults if not specified
    pub fn get_upload_directory(&self) -> Result<PathBuf, AppError> {
        match &self.upload_dir {
            Some(dir) => Ok(dir.clone()),
            None => {
                // This will be handled by UploadHandler::detect_os_download_directory()
                // We just return an error here to indicate it needs resolution
                Err(AppError::InvalidPath)
            }
        }
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
    fn test_validate_upload_dir() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        // Valid directory
        assert!(validate_upload_dir(temp_path).is_ok());

        // Empty path
        assert!(validate_upload_dir("").is_err());

        // Non-existent path with valid parent
        let new_dir = temp_dir.path().join("newdir");
        assert!(validate_upload_dir(new_dir.to_str().unwrap()).is_ok());

        // System directories (Linux)
        #[cfg(target_os = "linux")]
        {
            assert!(validate_upload_dir("/etc").is_err());
            assert!(validate_upload_dir("/sys").is_err());
            assert!(validate_upload_dir("/proc").is_err());
            assert!(validate_upload_dir("/dev").is_err());
            assert!(validate_upload_dir("/boot").is_err());
        }

        // System directories (Windows)
        #[cfg(windows)]
        {
            assert!(validate_upload_dir("C:\\Windows").is_err());
            assert!(validate_upload_dir("C:\\Program Files").is_err());
        }
    }

    #[test]
    fn test_max_upload_size_bytes() {
        let mut cli = Cli {
            directory: PathBuf::from("."),
            listen: "127.0.0.1".to_string(),
            port: 8080,
            allowed_extensions: "*".to_string(),
            threads: 4,
            chunk_size: 1024,
            verbose: false,
            detailed_logging: false,
            username: None,
            password: None,
            enable_upload: false,
            max_upload_size: 100,
            upload_dir: None,
        };

        // Test conversion
        assert_eq!(cli.max_upload_size_bytes(), 100 * 1024 * 1024);

        cli.max_upload_size = 1;
        assert_eq!(cli.max_upload_size_bytes(), 1024 * 1024);

        cli.max_upload_size = 1024;
        assert_eq!(cli.max_upload_size_bytes(), 1024 * 1024 * 1024);

        cli.max_upload_size = 10240;
        assert_eq!(cli.max_upload_size_bytes(), 10240 * 1024 * 1024);
    }

    #[test]
    fn test_cli_validate() {
        let temp_dir = TempDir::new().unwrap();

        // Valid configuration
        let cli = Cli {
            directory: temp_dir.path().to_path_buf(),
            listen: "127.0.0.1".to_string(),
            port: 8080,
            allowed_extensions: "*".to_string(),
            threads: 4,
            chunk_size: 1024,
            verbose: false,
            detailed_logging: false,
            username: None,
            password: None,
            enable_upload: true,
            max_upload_size: 100,
            upload_dir: Some(temp_dir.path().to_path_buf()),
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

    #[test]
    fn test_path_traversal_detection() {
        // Various path traversal attempts
        let traversal_attempts = vec!["../etc/passwd", "./../../etc/passwd", "/tmp/../etc/passwd"];

        for path in traversal_attempts {
            let result = validate_upload_dir(path);
            if result.is_ok() {
                let canonical = result.unwrap();
                let canonical_str = canonical.to_string_lossy();
                // Ensure no ".." in resolved path
                assert!(
                    !canonical_str.contains(".."),
                    "Path traversal not properly resolved: {canonical_str}"
                );
            }
        }
    }
}
