//! Configuration management for IronDrop
//! Supports INI files with CLI argument overrides

pub mod ini_parser;

use crate::cli::Cli;
use ini_parser::IniConfig;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Config {
    // Server settings
    pub listen: String,
    pub port: u16,
    pub threads: usize,
    pub chunk_size: usize,
    pub directory: PathBuf,

    // Upload settings
    pub enable_upload: bool,
    pub max_upload_size: u64,
    pub upload_dir: Option<PathBuf>,

    // Security settings
    pub username: Option<String>,
    pub password: Option<String>,
    pub allowed_extensions: Vec<String>,

    // Logging settings
    pub verbose: bool,
    pub detailed_logging: bool,
}

impl Config {
    /// Load configuration with precedence: CLI args > INI file > Defaults
    pub fn load(cli: &Cli) -> Result<Self, String> {
        // Try to load configuration file
        let config_file = Self::find_config_file(cli)?;
        let ini = if let Some(path) = config_file {
            log::info!("Loading configuration from: {}", path.display());
            IniConfig::load_file(&path)?
        } else {
            log::info!("No configuration file found, using defaults and CLI overrides");
            IniConfig::new()
        };

        // Build configuration with precedence
        Ok(Self {
            listen: Self::get_listen(&ini, cli),
            port: Self::get_port(&ini, cli),
            threads: Self::get_threads(&ini, cli),
            chunk_size: Self::get_chunk_size(&ini, cli),
            directory: Self::get_directory(&ini, cli)?,

            enable_upload: Self::get_enable_upload(&ini, cli),
            max_upload_size: Self::get_max_upload_size(&ini, cli),
            upload_dir: Self::get_upload_dir(&ini, cli),

            username: Self::get_username(&ini, cli),
            password: Self::get_password(&ini, cli),
            allowed_extensions: Self::get_allowed_extensions(&ini, cli),

            verbose: Self::get_verbose(&ini, cli),
            detailed_logging: Self::get_detailed_logging(&ini, cli),
        })
    }

    /// Find configuration file in order of preference
    fn find_config_file(cli: &Cli) -> Result<Option<PathBuf>, String> {
        // 1. Check if config file is explicitly specified via CLI
        if let Some(ref config_path) = cli.config_file {
            let path = PathBuf::from(config_path);
            if path.exists() {
                return Ok(Some(path));
            } else {
                return Err(format!(
                    "Config file specified but not found: {config_path}"
                ));
            }
        }

        // 2. Check current directory
        let current_config = PathBuf::from("irondrop.ini");
        if current_config.exists() {
            return Ok(Some(current_config));
        }

        // 3. Check current directory with .conf extension
        let current_config_alt = PathBuf::from("irondrop.conf");
        if current_config_alt.exists() {
            return Ok(Some(current_config_alt));
        }

        // 4. Check user config directory (~/.config/irondrop/config.ini)
        if let Some(home_dir) = std::env::var_os("HOME") {
            let user_config = Path::new(&home_dir)
                .join(".config")
                .join("irondrop")
                .join("config.ini");
            if user_config.exists() {
                return Ok(Some(user_config));
            }
        }

        // 6. Check system config (Unix-like systems)
        #[cfg(unix)]
        {
            let system_config = PathBuf::from("/etc/irondrop/config.ini");
            if system_config.exists() {
                return Ok(Some(system_config));
            }
        }

        Ok(None)
    }

    // Configuration value getters with precedence: CLI > ENV > INI > Default

    fn get_listen(ini: &IniConfig, cli: &Cli) -> String {
        // CLI argument
        if !cli.listen.is_empty() && cli.listen != "127.0.0.1" {
            return cli.listen.clone();
        }

        // INI file
        if let Some(listen) = ini.get_string("server", "listen") {
            return listen;
        }

        // Default
        "127.0.0.1".to_string()
    }

    fn get_port(ini: &IniConfig, cli: &Cli) -> u16 {
        // CLI argument (check if not default)
        if cli.port != 8080 {
            return cli.port;
        }

        // INI file
        if let Some(port) = ini.get_u16("server", "port") {
            return port;
        }

        // Default
        8080
    }

    fn get_threads(ini: &IniConfig, cli: &Cli) -> usize {
        // CLI argument (check if not default)
        if cli.threads != 8 {
            return cli.threads;
        }

        // INI file
        if let Some(threads) = ini.get_usize("server", "threads") {
            return threads;
        }

        // Default
        8
    }

    fn get_chunk_size(ini: &IniConfig, cli: &Cli) -> usize {
        // CLI argument (check if not default)
        if cli.chunk_size != 1024 {
            return cli.chunk_size;
        }

        // INI file
        if let Some(chunk_size) = ini.get_usize("server", "chunk_size") {
            return chunk_size;
        }

        // Default
        1024
    }

    fn get_directory(_ini: &IniConfig, cli: &Cli) -> Result<PathBuf, String> {
        // CLI argument (always available since it's required)
        Ok(cli.directory.clone())
    }

    fn get_enable_upload(ini: &IniConfig, cli: &Cli) -> bool {
        // CLI argument
        if cli.enable_upload {
            return true;
        }

        // INI file
        if let Some(enabled) = ini.get_bool("upload", "enabled") {
            return enabled;
        }

        // Default
        false
    }

