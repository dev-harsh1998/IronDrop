use irondrop::utils::is_hidden_file;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_is_hidden_file_dot_files() {
    // Test files starting with a single dot
    assert!(is_hidden_file(".gitignore"));
    assert!(is_hidden_file(".env"));
    assert!(is_hidden_file(".git"));
    assert!(is_hidden_file(".vscode"));
    assert!(is_hidden_file(".idea"));
    assert!(is_hidden_file(".bashrc"));
    assert!(is_hidden_file(".zshrc"));
}

#[test]
fn test_is_hidden_file_underscore_dot_files() {
    // Test files starting with '._'
    assert!(is_hidden_file("._file.txt"));
    assert!(is_hidden_file("._resource_fork"));
    assert!(is_hidden_file("._document.pdf"));
}

#[test]
fn test_is_hidden_file_ds_store() {
    // Test .DS_Store file
    assert!(is_hidden_file(".DS_Store"));
}

#[test]
fn test_is_hidden_file_visible_files() {
    // Test that normal files are not hidden
    assert!(!is_hidden_file("README.md"));
    assert!(!is_hidden_file("main.rs"));
    assert!(!is_hidden_file("config.toml"));
    assert!(!is_hidden_file("document.pdf"));
    assert!(!is_hidden_file("image.png"));
    assert!(!is_hidden_file("folder"));
    assert!(!is_hidden_file("test_file.txt"));
}

