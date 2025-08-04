# IronDrop Security Fixes - Upload Configuration v2.5

## Summary

This document outlines the security fixes implemented for the IronDrop CLI configuration to address critical vulnerabilities in file upload handling.

**Status**: All security fixes have been successfully implemented and are active in IronDrop v2.5. The application now includes comprehensive input validation, security boundaries, and defense-in-depth mechanisms.

## Issues Fixed

### 1. Upload Size Validation (OWASP A05:2021 - Security Misconfiguration)

**Problem**: The `max_upload_size` parameter accepted any u64 value (up to 64GB), which could cause:
- Memory exhaustion attacks
- Denial of Service (DoS)
- Resource starvation

**Solution**:
- Added `validate_upload_size()` function with bounds checking (1-10240 MB)
- Prevents setting dangerous values like u64::MAX
- Clear error messages for invalid configurations
- Safe conversion method `max_upload_size_bytes()` for MB to bytes

### 2. Path Traversal Protection (OWASP A01:2021 - Broken Access Control)

**Problem**: The `upload_dir` parameter lacked validation, allowing potential:
- Directory traversal attacks (e.g., `../../etc/`)
- Writing to system directories
- Arbitrary file system access

**Solution**:
- Added `validate_upload_dir()` function with comprehensive path validation:
  - Path canonicalization to resolve `.` and `..` components
  - Absolute path requirement
  - System directory blacklisting (Unix: /etc, /sys, /proc, /dev, /boot)
  - System directory blacklisting (Windows: C:\Windows, C:\Program Files)
  - Write permission verification
  - Parent directory existence check

### 3. Type Safety and Consistency

**Problem**: Inconsistent size types between CLI (u32 MB) and error handling (u64 bytes)

**Solution**:
- Added `max_upload_size_bytes()` method for safe type conversion
- Consistent u64 usage throughout upload handling
- Overflow-safe conversion (limited to 1024 MB max)

## Implementation Details

### CLI Validation Framework

```rust
impl Cli {
    pub fn validate(&self) -> Result<(), AppError>
    pub fn max_upload_size_bytes(&self) -> u64
    pub fn get_upload_directory(&self) -> Result<PathBuf, AppError>
}
```

### Security Features

1. **Input Validation at Parse Time**
   - Clap value_parser integration
   - Immediate feedback for invalid values
   - Prevents invalid configurations from starting

2. **Runtime Validation**
   - Additional checks during server initialization
   - Directory creation with proper permissions
   - Write permission verification

3. **Defense in Depth**
   - Multiple validation layers
   - Clear error messages
   - Logging of security-related events

### Error Handling

- New error variant: `AppError::InvalidConfiguration(String)`
- Descriptive error messages for troubleshooting
- Proper error propagation through the application

## Testing

Comprehensive test suite added:
- `test_validate_upload_size()` - Boundary testing for size limits
- `test_validate_upload_dir()` - Path traversal and system directory tests
- `test_max_upload_size_bytes()` - Type conversion verification
- `test_cli_validate()` - Integration testing
- `test_path_traversal_detection()` - Security-specific tests

## Usage Examples

### Valid Configurations
```bash
# Default configuration (100 MB limit, OS download directory)
irondrop -d /srv/files --enable-upload

# Custom upload directory with size limit
irondrop -d /srv/files --enable-upload --max-upload-size 50 --upload-dir /srv/uploads
```

### Invalid Configurations (Will Be Rejected)
```bash
# Size too large
irondrop -d /srv/files --enable-upload --max-upload-size 2000

# System directory
irondrop -d /srv/files --enable-upload --upload-dir /etc

# Path traversal
irondrop -d /srv/files --enable-upload --upload-dir ../../../tmp
```

## Security Considerations

1. **Resource Limits**: Even with 1GB max, consider system resources and concurrent uploads
2. **Disk Space**: The application performs basic disk space checks but monitoring is recommended
3. **File System Permissions**: Ensure upload directories have appropriate permissions
4. **Rate Limiting**: Consider implementing rate limiting for upload endpoints (future enhancement)

## OWASP References

- [A01:2021 – Broken Access Control](https://owasp.org/Top10/A01_2021-Broken_Access_Control/)
- [A05:2021 – Security Misconfiguration](https://owasp.org/Top10/A05_2021-Security_Misconfiguration/)
- [File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)

## Current Implementation Status (v2.5)

All security features are fully implemented and active:

### ✅ **Implemented Security Controls**
- **Input Validation**: CLI parameter validation with proper error handling
- **Path Traversal Protection**: Comprehensive directory validation in `src/cli.rs:validate_upload_dir()`
- **Size Limits**: Upload size bounds checking (1MB-10GB) with overflow protection
- **Directory Security**: System directory blacklisting for Unix and Windows
- **Write Permission Checks**: Runtime validation of upload directory permissions
- **Configuration Validation**: Server-side validation in `src/lib.rs:run()` at line 46

### 📊 **Test Coverage**
- **CLI Validation Tests**: Comprehensive testing in integration test suites
- **Security Tests**: Path traversal and boundary testing included
- **Error Handling Tests**: Validation of proper error responses
- **Total Tests**: 101+ tests covering all security scenarios

### 🔒 **Production Hardening**
- **Error Messages**: Descriptive but security-conscious error reporting
- **Logging**: Security events logged for audit purposes
- **Fail-Safe Defaults**: Secure defaults with explicit opt-in for features
- **Defense in Depth**: Multiple validation layers throughout the application

## Integration with Core Systems

Security validations integrate with:
- **Upload Handler**: `src/upload.rs` respects CLI security configurations
- **HTTP Layer**: `src/http.rs` enforces security boundaries  
- **Template System**: Upload UI respects security constraints
- **Error System**: Consistent security error handling via `AppError`

## Backward Compatibility

- Existing CLI usage remains unchanged
- Default values preserve current behavior
- Only invalid configurations are rejected
- Security fixes are transparent to valid configurations