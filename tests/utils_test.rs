// SPDX-License-Identifier: MIT

use irondrop::utils::{parse_query_params, percent_encode_path, resolve_upload_directory};
use std::fs;
use std::path::{Path, PathBuf};

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