#[test]
fn test_is_hidden_file_edge_cases() {
    // Test edge cases
    assert!(!is_hidden_file(""));
    assert!(is_hidden_file("."));
    assert!(is_hidden_file(".."));
    assert!(!is_hidden_file("file.hidden"));
    assert!(!is_hidden_file("dot.file"));
    assert!(!is_hidden_file("_underscore_file"));
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use irondrop::fs::generate_directory_listing;
    use std::fs::File;

    #[test]
    fn test_directory_listing_filters_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files - both visible and hidden
        let visible_files = vec!["README.md", "main.rs", "config.toml"];
        let hidden_files = vec![".gitignore", ".env", "._resource", ".DS_Store"];

        // Create visible files
        for file in &visible_files {
            let file_path = temp_path.join(file);
            File::create(&file_path).unwrap();
        }

        // Create hidden files
        for file in &hidden_files {
            let file_path = temp_path.join(file);
            File::create(&file_path).unwrap();
        }

        // Create hidden directory
        let hidden_dir = temp_path.join(".git");
        fs::create_dir(&hidden_dir).unwrap();

        // Generate directory listing HTML
        let listing_result = generate_directory_listing(temp_path, "/", None);
        assert!(listing_result.is_ok());

        let listing_html = listing_result.unwrap();

        // Check that visible files are included in the HTML
        for visible_file in &visible_files {
            assert!(
                listing_html.contains(visible_file),
                "Visible file {} should be in listing HTML",
                visible_file
            );
        }

        // Check that hidden files are excluded from the HTML
        for hidden_file in &hidden_files {
            assert!(
                !listing_html.contains(hidden_file),
                "Hidden file {} should not be in listing HTML",
                hidden_file
            );
        }

        // Check that hidden directory is excluded
        assert!(
            !listing_html.contains(".git"),
            "Hidden directory .git should not be in listing HTML"
        );
    }

    #[test]
    fn test_directory_listing_includes_visible_directories() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test directories - both visible and hidden
        let visible_dirs = vec!["src", "tests", "docs"];
        let hidden_dirs = vec![".git", ".vscode", ".idea"];

        // Create visible directories
        for dir in &visible_dirs {
            let dir_path = temp_path.join(dir);
            fs::create_dir(&dir_path).unwrap();
        }

        // Create hidden directories
        for dir in &hidden_dirs {
            let dir_path = temp_path.join(dir);
            fs::create_dir(&dir_path).unwrap();
        }

        // Generate directory listing HTML
        let listing_result = generate_directory_listing(temp_path, "/", None);
        assert!(listing_result.is_ok());

        let listing_html = listing_result.unwrap();

        // Check that visible directories are included in the HTML
        for visible_dir in &visible_dirs {
            assert!(
                listing_html.contains(&format!("{}/", visible_dir)),
                "Visible directory {} should be in listing HTML",
                visible_dir
            );
        }

        // Check that hidden directories are excluded from the HTML
        for hidden_dir in &hidden_dirs {
            assert!(
                !listing_html.contains(&format!("{}/", hidden_dir)),
                "Hidden directory {} should not be in listing HTML",
                hidden_dir
            );
        }
    }

    #[test]
    fn test_search_filters_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test files - both visible and hidden
        let visible_files = vec!["document.pdf", "readme.txt", "config.json"];
        let hidden_files = vec![".gitignore", ".env", "._backup", ".DS_Store"];

        // Create visible files with some content
        for file in &visible_files {
            let file_path = temp_path.join(file);
            File::create(&file_path).unwrap();
        }

        // Create hidden files
        for file in &hidden_files {
            let file_path = temp_path.join(file);
            File::create(&file_path).unwrap();
        }

        // Create subdirectory with mixed files
        let subdir = temp_path.join("subdir");
        fs::create_dir(&subdir).unwrap();
        File::create(subdir.join("visible_nested.txt")).unwrap();
        File::create(subdir.join(".hidden_nested")).unwrap();

        // Create hidden subdirectory
        let hidden_subdir = temp_path.join(".hidden_dir");
        fs::create_dir(&hidden_subdir).unwrap();
        File::create(hidden_subdir.join("file_in_hidden_dir.txt")).unwrap();

        // Initialize search system for this directory
        irondrop::search::initialize_search(temp_path.to_path_buf());

        // Give indexing time to complete
        std::thread::sleep(std::time::Duration::from_millis(200));

        // Test search for visible files
        let search_params = irondrop::search::SearchParams {
            query: "document".to_string(),
            path: "/".to_string(),
            limit: 50,
            offset: 0,
            case_sensitive: false,
        };

        let results = irondrop::search::perform_search(temp_path, &search_params).unwrap();

        // Should find visible files
        let result_names: Vec<String> = results.iter().map(|r| r.name.clone()).collect();
        assert!(
            result_names.contains(&"document.pdf".to_string()),
            "Should find visible document.pdf"
        );

        // Should not find hidden files
        for hidden_file in &hidden_files {
            assert!(
                !result_names.contains(&hidden_file.to_string()),
                "Should not find hidden file {}",
                hidden_file
            );
        }

        // Test broader search that might match hidden files
        let broad_search_params = irondrop::search::SearchParams {
            query: "git".to_string(),
            path: "/".to_string(),
            limit: 50,
            offset: 0,
            case_sensitive: false,
        };

        let broad_results =
            irondrop::search::perform_search(temp_path, &broad_search_params).unwrap();
        let broad_result_names: Vec<String> =
            broad_results.iter().map(|r| r.name.clone()).collect();

        // Should not find .gitignore even though it contains "git"
        assert!(
            !broad_result_names.contains(&".gitignore".to_string()),
            "Should not find .gitignore in search results"
        );

        // Test search for nested files
        let nested_search_params = irondrop::search::SearchParams {
            query: "nested".to_string(),
            path: "/".to_string(),
            limit: 50,
            offset: 0,
            case_sensitive: false,
        };

        let nested_results =
            irondrop::search::perform_search(temp_path, &nested_search_params).unwrap();
        let nested_result_names: Vec<String> =
            nested_results.iter().map(|r| r.name.clone()).collect();

        // Should find visible nested file
        assert!(
            nested_result_names.contains(&"visible_nested.txt".to_string()),
            "Should find visible nested file"
        );

        // Should not find hidden nested file
        assert!(
            !nested_result_names.contains(&".hidden_nested".to_string()),
            "Should not find hidden nested file"
        );
    }
}
