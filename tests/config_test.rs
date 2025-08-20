use irondrop::cli::Cli;
use irondrop::config::{ini_parser::IniConfig, Config};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_ini_parser_basic() {
    let ini_content = r#"
# This is a comment
[server]
listen = 0.0.0.0
port = 9000
threads = 16

[upload]
enabled = true
max_size = 2GB

[logging]
verbose = false
"#;

    let ini = IniConfig::parse(ini_content).expect("Failed to parse INI");

    assert_eq!(
        ini.get_string("server", "listen"),
        Some("0.0.0.0".to_string())
    );
    assert_eq!(ini.get_u16("server", "port"), Some(9000));
    assert_eq!(ini.get_usize("server", "threads"), Some(16));
    assert_eq!(ini.get_bool("upload", "enabled"), Some(true));
    assert_eq!(
        ini.get_file_size("upload", "max_size"),
        Some(2 * 1024 * 1024 * 1024)
    );
    assert_eq!(ini.get_bool("logging", "verbose"), Some(false));
}

#[test]
fn test_ini_parser_file_sizes() {
    let ini_content = r#"
[upload]
size_bytes = 1024
size_kb = 500KB
size_mb = 100MB
size_gb = 5GB
size_tb = 2TB
"#;

    let ini = IniConfig::parse(ini_content).expect("Failed to parse INI");

    assert_eq!(ini.get_file_size("upload", "size_bytes"), Some(1024));
    assert_eq!(ini.get_file_size("upload", "size_kb"), Some(500 * 1024));
    assert_eq!(
        ini.get_file_size("upload", "size_mb"),
        Some(100 * 1024 * 1024)
    );
    assert_eq!(
        ini.get_file_size("upload", "size_gb"),
        Some(5 * 1024 * 1024 * 1024)
    );
    assert_eq!(
        ini.get_file_size("upload", "size_tb"),
        Some(2 * 1024 * 1024 * 1024 * 1024)
    );
}

#[test]
fn test_ini_parser_boolean_formats() {
    let ini_content = r#"
[test]
bool_true = true
bool_false = false
bool_yes = yes
bool_no = no
bool_on = on
bool_off = off
bool_1 = 1
bool_0 = 0
"#;

    let ini = IniConfig::parse(ini_content).expect("Failed to parse INI");

    assert_eq!(ini.get_bool("test", "bool_true"), Some(true));
    assert_eq!(ini.get_bool("test", "bool_false"), Some(false));
    assert_eq!(ini.get_bool("test", "bool_yes"), Some(true));
    assert_eq!(ini.get_bool("test", "bool_no"), Some(false));
    assert_eq!(ini.get_bool("test", "bool_on"), Some(true));
    assert_eq!(ini.get_bool("test", "bool_off"), Some(false));
    assert_eq!(ini.get_bool("test", "bool_1"), Some(true));
    assert_eq!(ini.get_bool("test", "bool_0"), Some(false));
}

#[test]
fn test_ini_parser_list_parsing() {
    let ini_content = r#"
[security]
extensions = *.zip,*.txt,*.pdf
empty_list = 
single_item = *.doc
"#;

    let ini = IniConfig::parse(ini_content).expect("Failed to parse INI");

    let extensions = ini.get_list("security", "extensions");
    assert_eq!(extensions, vec!["*.zip", "*.txt", "*.pdf"]);

    let empty = ini.get_list("security", "empty_list");
    assert_eq!(empty, Vec::<String>::new());

    let single = ini.get_list("security", "single_item");
    assert_eq!(single, vec!["*.doc"]);
}

#[test]
fn test_ini_parser_comments_and_whitespace() {
    let ini_content = r#"
# Global comment
key1 = value1

[section1]
# Section comment
  key2   =   value2   # Inline comment
key3=value3

# Another comment
[section2]
key4 = value4
"#;

    let ini = IniConfig::parse(ini_content).expect("Failed to parse INI");

    assert_eq!(ini.get_string("", "key1"), Some("value1".to_string()));
    assert_eq!(
        ini.get_string("section1", "key2"),
        Some("value2".to_string())
    );
    assert_eq!(
        ini.get_string("section1", "key3"),
        Some("value3".to_string())
    );
    assert_eq!(
        ini.get_string("section2", "key4"),
        Some("value4".to_string())
    );
}

#[test]
fn test_config_precedence_cli_highest() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("test.ini");

    fs::write(
        &config_file,
        r#"
[server]
port = 9000
threads = 16
listen = 0.0.0.0

[logging]
verbose = false
"#,
    )
    .unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: Some("192.168.1.1".to_string()), // CLI override
        port: Some(8888),                        // CLI override
        allowed_extensions: Some("*.zip,*.txt".to_string()),
        threads: Some(4), // CLI override (non-default value)
        chunk_size: Some(1024),
        verbose: Some(true), // CLI override
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: Some(config_file.to_string_lossy().to_string()),
        log_file: None,
    };

    let config = Config::load(&cli).expect("Failed to load config");

    // CLI should have highest precedence over INI file
    assert_eq!(config.listen, "192.168.1.1");
    assert_eq!(config.port, 8888);
    assert_eq!(config.threads, 4);
    assert_eq!(config.verbose, true);
}

