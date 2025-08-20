// Allow some clippy lints for tests and debug code
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::useless_format)]
#![allow(clippy::needless_as_bytes)]
#![allow(clippy::expect_fun_call)]
#![allow(clippy::items_after_test_module)]
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(clippy::assertions_on_result_states)]
#![allow(clippy::needless_return)]
#![allow(clippy::redundant_closure_for_method_calls)]

/// # IronDrop
///
/// A lightweight, configurable file download server written in Rust.
///
/// This library contains the core logic for the server. The `run` function
/// initializes and starts the server based on command-line arguments.
pub mod cli;
pub mod config;
pub mod error;
pub mod fs;
pub mod handlers;
pub mod http;
pub mod middleware;
pub mod response;
pub mod router;
pub mod search;
pub mod server;
pub mod templates;
pub mod ultra_compact_search;
#[cfg(test)]
pub mod ultra_memory_test;
pub mod upload;
pub mod utils;

use crate::cli::Cli;
use crate::config::Config;
use clap::Parser;
use log::error;
use std::fs::OpenOptions;

/// Initializes the logger, parses command-line arguments, and starts the server.
///
/// This is the main entry point for the application. It sets up the logging
/// framework and then calls the `run_server` function to start the server.
/// If the server returns an error, it is logged and the process exits.
pub fn run() {
    let cli = Cli::parse();

    // Load configuration with precedence: CLI > ENV > INI > Defaults
    let config = match Config::load(&cli) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration error: {e}");
            std::process::exit(1);
        }
    };

    let log_level = if config.verbose {
        "debug"
    } else if config.detailed_logging {
        "info"
    } else {
        "warn"
    };

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", log_level);
    }

    // Initialize logging with optional file output
    if let Some(ref log_file_path) = config.log_file {
        init_file_logger(log_file_path).unwrap_or_else(|e| {
            eprintln!("Failed to initialize file logger: {e}");
            std::process::exit(1);
        });
    } else {
        env_logger::init();
    }

    log::debug!("Log level set to: {log_level}");

    // Print configuration summary in debug mode
    if config.verbose {
        config.print_summary();
    }

    // Validate CLI configuration before starting the server
    if let Err(e) = cli.validate() {
        error!("Configuration validation error: {e}");
        std::process::exit(1);
    }

    if let Err(e) = server::run_server_with_config(config) {
        error!("Server error: {e}");
        std::process::exit(1);
    }
}

/// Initialize file-based logging
fn init_file_logger(log_file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use env_logger::Builder;

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path)?;

    Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    Ok(())
}