    fn get_max_upload_size(ini: &IniConfig, cli: &Cli) -> u64 {
        // CLI argument (check if not default)
        if cli.max_upload_size != 10240 {
            return cli.max_upload_size * 1024 * 1024; // Convert MB to bytes
        }

        // INI file (supports file size format like "10GB")
        if let Some(size_bytes) = ini.get_file_size("upload", "max_size") {
            return size_bytes;
        }

        // Default: 10GB in bytes
        10240u64 * 1024 * 1024
    }

    fn get_upload_dir(ini: &IniConfig, cli: &Cli) -> Option<PathBuf> {
        // CLI argument
        if let Some(ref upload_dir) = cli.upload_dir {
            return Some(upload_dir.clone());
        }

        // INI file
        if let Some(upload_dir) = ini.get_string("upload", "directory") {
            return Some(PathBuf::from(upload_dir));
        }

        // Default: None (will use OS default download directory)
        None
    }

    fn get_username(ini: &IniConfig, cli: &Cli) -> Option<String> {
        // CLI argument
        if let Some(ref username) = cli.username {
            return Some(username.clone());
        }

        // INI file
        ini.get_string("auth", "username")
    }

    fn get_password(ini: &IniConfig, cli: &Cli) -> Option<String> {
        // CLI argument
        if let Some(ref password) = cli.password {
            return Some(password.clone());
        }

        // INI file
        ini.get_string("auth", "password")
    }

    fn get_allowed_extensions(ini: &IniConfig, cli: &Cli) -> Vec<String> {
        // CLI argument (check if not default)
        if cli.allowed_extensions != "*.zip,*.txt" {
            return cli
                .allowed_extensions
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // INI file
        let ini_extensions = ini.get_list("security", "allowed_extensions");
        if !ini_extensions.is_empty() {
            return ini_extensions;
        }

        // Default
        vec!["*.zip".to_string(), "*.txt".to_string()]
    }

    fn get_verbose(ini: &IniConfig, cli: &Cli) -> bool {
        // CLI argument
        if cli.verbose {
            return true;
        }

        // INI file
        ini.get_bool_or("logging", "verbose", false)
    }

    fn get_detailed_logging(ini: &IniConfig, cli: &Cli) -> bool {
        // CLI argument
        if cli.detailed_logging {
            return true;
        }

        // INI file
        ini.get_bool_or("logging", "detailed", false)
    }

    /// Print configuration summary
    pub fn print_summary(&self) {
        log::info!("Configuration Summary:");
        log::info!("  Server: {}:{}", self.listen, self.port);
        log::info!("  Directory: {}", self.directory.display());
        log::info!("  Threads: {}", self.threads);
        log::info!("  Chunk Size: {} bytes", self.chunk_size);
        log::info!("  Upload Enabled: {}", self.enable_upload);
        if self.enable_upload {
            log::info!(
                "  Max Upload Size: {} MB",
                self.max_upload_size / (1024 * 1024)
            );
            if let Some(ref upload_dir) = self.upload_dir {
                log::info!("  Upload Directory: {}", upload_dir.display());
            }
        }
        log::info!(
            "  Authentication: {}",
            if self.username.is_some() {
                "Enabled"
            } else {
                "Disabled"
            }
        );
        log::info!("  Allowed Extensions: {:?}", self.allowed_extensions);
        log::info!("  Verbose Logging: {}", self.verbose);
        log::info!("  Detailed Logging: {}", self.detailed_logging);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_cli(directory: PathBuf) -> Cli {
        Cli {
            directory,
            listen: "127.0.0.1".to_string(),
            port: 8080,
            allowed_extensions: "*.zip,*.txt".to_string(),
            threads: 8,
            chunk_size: 1024,
            verbose: false,
            detailed_logging: false,
            username: None,
            password: None,
            enable_upload: false,
            max_upload_size: 10240,
            upload_dir: None,
            config_file: None,
        }
    }

    #[test]
    fn test_config_load_no_config_file() {
        let temp_dir = TempDir::new().unwrap();

        let cli = create_test_cli(temp_dir.path().to_path_buf());
        let config = Config::load(&cli).unwrap();

        // Should use CLI defaults when no config file exists
        assert_eq!(config.listen, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.threads, 8);
        assert_eq!(config.chunk_size, 1024);
        assert_eq!(config.directory, temp_dir.path());
        assert_eq!(config.enable_upload, false);
        assert_eq!(config.max_upload_size, 10240 * 1024 * 1024);
        assert_eq!(config.upload_dir, None);
        assert_eq!(config.username, None);
        assert_eq!(config.password, None);
        assert_eq!(config.allowed_extensions, vec!["*.zip", "*.txt"]);
        assert_eq!(config.verbose, false);
        assert_eq!(config.detailed_logging, false);
    }

    #[test]
    fn test_config_load_with_ini_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.ini");

        let ini_content = r#"
[server]
listen = 0.0.0.0
port = 9000
threads = 16
chunk_size = 2048

[upload]
enabled = true
max_size = 5GB

[auth]
username = testuser
password = testpass

[security]
allowed_extensions = *.pdf,*.doc

[logging]
verbose = true
detailed = false
"#;

        fs::write(&config_file, ini_content).unwrap();

        let mut cli = create_test_cli(temp_dir.path().to_path_buf());
        cli.config_file = Some(config_file.to_string_lossy().to_string());

        let config = Config::load(&cli).unwrap();

        // Should use INI file values
        assert_eq!(config.listen, "0.0.0.0");
        assert_eq!(config.port, 9000);
        assert_eq!(config.threads, 16);
        assert_eq!(config.chunk_size, 2048);
        assert_eq!(config.enable_upload, true);
        assert_eq!(config.max_upload_size, 5 * 1024 * 1024 * 1024);
        assert_eq!(config.username, Some("testuser".to_string()));
        assert_eq!(config.password, Some("testpass".to_string()));
        assert_eq!(config.allowed_extensions, vec!["*.pdf", "*.doc"]);
        assert_eq!(config.verbose, true);
        assert_eq!(config.detailed_logging, false);
    }

    #[test]
    fn test_config_load_cli_overrides_ini() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.ini");

        let ini_content = r#"
[server]
listen = 0.0.0.0
port = 9000
threads = 16
"#;

        fs::write(&config_file, ini_content).unwrap();

        let mut cli = create_test_cli(temp_dir.path().to_path_buf());
        cli.config_file = Some(config_file.to_string_lossy().to_string());
        cli.listen = "192.168.1.1".to_string();
        cli.port = 7777;
        cli.verbose = true;

        let config = Config::load(&cli).unwrap();

        // CLI should override INI
        assert_eq!(config.listen, "192.168.1.1");
        assert_eq!(config.port, 7777);
        assert_eq!(config.verbose, true);

        // INI should provide non-overridden values
        assert_eq!(config.threads, 16);
    }