#[test]
fn test_config_file_discovery() {
    let temp_dir = TempDir::new().unwrap();

    // Create a config file in the temp directory
    let config_content = r#"
[server]
port = 5555
threads = 4

[upload]
enable_upload = true
max_upload_size = 1GB
"#;

    // Test explicit config file path
    let explicit_config = temp_dir.path().join("explicit.ini");
    fs::write(&explicit_config, config_content).unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: None,
        port: None,
        allowed_extensions: None,
        threads: None,
        chunk_size: None,
        verbose: None,
        detailed_logging: None,
        username: None,
        password: None,
        enable_upload: None,
        max_upload_size: None,
        config_file: Some(explicit_config.to_string_lossy().to_string()),
        log_file: None,
    };

    let config = Config::load(&cli).expect("Failed to load config");

    assert_eq!(config.port, 5555);
    assert_eq!(config.threads, 4);
    assert_eq!(config.enable_upload, true);
    assert_eq!(config.max_upload_size, 1024 * 1024 * 1024); // 1GB in bytes
}

#[test]
fn test_config_defaults() {
    let temp_dir = TempDir::new().unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(8080),
        allowed_extensions: Some("*.zip,*.txt".to_string()),
        threads: Some(8),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: None,
        log_file: None,
    };

    let config = Config::load(&cli).expect("Failed to load config");

    // Should use default/CLI values
    assert_eq!(config.listen, "127.0.0.1");
    assert_eq!(config.port, 8080);
    assert_eq!(config.threads, 8);
    assert_eq!(config.chunk_size, 1024);
    assert_eq!(config.enable_upload, false);
    assert_eq!(config.max_upload_size, 10240 * 1024 * 1024); // 10GB in bytes
    assert_eq!(config.username, None);
    assert_eq!(config.password, None);
    assert_eq!(config.allowed_extensions, vec!["*.zip", "*.txt"]);
    assert_eq!(config.verbose, false);
    assert_eq!(config.detailed_logging, false);
}

#[test]
fn test_config_file_load_error() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent_config = temp_dir.path().join("nonexistent.ini");

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: Some("127.0.0.1".to_string()),
        port: Some(8080),
        allowed_extensions: Some("*.zip,*.txt".to_string()),
        threads: Some(8),
        chunk_size: Some(1024),
        verbose: Some(false),
        detailed_logging: Some(false),
        username: None,
        password: None,
        enable_upload: Some(false),
        max_upload_size: Some(10240),
        config_file: Some(nonexistent_config.to_string_lossy().to_string()),
        log_file: None,
    };

    let result = Config::load(&cli);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("Config file specified but not found"));
}

#[test]
fn test_ini_parser_invalid_syntax() {
    // Test various invalid INI syntax
    let invalid_content = r#"
[section without closing bracket
key = value
"#;

    let result = IniConfig::parse(invalid_content);
    assert!(result.is_ok()); // Should handle gracefully, ignoring invalid lines

    let ini = result.unwrap();
    assert_eq!(ini.get_string("", "key"), Some("value".to_string()));
}

#[test]
fn test_config_upload_settings() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("upload_test.ini");
    let upload_dir = temp_dir.path().join("uploads");
    fs::create_dir_all(&upload_dir).unwrap();

    fs::write(
        &config_file,
        format!(
            r#"
[upload]
enable_upload = true
max_upload_size = 500MB
directory = {}

[server]
directory = {}
"#,
            upload_dir.to_string_lossy(),
            temp_dir.path().to_string_lossy()
        ),
    )
    .unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: None,
        port: None,
        allowed_extensions: None,
        threads: None,
        chunk_size: None,
        verbose: None,
        detailed_logging: None,
        username: None,
        password: None,
        enable_upload: None,
        max_upload_size: None,
        config_file: Some(config_file.to_string_lossy().to_string()),
        log_file: None,
    };

    let config = Config::load(&cli).expect("Failed to load config");

    assert_eq!(config.enable_upload, true);
    assert_eq!(config.max_upload_size, 500 * 1024 * 1024); // 500MB in bytes
}

#[test]
fn test_config_authentication_settings() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("auth_test.ini");

    fs::write(
        &config_file,
        r#"
[auth]
username = configuser
password = configpass123

[server]
port = 9999
"#,
    )
    .unwrap();

    let cli = Cli {
        directory: temp_dir.path().to_path_buf(),
        listen: None,
        port: None,
        allowed_extensions: None,
        threads: None,
        chunk_size: None,
        verbose: None,
        detailed_logging: None,
        username: None,
        password: None,
        enable_upload: None,
        max_upload_size: None,
        config_file: Some(config_file.to_string_lossy().to_string()),
        log_file: None,
    };

    let config = Config::load(&cli).expect("Failed to load config");

    assert_eq!(config.username, Some("configuser".to_string()));
    assert_eq!(config.password, Some("configpass123".to_string()));
    assert_eq!(config.port, 9999);
}
