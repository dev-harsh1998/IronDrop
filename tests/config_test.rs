// SPDX-License-Identifier: MIT
use irondrop::cli::Cli;
use irondrop::config::{Config, ini_parser::IniConfig};
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
    assert_eq!(extensions, ["*.zip", "*.txt", "*.pdf"]);

    let empty = ini.get_list("security", "empty_list");
    assert_eq!(empty, Vec::<String>::new());

    let single = ini.get_list("security", "single_item");
    assert_eq!(single, ["*.doc"]);
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
        log_dir: None,
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
        log_dir: None,
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
        log_dir: None,
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
    assert_eq!(config.allowed_extensions, ["*.zip", "*.txt"]);
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
        log_dir: None,
    };

    let result = Config::load(&cli);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("Config file specified but not found")
    );
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
        log_dir: None,
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
        log_dir: None,
    };

    let config = Config::load(&cli).expect("Failed to load config");

    assert_eq!(config.username, Some("configuser".to_string()));
    assert_eq!(config.password, Some("configpass123".to_string()));
    assert_eq!(config.port, 9999);
}

#[test]
fn test_config_invalid_port_values() {
    let temp_dir = TempDir::new().unwrap();

    let test_cases = [
        ("port = -1", "negative port"),
        ("port = 65536", "port too high"),
        ("port = abc", "non-numeric port"),
        ("port = 8080.5", "decimal port"),
        ("port = ", "empty port"),
    ];

    // Test port 0 separately as it's technically valid but unusual
    let zero_port_cases = [("port = 0", "zero port")];

    for (port_config, description) in test_cases {
        let config_file = temp_dir
            .path()
            .join(format!("test_{}.ini", description.replace(" ", "_")));
        let config_content = format!("[server]\n{}", port_config);
        fs::write(&config_file, config_content).unwrap();

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
            log_dir: None,
        };

        let result = Config::load(&cli);

        // Should either use default port or return error for invalid values
        match result {
            Ok(config) => {
                // If parsing succeeds, should use a non-zero port for invalid values
                assert!(
                    config.port != 0,
                    "Should use valid non-zero port for {}",
                    description
                );
            }
            Err(_) => {
                // Acceptable to return error for invalid port values
            }
        }
    }

    // Test port 0 case separately - it parses successfully but is unusual
    for (port_config, description) in zero_port_cases {
        let config_file = temp_dir
            .path()
            .join(format!("test_{}.ini", description.replace(" ", "_")));
        let config_content = format!("[server]\n{}", port_config);
        fs::write(&config_file, config_content).unwrap();

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
            log_dir: None,
        };

        let result = Config::load(&cli);

        // Port 0 is technically valid u16 but unusual for servers
        match result {
            Ok(config) => {
                // Port 0 is parsed successfully, so we accept it
                assert_eq!(
                    config.port, 0,
                    "Port 0 should be parsed as 0 for {}",
                    description
                );
            }
            Err(_) => {
                // Also acceptable if the system rejects port 0
            }
        }
    }
}

#[test]
fn test_config_invalid_file_size_formats() {
    let temp_dir = TempDir::new().unwrap();

    let test_cases = [
        "max_upload_size = -1MB",
        "max_upload_size = 0.5.5GB",
        "max_upload_size = ABCMB",
        "max_upload_size = 100XB", // Invalid unit
        "max_upload_size = MB100", // Unit before number
        "max_upload_size = ",      // Empty value
    ];

    for (i, invalid_config) in test_cases.iter().enumerate() {
        let config_file = temp_dir.path().join(format!("test_size_{}.ini", i));
        let config_content = format!("[upload]\n{}", invalid_config);
        fs::write(&config_file, config_content).unwrap();

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
            log_dir: None,
        };

        let result = Config::load(&cli);

        // Should either use default or return error
        match result {
            Ok(config) => {
                // If parsing succeeds, should either have a reasonable file size or the default (u64::MAX)
                // For invalid formats, the system falls back to u64::MAX (effectively unlimited)
                assert!(
                    config.max_upload_size > 0,
                    "Should have valid file size for case: {}",
                    invalid_config
                );

                // For clearly invalid formats like negative values, we expect the fallback to u64::MAX
                if invalid_config.contains("-1MB")
                    || invalid_config.contains("ABCMB")
                    || invalid_config.contains("100XB")
                    || invalid_config.contains("MB100")
                    || invalid_config.contains("= ")
                {
                    assert_eq!(
                        config.max_upload_size,
                        u64::MAX,
                        "Invalid file size should fall back to u64::MAX for case: {}",
                        invalid_config
                    );
                }
            }
            Err(_) => {
                // Acceptable to return error for invalid file size formats
            }
        }
    }
}

#[test]
fn test_config_boolean_edge_cases() {
    let temp_dir = TempDir::new().unwrap();

    let test_cases = [
        ("enable_upload = TRUE", true),
        ("enable_upload = FALSE", false),
        ("enable_upload = True", true),
        ("enable_upload = False", false),
        ("enable_upload = 1", true),
        ("enable_upload = 0", false),
        ("enable_upload = yes", true),
        ("enable_upload = no", false),
        ("enable_upload = on", true),
        ("enable_upload = off", false),
    ];

    for (i, (config_line, expected)) in test_cases.iter().enumerate() {
        let config_file = temp_dir.path().join(format!("test_bool_{}.ini", i));
        let config_content = format!("[upload]\n{}", config_line);
        fs::write(&config_file, config_content).unwrap();

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
            log_dir: None,
        };

        let result = Config::load(&cli);

        if let Ok(config) = result {
            assert_eq!(
                config.enable_upload, *expected,
                "Failed for: {}",
                config_line
            );
        }
    }
}

#[test]
fn test_config_malformed_ini_syntax() {
    let temp_dir = TempDir::new().unwrap();

    let malformed_configs = [
        "[server\nport = 8080", // Missing closing bracket
        "server]\nport = 8080", // Missing opening bracket
        "[server]\nport 8080",  // Missing equals sign
        "[server]\n= 8080",     // Missing key
        "[server]\nport =\n",   // Missing value
        "[\nport = 8080",       // Empty section name
    ];

    for (i, malformed_config) in malformed_configs.iter().enumerate() {
        let config_file = temp_dir.path().join(format!("test_malformed_{}.ini", i));
        fs::write(&config_file, malformed_config).unwrap();

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
            log_dir: None,
        };

        let result = Config::load(&cli);

        // Should either handle gracefully or return appropriate error
        // This test ensures no panic occurs
    }
}
