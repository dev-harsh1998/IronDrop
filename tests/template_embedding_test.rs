use irondrop::templates::TemplateEngine;
use std::collections::HashMap;

/// Test that templates are properly embedded and can render without filesystem access
#[test]
fn test_embedded_templates_functionality() {
    let engine = TemplateEngine::new();

    // Test directory listing template rendering
    let mut variables = HashMap::new();
    variables.insert("PATH".to_string(), "/test/path".to_string());
    variables.insert("ENTRY_COUNT".to_string(), "5".to_string());
    variables.insert(
        "ENTRIES".to_string(),
        "<tr><td>test file</td></tr>".to_string(),
    );
    variables.insert("UPLOAD_ENABLED".to_string(), "false".to_string());
    variables.insert("CURRENT_PATH".to_string(), "/test/path".to_string());

    let result = engine.render_directory_page(&variables);
    assert!(
        result.is_ok(),
        "Directory template should render successfully"
    );

    let html = result.unwrap();
    assert!(
        html.contains("test/path"),
        "Should contain the cleaned path variable (without leading slash)"
    );
    assert!(html.contains("test file"), "Should contain the entries");
    assert!(
        html.contains("/_irondrop/static/directory/styles.css"),
        "Should reference embedded CSS"
    );
    assert!(
        html.contains("/_irondrop/static/directory/script.js"),
        "Should reference embedded JS"
    );

    // Test error page template rendering
    let error_result =
        engine.render_error_page_new(404, "Not Found", "The requested resource was not found.");
    assert!(
        error_result.is_ok(),
        "Error template should render successfully"
    );

    let error_html = error_result.unwrap();
    assert!(error_html.contains("404"), "Should contain the status code");
    assert!(
        error_html.contains("Not Found"),
        "Should contain the status text"
    );
    assert!(
        error_html.contains("/_irondrop/static/error/styles.css"),
        "Should reference embedded error CSS"
    );
    assert!(
        error_html.contains("/_irondrop/static/error/script.js"),
        "Should reference embedded error JS"
    );
}

/// Test static asset retrieval
#[test]
fn test_embedded_static_assets() {
    let engine = TemplateEngine::new();

    // Test directory CSS
    let css = engine.get_static_asset("directory/styles.css");
    assert!(css.is_some(), "Directory CSS should be available");
    let (css_content, css_type) = css.unwrap();
    assert_eq!(css_type, "text/css");
    assert!(css_content.contains("Professional Blackish Grey Design"));

    // Test base CSS (contains the CSS variables)
    let base_css = engine.get_static_asset("common/base.css");
    assert!(base_css.is_some(), "Base CSS should be available");
    let (base_css_content, base_css_type) = base_css.unwrap();
    assert_eq!(base_css_type, "text/css");
    assert!(base_css_content.contains("--bg-primary: #0a0a0a"));

    // Test directory JS
    let js = engine.get_static_asset("directory/script.js");
    assert!(js.is_some(), "Directory JS should be available");
    let (js_content, js_type) = js.unwrap();
    assert_eq!(js_type, "application/javascript");
    assert!(js_content.contains("DOMContentLoaded"));
    assert!(js_content.contains("loading animation"));

    // Test error CSS
    let error_css = engine.get_static_asset("error/styles.css");
    assert!(error_css.is_some(), "Error CSS should be available");
    let (error_css_content, error_css_type) = error_css.unwrap();
    assert_eq!(error_css_type, "text/css");
    assert!(error_css_content.contains("Professional Blackish Grey Error Page Design"));

    // Test error JS
    let error_js = engine.get_static_asset("error/script.js");
    assert!(error_js.is_some(), "Error JS should be available");
    let (error_js_content, error_js_type) = error_js.unwrap();
    assert_eq!(error_js_type, "application/javascript");
    assert!(error_js_content.contains("Keyboard shortcuts"));

    // Test non-existent asset
    let nonexistent = engine.get_static_asset("nonexistent/file.css");
    assert!(
        nonexistent.is_none(),
        "Non-existent asset should return None"
    );
}