    #[test]
    fn test_config_file_discovery_nonexistent() {
        let temp_dir = TempDir::new().unwrap();

        let mut cli = create_test_cli(temp_dir.path().to_path_buf());
        cli.config_file = Some("/nonexistent/path.ini".to_string());

        let result = Config::load(&cli);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Config file specified but not found"));
    }

    #[test]
    fn test_config_upload_settings() {
        let temp_dir = TempDir::new().unwrap();
        let upload_dir = temp_dir.path().join("uploads");
        fs::create_dir_all(&upload_dir).unwrap();

        let config_file = temp_dir.path().join("test.ini");
        let ini_content = format!(
            r#"
[upload]
enabled = true
max_size = 2GB
directory = {}
"#,
            upload_dir.to_string_lossy()
        );

        fs::write(&config_file, ini_content).unwrap();

        let mut cli = create_test_cli(temp_dir.path().to_path_buf());
        cli.config_file = Some(config_file.to_string_lossy().to_string());

        let config = Config::load(&cli).unwrap();

        assert_eq!(config.enable_upload, true);
        assert_eq!(config.max_upload_size, 2 * 1024 * 1024 * 1024);
        assert_eq!(config.upload_dir, Some(upload_dir));
    }

    #[test]
    fn test_config_max_upload_size_formats() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.ini");

        // Temporarily move any irondrop.ini in current directory to avoid interference
        let current_config = PathBuf::from("irondrop.ini");
        let backup_config = PathBuf::from("irondrop.ini.backup");
        let config_existed = if current_config.exists() {
            std::fs::rename(&current_config, &backup_config).ok();
            true
        } else {
            false
        };

        let ini_content = r#"
[upload]
max_size = 1.5GB
"#;

        fs::write(&config_file, ini_content).unwrap();

        let mut cli = create_test_cli(temp_dir.path().to_path_buf());
        cli.config_file = Some(config_file.to_string_lossy().to_string());
        // Set CLI to use default value (10240 MB = 10GB) so INI takes precedence
        cli.max_upload_size = 10240;

        let config = Config::load(&cli).unwrap();

        // 1.5GB should be converted to bytes
        assert_eq!(
            config.max_upload_size,
            (1.5 * 1024.0 * 1024.0 * 1024.0) as u64
        );

        // Restore the config file if it existed
        if config_existed {
            std::fs::rename(&backup_config, &current_config).ok();
        }
    }

    #[test]
    fn test_config_print_summary() {
        let temp_dir = TempDir::new().unwrap();
        let cli = create_test_cli(temp_dir.path().to_path_buf());
        let config = Config::load(&cli).unwrap();

        // This should not panic
        config.print_summary();
    }

    #[test]
    fn test_config_directory_always_from_cli() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.ini");

        // Even if INI has directory, CLI should always win (since it's required)
        let ini_content = r#"
[server]
directory = /some/other/path
"#;
        fs::write(&config_file, ini_content).unwrap();

        let mut cli = create_test_cli(temp_dir.path().to_path_buf());
        cli.config_file = Some(config_file.to_string_lossy().to_string());

        let config = Config::load(&cli).unwrap();

        // Directory should always come from CLI
        assert_eq!(config.directory, temp_dir.path());
    }
}
