/// # IronDrop
///
/// A lightweight, configurable file download server written in Rust.
///
/// This library contains the core logic for the server. The `run` function
/// initializes and starts the server based on command-line arguments.
pub mod cli;
pub mod error;
pub mod fs;
pub mod http;
pub mod multipart;
pub mod response;
pub mod server;
pub mod templates;
pub mod upload;
pub mod utils;

use crate::cli::Cli;
use clap::Parser;
use log::error;

/// Initializes the logger, parses command-line arguments, and starts the server.
///
/// This is the main entry point for the application. It sets up the logging
/// framework and then calls the `run_server` function to start the server.
/// If the server returns an error, it is logged and the process exits.
pub fn run() {
    let cli = Cli::parse();

    let log_level = if cli.verbose {
        "debug"
    } else if cli.detailed_logging {
        "info"
    } else {
        "warn"
    };

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", log_level);
    }
    env_logger::init();

    log::debug!("Log level set to: {log_level}");

    // Validate CLI configuration before starting the server
    if let Err(e) = cli.validate() {
        error!("Configuration validation error: {e}");
        std::process::exit(1);
    }

    if let Err(e) = server::run_server(cli, None, None) {
        error!("Server error: {e}");
        std::process::exit(1);
    }
}
