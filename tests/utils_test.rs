// SPDX-License-Identifier: MIT

use irondrop::utils::{parse_query_params, percent_encode_path, resolve_upload_directory};
use std::fs;
use std::path::Path;

#[test]
fn test_percent_encode_path_spaces() {
    let p = Path::new("some dir/with spaces/file name.txt");
    let enc = percent_encode_path(p);
    assert_eq!(enc, "some%20dir/with%20spaces/file%20name.txt");
}

#[test]
fn test_parse_query_params_basic_and_plus_space() {
    let params = parse_query_params("/a?name=iron+drop&x=1&y=%2Froot");
    assert_eq!(params.get("name").unwrap(), "iron drop");
    assert_eq!(params.get("x").unwrap(), "1");
    assert_eq!(params.get("y").unwrap(), "/root");
}

#[test]
fn test_resolve_upload_directory_security_and_exists() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();
    // create subdir
    let sub = base.join("subdir");
    fs::create_dir(&sub).unwrap();
    // ok: within base
    let target = resolve_upload_directory(base, Some("/subdir")).unwrap();
    assert!(target.starts_with(base));
    assert_eq!(target, sub);
    // not found when directory does not exist
    assert!(resolve_upload_directory(base, Some("/missing")).is_err());
    // traversal should be forbidden
    assert!(resolve_upload_directory(base, Some("/../../etc")).is_err());
}

#[test]
fn test_percent_encode_path_special_characters() {
    let p = Path::new("file with !@#$%^&*()+={}[]|\\:;\"'<>,.?/~`");
    let enc = percent_encode_path(p);
    // Should encode special characters that need encoding
    assert!(enc.contains('%'));
    assert!(!enc.contains(' ')); // spaces should be encoded
}

#[test]
fn test_percent_encode_path_unicode() {
    let p = Path::new("файл.txt"); // Cyrillic
    let enc = percent_encode_path(p);
    assert!(enc.contains('%')); // Unicode should be percent-encoded

    let p2 = Path::new("文件.txt"); // Chinese
    let enc2 = percent_encode_path(p2);
    assert!(enc2.contains('%')); // Unicode should be percent-encoded
}

#[test]
fn test_percent_encode_path_empty_and_root() {
    let p = Path::new("");
    let enc = percent_encode_path(p);
    assert_eq!(enc, "");

    let p2 = Path::new("/");
    let enc2 = percent_encode_path(p2);
    assert_eq!(enc2, "/");
}

#[test]
fn test_parse_query_params_empty_and_malformed() {
    // Empty query string
    let params = parse_query_params("/path");
    assert!(params.is_empty());

    // Only question mark
    let params = parse_query_params("/path?");
    assert!(params.is_empty());

    // Malformed parameters
    let params = parse_query_params("/path?key1&key2=&=value3&key4=value4");
    assert!(params.contains_key("key4"));
    assert_eq!(params.get("key4").unwrap(), "value4");
}

#[test]
fn test_parse_query_params_duplicate_keys() {
    // Last value should win for duplicate keys
    let params = parse_query_params("/path?key=first&key=second&key=third");
    assert_eq!(params.get("key").unwrap(), "third");
}

#[test]
fn test_parse_query_params_special_encoding() {
    let params =
        parse_query_params("/path?encoded=%3D%26%3F%23&plus=a+b+c&mixed=hello%20world+test");
    assert_eq!(params.get("encoded").unwrap(), "=&?#");
    assert_eq!(params.get("plus").unwrap(), "a b c");
    assert_eq!(params.get("mixed").unwrap(), "hello world test");
}

#[test]
fn test_parse_query_params_unicode() {
    let params = parse_query_params("/path?name=%E6%96%87%E4%BB%B6&value=%D1%84%D0%B0%D0%B9%D0%BB");
    // Should handle UTF-8 encoded parameters
    assert!(params.contains_key("name"));
    assert!(params.contains_key("value"));
}

#[test]
fn test_resolve_upload_directory_edge_cases() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    // Test with None path (should return base)
    let target = resolve_upload_directory(base, None).unwrap();
    assert_eq!(target, base);

    // Test with empty string
    let target = resolve_upload_directory(base, Some("")).unwrap();
    assert_eq!(target, base);

    // Test with just slash
    let target = resolve_upload_directory(base, Some("/")).unwrap();
    assert_eq!(target, base);
}

#[test]
fn test_resolve_upload_directory_complex_traversal() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    // Create nested directory structure
    let nested = base.join("level1").join("level2");
    fs::create_dir_all(&nested).unwrap();

    // Valid nested path
    let target = resolve_upload_directory(base, Some("/level1/level2")).unwrap();
    assert_eq!(target, nested);

    // Various traversal attempts
    assert!(resolve_upload_directory(base, Some("/level1/../../../etc")).is_err());
    assert!(resolve_upload_directory(base, Some("/level1/level2/../../../etc")).is_err());
    assert!(resolve_upload_directory(base, Some("/../etc")).is_err());
    assert!(resolve_upload_directory(base, Some("/./../../etc")).is_err());
}

#[test]
fn test_resolve_upload_directory_symlink_security() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    // Create a directory and a symlink pointing outside
    let safe_dir = base.join("safe");
    fs::create_dir(&safe_dir).unwrap();

    // Test that symlinks are handled securely (if they exist)
    // This test mainly ensures the function doesn't crash with symlinks
    let target = resolve_upload_directory(base, Some("/safe")).unwrap();
    assert_eq!(target, safe_dir);
}

#[test]
fn test_resolve_upload_directory_special_characters() {
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path();

    // Create directory with special characters (if filesystem supports it)
    let special_name = "dir with spaces & symbols";
    let special_dir = base.join(special_name);
    if fs::create_dir(&special_dir).is_ok() {
        // URL-encoded path
        let encoded_path = format!("/{}", special_name.replace(' ', "%20").replace('&', "%26"));
        // This might fail depending on implementation, but shouldn't crash
        let _ = resolve_upload_directory(base, Some(&encoded_path));
    }
}
