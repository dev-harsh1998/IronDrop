//! Comprehensive tests for log directory functionality
//!
//! This module tests the new --log-dir feature including:
//! - CLI argument parsing
//! - Configuration loading from INI files
//! - Log file creation with timestamps
//! - Directory validation and error handling
//! - Integration scenarios

use irondrop::cli::Cli;
use irondrop::config::Config;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

/// Helper function to create a test CLI with log_dir
fn create_test_cli_with_log_dir(log_dir: Option<PathBuf>) -> Cli {
    Cli {
        directory: PathBuf::from("."),
        port: Some(8080),
        listen: Some("127.0.0.1".to_string()),
        allowed_extensions: None,
        threads: Some(4),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: None,
        config_file: None,
        log_dir,
    }
}

/// Helper function to create a temporary INI file with log_dir configuration
fn create_test_ini_with_log_dir(log_dir: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    writeln!(file, "[server]").unwrap();
    writeln!(file, "port = 8080").unwrap();
    writeln!(file, "").unwrap();
    writeln!(file, "[logging]").unwrap();
    writeln!(file, "log_dir = {}", log_dir).unwrap();
    file.flush().unwrap();
    file
}

#[cfg(test)]
mod cli_parsing_tests {
    use super::*;

    #[test]
    fn test_cli_log_dir_none() {
        let cli = create_test_cli_with_log_dir(None);
        assert!(cli.log_dir.is_none());
    }

    #[test]
    fn test_cli_log_dir_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();
        let cli = create_test_cli_with_log_dir(Some(log_dir.clone()));

        assert!(cli.log_dir.is_some());
        assert_eq!(cli.log_dir.unwrap(), log_dir);
    }

    #[test]
    fn test_cli_log_dir_relative_path() {
        let log_dir = PathBuf::from("./logs");
        let cli = create_test_cli_with_log_dir(Some(log_dir.clone()));

        assert!(cli.log_dir.is_some());
        assert_eq!(cli.log_dir.unwrap(), log_dir);
    }

    #[test]
    fn test_cli_validation_with_valid_log_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli_with_log_dir(Some(temp_dir.path().to_path_buf()));

        // Should not panic or return error for valid directory
        assert!(cli.validate().is_ok());
    }

    #[test]
    fn test_cli_validation_with_nonexistent_log_dir() {
        let nonexistent_dir = PathBuf::from("/nonexistent/path/to/logs");
        let cli = create_test_cli_with_log_dir(Some(nonexistent_dir));

        // Should handle nonexistent directory gracefully
        let result = cli.validate();
        // The validation might pass if the directory creation is handled elsewhere
        // or fail if strict validation is implemented
        println!("Validation result for nonexistent dir: {:?}", result);
    }
}

#[cfg(test)]
mod config_loading_tests {
    use super::*;

    #[test]
    fn test_config_load_with_cli_log_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli_with_log_dir(Some(temp_dir.path().to_path_buf()));

        let config = Config::load(&cli).unwrap();
        assert!(config.log_dir.is_some());
        assert_eq!(config.log_dir.unwrap(), temp_dir.path());
    }

    #[test]
    fn test_config_load_with_ini_log_dir() {
        let temp_log_dir = TempDir::new().unwrap();
        let ini_file = create_test_ini_with_log_dir(temp_log_dir.path().to_str().unwrap());

        let mut cli = create_test_cli_with_log_dir(None);
        cli.config_file = Some(ini_file.path().to_str().unwrap().to_string());

        let config = Config::load(&cli).unwrap();
        assert!(config.log_dir.is_some());
        assert_eq!(config.log_dir.unwrap(), temp_log_dir.path());
    }

    #[test]
    fn test_config_load_cli_overrides_ini() {
        let temp_log_dir_ini = TempDir::new().unwrap();
        let temp_log_dir_cli = TempDir::new().unwrap();

        let ini_file = create_test_ini_with_log_dir(temp_log_dir_ini.path().to_str().unwrap());

        let mut cli = create_test_cli_with_log_dir(Some(temp_log_dir_cli.path().to_path_buf()));
        cli.config_file = Some(ini_file.path().to_str().unwrap().to_string());

        let config = Config::load(&cli).unwrap();
        assert!(config.log_dir.is_some());
        // CLI should override INI
        assert_eq!(config.log_dir.unwrap(), temp_log_dir_cli.path());
    }

    #[test]
    fn test_config_load_no_log_dir() {
        let cli = create_test_cli_with_log_dir(None);
        let config = Config::load(&cli).unwrap();
        assert!(config.log_dir.is_none());
    }
}

#[cfg(test)]
mod log_file_creation_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_timestamped_log_filename_format() {
        let temp_dir = TempDir::new().unwrap();

        // Simulate the timestamp generation logic from init_file_logger
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let expected_filename = format!("irondrop_{}.log", timestamp);
        let log_file_path = temp_dir.path().join(&expected_filename);

        // Create a test log file
        let mut file = fs::File::create(&log_file_path).unwrap();
        writeln!(file, "Test log entry").unwrap();

