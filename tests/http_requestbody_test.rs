// SPDX-License-Identifier: MIT

use irondrop::http::RequestBody;

#[test]
fn test_request_body_len_and_empty() {
    let m = RequestBody::Memory(vec![1, 2, 3]);
    assert_eq!(m.len(), 3);
    assert!(!m.is_empty());

    let m2 = RequestBody::Memory(vec![]);
    assert_eq!(m2.len(), 0);
    assert!(m2.is_empty());

    let f = RequestBody::File {
        path: std::path::PathBuf::from("/tmp/x"),
        size: 10,
    };
    assert_eq!(f.len(), 10);
    assert!(!f.is_empty());
}
