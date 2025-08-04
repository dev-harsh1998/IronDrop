use clap::Parser;
use std::path::PathBuf;

// Defines the command-line interface using clap. 🎉
// This struct represents the structure of arguments you can pass when running the server.
#[derive(Parser)]
#[command(
     author = "Harshit Jain",
     version = "2.0.0", //  Version of our IronDrop - feels like we're shipping software! 🚢
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
}