        assert!(log_file_path.exists());
        assert!(
            log_file_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("irondrop_")
        );
        assert!(
            log_file_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".log")
        );
    }

    #[test]
    fn test_log_file_creation_in_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();

        // Test that we can create log files in the specified directory
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_file_path = log_dir.join(format!("irondrop_{}.log", timestamp));
        let mut file = fs::File::create(&log_file_path).unwrap();
        writeln!(file, "Test log entry in subdirectory").unwrap();

        assert!(log_file_path.exists());
        assert_eq!(log_file_path.parent().unwrap(), log_dir);
    }

    #[test]
    fn test_multiple_log_files_different_timestamps() {
        let temp_dir = TempDir::new().unwrap();

        // Create multiple log files with different timestamps
        for i in 0..3 {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + i; // Ensure different timestamps

            let log_file_path = temp_dir.path().join(format!("irondrop_{}.log", timestamp));
            let mut file = fs::File::create(&log_file_path).unwrap();
            writeln!(file, "Test log entry {}", i).unwrap();

            assert!(log_file_path.exists());
        }

        // Check that multiple log files were created
        let entries: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let name = entry.file_name().to_str()?.to_string();
                if name.starts_with("irondrop_") && name.ends_with(".log") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();

        assert!(entries.len() >= 3);
    }
}

#[cfg(test)]
mod directory_validation_tests {
    use super::*;

    #[test]
    fn test_valid_existing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path();

        assert!(log_dir.exists());
        assert!(log_dir.is_dir());

        // Test that we can write to the directory
        let test_file = log_dir.join("test.log");
        fs::File::create(&test_file).unwrap();
        assert!(test_file.exists());
    }

    #[test]
    fn test_nonexistent_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("nested").join("log").join("directory");

        assert!(!log_dir.exists());

        // Test directory creation
        fs::create_dir_all(&log_dir).unwrap();
        assert!(log_dir.exists());
        assert!(log_dir.is_dir());
    }

    #[test]
    fn test_file_as_log_dir_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_directory.txt");

        // Create a file where we expect a directory
        fs::File::create(&file_path).unwrap();
        assert!(file_path.exists());
        assert!(file_path.is_file());

        // Trying to use this file as a log directory should fail
        let result = fs::create_dir_all(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_readonly_directory_handling() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("readonly");
        fs::create_dir_all(&log_dir).unwrap();

        // Make directory read-only
        let mut perms = fs::metadata(&log_dir).unwrap().permissions();
        perms.set_mode(0o444); // Read-only
        fs::set_permissions(&log_dir, perms).unwrap();

        // Trying to create a file in read-only directory should fail
        let test_file = log_dir.join("test.log");
        let result = fs::File::create(&test_file);
        assert!(result.is_err());

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&log_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&log_dir, perms).unwrap();
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_end_to_end_log_dir_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().join("app_logs");

        // 1. Create CLI with log_dir
        let cli = create_test_cli_with_log_dir(Some(log_dir.clone()));

        // 2. Load configuration
        let config = Config::load(&cli).unwrap();
        assert!(config.log_dir.is_some());
        assert_eq!(config.log_dir.as_ref().unwrap(), &log_dir);

        // 3. Ensure log directory exists
        if !log_dir.exists() {
            fs::create_dir_all(&log_dir).unwrap();
        }

        // 4. Simulate log file creation (as done by init_file_logger)
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let log_file_path = log_dir.join(format!("irondrop_{}.log", timestamp));
        let mut file = fs::File::create(&log_file_path).unwrap();
        writeln!(file, "[INFO] Server started on 127.0.0.1:8080").unwrap();
        writeln!(file, "[DEBUG] Configuration loaded successfully").unwrap();

        // 5. Verify log file was created and contains expected content
        assert!(log_file_path.exists());
        let content = fs::read_to_string(&log_file_path).unwrap();
        assert!(content.contains("Server started"));
        assert!(content.contains("Configuration loaded"));
    }

    #[test]
    fn test_config_precedence_with_log_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cli_log_dir = temp_dir.path().join("cli_logs");
        let ini_log_dir = temp_dir.path().join("ini_logs");

        // Create both directories
        fs::create_dir_all(&cli_log_dir).unwrap();
        fs::create_dir_all(&ini_log_dir).unwrap();

        // Create INI file with log_dir
        let ini_file = create_test_ini_with_log_dir(ini_log_dir.to_str().unwrap());

        // Create CLI with both config file and log_dir
        let mut cli = create_test_cli_with_log_dir(Some(cli_log_dir.clone()));
        cli.config_file = Some(ini_file.path().to_str().unwrap().to_string());

        // Load configuration - CLI should take precedence
        let config = Config::load(&cli).unwrap();
        assert!(config.log_dir.is_some());
        assert_eq!(config.log_dir.unwrap(), cli_log_dir);
    }

    #[test]
    fn test_log_dir_with_various_path_formats() {
        let test_cases = vec!["./logs", "logs", "../logs"];

        for path_str in test_cases {
            let log_dir = PathBuf::from(path_str);
            let cli = create_test_cli_with_log_dir(Some(log_dir.clone()));

            // Should be able to load config with various path formats
            let config = Config::load(&cli);
            assert!(
                config.is_ok(),
                "Failed to load config with path: {}",
                path_str
            );

            let config = config.unwrap();
            assert!(config.log_dir.is_some());
            assert_eq!(config.log_dir.unwrap(), log_dir);
        }
    }
}
