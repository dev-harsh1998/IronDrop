//! Simple INI file parser with zero dependencies
//! Supports sections, key-value pairs, comments, and basic data types

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct IniConfig {
    sections: HashMap<String, HashMap<String, String>>,
    global: HashMap<String, String>,
}

impl Default for IniConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl IniConfig {
    pub fn new() -> Self {
        Self {
            sections: HashMap::new(),
            global: HashMap::new(),
        }
    }

    /// Load configuration from file
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {e}"))?;
        Self::parse(&content)
    }

    /// Parse INI content from string
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut config = Self::new();
        let mut current_section = String::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            let line_number = line_num + 1;

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            // Parse section headers [section]
            if line.starts_with('[') && line.ends_with(']') {
                if line.len() < 3 {
                    return Err(format!("Invalid section at line {line_number}: {line}"));
                }
                current_section = line[1..line.len() - 1].trim().to_string();
                if current_section.is_empty() {
                    return Err(format!("Empty section name at line {line_number}"));
                }
                config.sections.entry(current_section.clone()).or_default();
                continue;
            } else if line.starts_with('[') {
                // Malformed section header - ignore it gracefully
                continue;
            }

            // Parse key=value pairs
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let mut value = line[eq_pos + 1..].trim();

                if key.is_empty() {
                    return Err(format!("Empty key at line {line_number}: {line}"));
                }

                // Handle inline comments - remove everything after # or ;
                if let Some(comment_pos) = value.find('#') {
                    value = value[..comment_pos].trim();
                } else if let Some(comment_pos) = value.find(';') {
                    value = value[..comment_pos].trim();
                }

                let key = key.to_string();
                let value = value.to_string();

                if current_section.is_empty() {
                    // Global section
                    config.global.insert(key, value);
                } else {
                    // Named section
                    config
                        .sections
                        .get_mut(&current_section)
                        .unwrap()
                        .insert(key, value);
                }
            } else {
                return Err(format!("Invalid syntax at line {line_number}: {line}"));
            }
        }

        Ok(config)
    }

    /// Get string value
    pub fn get_string(&self, section: &str, key: &str) -> Option<String> {
        if section.is_empty() {
            self.global.get(key).cloned()
        } else {
            self.sections.get(section)?.get(key).cloned()
        }
    }

    /// Get string value with default fallback
    #[allow(dead_code)]
    pub fn get_string_or(&self, section: &str, key: &str, default: &str) -> String {
        self.get_string(section, key)
            .unwrap_or_else(|| default.to_string())
    }

    /// Get integer value
    pub fn get_u16(&self, section: &str, key: &str) -> Option<u16> {
        self.get_string(section, key)?.parse().ok()
    }

    #[allow(dead_code)]
    pub fn get_u64(&self, section: &str, key: &str) -> Option<u64> {
        self.get_string(section, key)?.parse().ok()
    }

    pub fn get_usize(&self, section: &str, key: &str) -> Option<usize> {
        self.get_string(section, key)?.parse().ok()
    }

    /// Get boolean value
    pub fn get_bool(&self, section: &str, key: &str) -> Option<bool> {
        match self.get_string(section, key)?.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Some(true),
            "false" | "no" | "0" | "off" => Some(false),
            _ => None,
        }
    }

    pub fn get_bool_or(&self, section: &str, key: &str, default: bool) -> bool {
        self.get_bool(section, key).unwrap_or(default)
    }

    /// Get comma-separated list
    pub fn get_list(&self, section: &str, key: &str) -> Vec<String> {
        self.get_string(section, key)
            .map(|s| {
                s.split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Parse file size (supports KB, MB, GB suffixes)
    pub fn get_file_size(&self, section: &str, key: &str) -> Option<u64> {
        let value = self.get_string(section, key)?;
        parse_file_size(&value)
    }

    /// Check if section exists
    #[allow(dead_code)]
    pub fn has_section(&self, section: &str) -> bool {
        self.sections.contains_key(section)
    }

    /// Check if key exists
    #[allow(dead_code)]
    pub fn has_key(&self, section: &str, key: &str) -> bool {
        if section.is_empty() {
            self.global.contains_key(key)
        } else {
            self.sections
                .get(section)
                .map(|s| s.contains_key(key))
                .unwrap_or(false)
        }
    }

    /// Get all section names
    #[allow(dead_code)]
    pub fn sections(&self) -> Vec<String> {
        self.sections.keys().cloned().collect()
    }
}

/// Helper function to parse file sizes like "10GB", "500MB", etc.
fn parse_file_size(value: &str) -> Option<u64> {
    let value = value.trim().to_uppercase();

    if let Ok(num) = value.parse::<u64>() {
        return Some(num);
    }

    let (num_part, suffix) = if value.ends_with("TB") {
        (value.strip_suffix("TB")?, 1024u64 * 1024 * 1024 * 1024)
    } else if value.ends_with("GB") {
        (value.strip_suffix("GB")?, 1024 * 1024 * 1024)
    } else if value.ends_with("MB") {
        (value.strip_suffix("MB")?, 1024 * 1024)
    } else if value.ends_with("KB") {
        (value.strip_suffix("KB")?, 1024)
    } else if value.ends_with("B") {
        (value.strip_suffix("B")?, 1)
    } else {
        return None;
    };

    let num_str = num_part.trim();

    // Try parsing as integer first
    if let Ok(num) = num_str.parse::<u64>() {
        return Some(num * suffix);
    }

    // Try parsing as float for decimal values like "1.5"
    if let Ok(num) = num_str.parse::<f64>() {
        return Some((num * suffix as f64) as u64);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_file_size() {
        assert_eq!(parse_file_size("1024"), Some(1024));
        assert_eq!(parse_file_size("1KB"), Some(1024));
        assert_eq!(parse_file_size("1MB"), Some(1024 * 1024));
        assert_eq!(parse_file_size("10GB"), Some(10 * 1024 * 1024 * 1024));
        assert_eq!(
            parse_file_size("2TB"),
            Some(2 * 1024u64 * 1024 * 1024 * 1024)
        );
        assert_eq!(
            parse_file_size("1.5GB"),
            Some((1.5 * 1024.0 * 1024.0 * 1024.0) as u64)
        );
        assert_eq!(
            parse_file_size("2.5MB"),
            Some((2.5 * 1024.0 * 1024.0) as u64)
        );
        assert_eq!(parse_file_size("invalid"), None);
    }

    #[test]
    fn test_ini_parsing() {
        let content = r#"
# Global config
debug=true

[server]
host=127.0.0.1
port=8080

[upload]
enabled=true
max_size=10GB
        "#;

        let config = IniConfig::parse(content).unwrap();
        assert_eq!(config.get_bool("", "debug"), Some(true));
        assert_eq!(
            config.get_string("server", "host"),
            Some("127.0.0.1".to_string())
        );
        assert_eq!(config.get_u16("server", "port"), Some(8080));
        assert_eq!(
            config.get_file_size("upload", "max_size"),
            Some(10 * 1024 * 1024 * 1024)
        );
    }

    #[test]
    fn test_boolean_parsing() {
        let content = r#"
[test]
true1=true
true2=yes
true3=1
true4=on
false1=false
false2=no
false3=0
false4=off
        "#;

        let config = IniConfig::parse(content).unwrap();
        assert_eq!(config.get_bool("test", "true1"), Some(true));
        assert_eq!(config.get_bool("test", "true2"), Some(true));
        assert_eq!(config.get_bool("test", "true3"), Some(true));
        assert_eq!(config.get_bool("test", "true4"), Some(true));
        assert_eq!(config.get_bool("test", "false1"), Some(false));
        assert_eq!(config.get_bool("test", "false2"), Some(false));
        assert_eq!(config.get_bool("test", "false3"), Some(false));
        assert_eq!(config.get_bool("test", "false4"), Some(false));
    }

    #[test]
    fn test_list_parsing() {
        let content = r#"
[extensions]
allowed=jpg,png,pdf,txt
        "#;

        let config = IniConfig::parse(content).unwrap();
        let list = config.get_list("extensions", "allowed");
        assert_eq!(list, vec!["jpg", "png", "pdf", "txt"]);
    }
}