/// Test directory listing rendering with embedded templates
#[test]
fn test_directory_listing_rendering() {
    let engine = TemplateEngine::new();

    let test_entries = vec![
        (
            "file1.txt".to_string(),
            "1.2 KB".to_string(),
            "2 hours ago".to_string(),
        ),
        (
            "directory/".to_string(),
            "-".to_string(),
            "1 day ago".to_string(),
        ),
        (
            "file2.zip".to_string(),
            "45.8 MB".to_string(),
            "3 days ago".to_string(),
        ),
    ];

    let result =
        engine.render_directory_listing("/downloads", &test_entries, 3, false, "/downloads");
    assert!(
        result.is_ok(),
        "Directory listing should render successfully"
    );

    let html = result.unwrap();

    // Should contain all test entries
    assert!(html.contains("file1.txt"), "Should contain file1.txt");
    assert!(html.contains("directory/"), "Should contain directory/");
    assert!(html.contains("file2.zip"), "Should contain file2.zip");
    assert!(html.contains("1.2 KB"), "Should contain file sizes");
    assert!(html.contains("45.8 MB"), "Should contain large file size");

    // Should contain proper HTML structure
    assert!(html.contains("<table"), "Should contain table structure");
    assert!(html.contains("file-link"), "Should contain styled links");
    assert!(
        html.contains("file-type directory"),
        "Should identify directories"
    );
    assert!(html.contains("file-type file"), "Should identify files");

    // Should reference embedded assets
    assert!(
        html.contains("/_irondrop/static/directory/styles.css"),
        "Should reference CSS"
    );
    assert!(
        html.contains("/_irondrop/static/directory/script.js"),
        "Should reference JS"
    );
}

/// Test error page rendering with embedded templates
#[test]
fn test_error_page_rendering() {
    let engine = TemplateEngine::new();

    let result = engine.render_error_page(404, "Not Found", "The requested file was not found");
    assert!(result.is_ok(), "Error page should render successfully");

    let html = result.unwrap();

    // Should contain error information
    assert!(html.contains("404"), "Should contain status code");
    assert!(html.contains("Not Found"), "Should contain status text");
    assert!(
        html.contains("The requested file was not found"),
        "Should contain description"
    );

    // Should contain proper HTML structure
    assert!(
        html.contains("error-container"),
        "Should contain error container"
    );
    assert!(
        html.contains("error-code"),
        "Should contain error code styling"
    );
    assert!(html.contains("Go Back"), "Should contain back link");

    // Should reference embedded assets
    assert!(
        html.contains("/_irondrop/static/error/styles.css"),
        "Should reference error CSS"
    );
    assert!(
        html.contains("/_irondrop/static/error/script.js"),
        "Should reference error JS"
    );
}

/// Test that favicon files are properly embedded and accessible
#[test]
fn test_embedded_favicon_functionality() {
    let engine = TemplateEngine::new();

    // Test favicon.ico
    let (favicon_ico, content_type_ico) = engine
        .get_favicon("favicon.ico")
        .expect("favicon.ico should be embedded");
    assert_eq!(content_type_ico, "image/x-icon");
    assert!(!favicon_ico.is_empty(), "favicon.ico should have content");

    // Test favicon-16x16.png
    let (favicon_16, content_type_16) = engine
        .get_favicon("favicon-16x16.png")
        .expect("favicon-16x16.png should be embedded");
    assert_eq!(content_type_16, "image/png");
    assert!(!favicon_16.is_empty(), "16x16 PNG should have content");

    // Test favicon-32x32.png
    let (favicon_32, content_type_32) = engine
        .get_favicon("favicon-32x32.png")
        .expect("favicon-32x32.png should be embedded");
    assert_eq!(content_type_32, "image/png");
    assert!(!favicon_32.is_empty(), "32x32 PNG should have content");

    // Test PNG file signature validation
    assert_eq!(
        &favicon_16[0..4],
        &[137, 80, 78, 71],
        "16x16 PNG should have valid signature"
    );
    assert_eq!(
        &favicon_32[0..4],
        &[137, 80, 78, 71],
        "32x32 PNG should have valid signature"
    );

    // Test non-existent favicon
    assert!(
        engine.get_favicon("nonexistent.ico").is_none(),
        "Should return None for non-existent favicon"
    );
}
