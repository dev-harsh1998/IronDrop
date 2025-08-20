// SPDX-License-Identifier: MIT

use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Glob(glob::PatternError),
    AddrParse(std::net::AddrParseError),
    InvalidPath,
    DirectoryNotFound(String),
    Forbidden,
    NotFound,
    BadRequest,
    Unauthorized,
    MethodNotAllowed,
    InternalServerError(String),
    // Upload-specific errors
    PayloadTooLarge(u64),         // Contains the maximum allowed size
    InvalidFilename(String),      // Contains the problematic filename
    UploadDiskFull(u64),          // Contains available space in bytes
    UnsupportedMediaType(String), // Contains the rejected media type
    UploadDisabled,
    InvalidConfiguration(String), // Contains configuration error details
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "IO error: {err}"),
            AppError::Glob(err) => write!(f, "Glob pattern error: {err}"),
            AppError::AddrParse(err) => write!(f, "Address parse error: {err}"),
            AppError::InvalidPath => write!(f, "Invalid path"),
            AppError::DirectoryNotFound(path) => write!(f, "Directory not found: {path}"),
            AppError::Forbidden => write!(f, "Forbidden"),
            AppError::NotFound => write!(f, "Not Found"),
            AppError::BadRequest => write!(f, "Bad request"),
            AppError::Unauthorized => write!(f, "Unauthorized"),
            AppError::MethodNotAllowed => write!(f, "Method not allowed"),
            AppError::InternalServerError(msg) => write!(f, "Internal server error: {msg}"),
            AppError::PayloadTooLarge(max_size) => {
                write!(
                    f,
                    "Upload payload too large. Maximum allowed size: {max_size} bytes"
                )
            }
            AppError::InvalidFilename(filename) => {
                write!(
                    f,
                    "Invalid filename '{filename}': contains illegal characters or path traversal"
                )
            }
            AppError::UploadDiskFull(available_bytes) => {
                write!(
                    f,
                    "Insufficient disk space for upload. Available: {available_bytes} bytes"
                )
            }
            AppError::UnsupportedMediaType(media_type) => {
                write!(
                    f,
                    "Unsupported media type '{media_type}': file type not allowed"
                )
            }
            AppError::UploadDisabled => write!(f, "Upload functionality is disabled"),
            AppError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {msg}"),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<glob::PatternError> for AppError {
    fn from(err: glob::PatternError) -> Self {
        AppError::Glob(err)
    }
}

impl From<std::net::AddrParseError> for AppError {
    fn from(err: std::net::AddrParseError) -> Self {
        AppError::AddrParse(err)
    }
}

impl std::error::Error for AppError {}

// Additional utility methods for upload error handling
impl AppError {
    /// Creates a PayloadTooLarge error with the maximum allowed size
    pub fn payload_too_large(max_size: u64) -> Self {
        AppError::PayloadTooLarge(max_size)
    }

    /// Creates an InvalidFilename error
    pub fn invalid_filename<S: Into<String>>(filename: S) -> Self {
        AppError::InvalidFilename(filename.into())
    }

    /// Creates an UploadDiskFull error with available space
    pub fn upload_disk_full(available_bytes: u64) -> Self {
        AppError::UploadDiskFull(available_bytes)
    }

    /// Creates an UnsupportedMediaType error
    pub fn unsupported_media_type<S: Into<String>>(media_type: S) -> Self {
        AppError::UnsupportedMediaType(media_type.into())
    }

    /// Creates an UploadDisabled error
    pub fn upload_disabled() -> Self {
        AppError::UploadDisabled
    }

    /// Checks if the error is upload-related
    pub fn is_upload_error(&self) -> bool {
        matches!(
            self,
            AppError::PayloadTooLarge(_)
                | AppError::InvalidFilename(_)
                | AppError::UploadDiskFull(_)
                | AppError::UnsupportedMediaType(_)
                | AppError::UploadDisabled
                | AppError::InvalidConfiguration(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_error_display() {
        let errors = [
            AppError::payload_too_large(1024),
            AppError::invalid_filename("invalid.txt"),
            AppError::invalid_filename("../../../etc/passwd"),
            AppError::upload_disk_full(512),
            AppError::unsupported_media_type("application/x-executable"),
            AppError::upload_disabled(),
        ];

        let expected = [
            "Upload payload too large. Maximum allowed size: 1024 bytes",
            "Invalid filename 'invalid.txt': contains illegal characters or path traversal",
            "Invalid filename '../../../etc/passwd': contains illegal characters or path traversal",
            "Insufficient disk space for upload. Available: 512 bytes",
            "Unsupported media type 'application/x-executable': file type not allowed",
            "Upload functionality is disabled",
        ];

        for (error, expected_msg) in errors.iter().zip(expected.iter()) {
            assert_eq!(error.to_string(), *expected_msg);
        }
    }

    #[test]
    fn test_is_upload_error() {
        let upload_errors = vec![
            AppError::payload_too_large(1024),
            AppError::invalid_filename("test"),
            AppError::invalid_filename("test"),
            AppError::upload_disk_full(512),
            AppError::unsupported_media_type("test"),
            AppError::upload_disabled(),
        ];

        let non_upload_errors = vec![
            AppError::NotFound,
            AppError::Forbidden,
            AppError::BadRequest,
            AppError::InternalServerError("test".to_string()),
        ];

        for error in upload_errors {
            assert!(
                error.is_upload_error(),
                "Expected {error} to be an upload error"
            );
        }

        for error in non_upload_errors {
            assert!(
                !error.is_upload_error(),
                "Expected {error} to not be an upload error"
            );
        }
    }

    #[test]
    fn test_error_trait_implementation() {
        let error = AppError::payload_too_large(1024);
        let _: &dyn std::error::Error = &error; // This ensures Error trait is implemented
    }
}
