// SPDX-License-Identifier: MIT

use irondrop::templates::TemplateEngine;

#[test]
fn test_directory_listing_escapes_and_percent_encodes() {
    let engine = TemplateEngine::new();

    let entries = vec![
        (
            "my file \"weird\"&<.txt".to_string(),
            "123 B".to_string(),
            "now".to_string(),
        ),
        (
            "dir with space/".to_string(),
            "-".to_string(),
            "today".to_string(),
        ),
    ];

    let html = engine
        .render_directory_listing("/downloads", &entries, entries.len(), false, "/downloads")
        .expect("render ok");

    // Visible name should be HTML-escaped
    assert!(html.contains("my file &quot;weird&quot;&amp;&lt;.txt"));
    // Href should be absolute from current path and percent-encode spaces and quotes (ampersand is not percent-encoded in current impl)
    assert!(html.contains("href=\"/downloads/my%20file%20%22weird%22&%3C.txt\""));
    // Directory entry name should be displayed without trailing slash
    assert!(html.contains(">dir with space<"));
    // Directory link should include trailing slash in href encoding result and be absolute
    assert!(html.contains("href=\"/downloads/dir%20with%20space/\""));
}
