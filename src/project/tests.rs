use super::*;
use std::path::Path;

fn write(directory: &Path, name: &str, text: &str) {
    fs::create_dir_all(directory).unwrap();
    fs::write(directory.join(name), text).unwrap();
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn a_request_is_one_to_three_numbers() {
    for text in ["8", "8.3", "8.3.9", "10.0.12"] {
        assert!(is_valid_request(text), "{}", text);
    }
    for text in ["", "8.", ".8", "8.3.9.1", "8.x", "latest", "-1", "8..3"] {
        assert!(!is_valid_request(text), "{}", text);
    }
}

#[test]
fn common_spellings_of_a_version_are_accepted() {
    assert_eq!(normalize_request("8.3"), Some("8.3".to_string()));
    assert_eq!(normalize_request(" v8.3.1 \n"), Some("8.3.1".to_string()));
    assert_eq!(normalize_request("php-8.2"), Some("8.2".to_string()));
    assert_eq!(normalize_request("system"), None);
}

#[test]
fn the_version_file_uses_its_first_real_line() {
    let file = Path::new(".php-version");
    assert_eq!(
        parse_version_file("8.3\n", file).unwrap().as_deref(),
        Some("8.3")
    );
    assert_eq!(
        parse_version_file("# comment\n\n  8.2.1  \n8.4\n", file)
            .unwrap()
            .as_deref(),
        Some("8.2.1")
    );
    assert_eq!(parse_version_file("", file).unwrap(), None);
    assert_eq!(
        parse_version_file("# only a comment\n", file).unwrap(),
        None
    );
}

#[test]
fn a_version_file_with_nonsense_is_an_error_naming_the_file() {
    let error = parse_version_file("banana\n", Path::new("/p/.php-version")).unwrap_err();
    assert!(
        error.contains("banana") && error.contains("/p/.php-version"),
        "{}",
        error
    );
}

#[test]
fn the_project_file_carries_a_version_and_extensions() {
    let parsed = parse_project_file(
        "php = \"8.3\"\n\n[extensions]\nenable = [\"intl\", \"php_redis.dll\"]\ndisable = [\"Xdebug\"]\n",
        Path::new(".phpz.toml"),
    )
    .unwrap();
    assert_eq!(parsed.php.as_deref(), Some("8.3"));
    assert_eq!(parsed.extensions.enable, names(&["intl", "redis"]));
    assert_eq!(parsed.extensions.disable, names(&["xdebug"]));
}

#[test]
fn the_project_file_may_hold_only_extensions() {
    let parsed =
        parse_project_file("[extensions]\nenable = [\"gd\"]\n", Path::new(".phpz.toml")).unwrap();
    assert_eq!(parsed.php, None);
    assert_eq!(parsed.extensions.enable, names(&["gd"]));
}

#[test]
fn extension_names_cannot_be_paths_or_odd_characters() {
    for bad in [
        "../evil",
        "C:\\x\\evil",
        "/etc/passwd",
        "a b",
        "x;y",
        "ev!l",
    ] {
        let text = format!("[extensions]\nenable = [{:?}]\n", bad);
        let error = parse_project_file(&text, Path::new(".phpz.toml")).unwrap_err();
        assert!(
            error.contains("invalid extension name"),
            "{}: {}",
            bad,
            error
        );
    }
}

#[test]
fn duplicate_extension_names_collapse() {
    let parsed = parse_project_file(
        "[extensions]\nenable = [\"intl\", \"INTL\", \"php_intl.dll\"]\n",
        Path::new("f"),
    )
    .unwrap();
    assert_eq!(parsed.extensions.enable, names(&["intl"]));
}

#[test]
fn a_broken_project_file_is_an_error_naming_the_file() {
    for text in [
        "php = 8.3\n",
        "php = \"latest\"\n",
        "[extensions\n",
        "extensions = 3\n",
    ] {
        let error = parse_project_file(text, Path::new("/p/.phpz.toml")).unwrap_err();
        assert!(error.contains("/p/.phpz.toml"), "{:?}: {}", text, error);
    }
}

#[test]
fn the_nearest_directory_with_a_project_file_wins() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), ".php-version", "8.1\n");
    write(&root.path().join("app"), ".php-version", "8.3\n");
    let deep = root.path().join("app/src/deep");
    fs::create_dir_all(&deep).unwrap();

    let project = find(&deep).unwrap().unwrap();
    assert_eq!(project.php.unwrap().request, "8.3");
    assert_eq!(project.directory, root.path().join("app"));

    let outside = find(root.path()).unwrap().unwrap();
    assert_eq!(outside.php.unwrap().request, "8.1");
}

#[test]
fn nothing_is_found_without_files() {
    let root = tempfile::tempdir().unwrap();
    // The test machine could have a pin above the temporary directory, so only a pin below it counts.
    let found = find(root.path()).unwrap();
    assert!(found
        .map(|project| !project.directory.starts_with(root.path()))
        .unwrap_or(true));
}

#[test]
fn the_project_file_beats_the_version_file_in_the_same_directory() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), ".php-version", "8.1\n");
    write(root.path(), ".phpz.toml", "php = \"8.4\"\n");

    let pinned = find(root.path()).unwrap().unwrap().php.unwrap();
    assert_eq!(pinned.request, "8.4");
    assert_eq!(pinned.file, root.path().join(".phpz.toml"));
}

#[test]
fn a_project_file_without_a_version_defers_to_the_version_file_next_to_it() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), ".php-version", "8.1\n");
    write(
        root.path(),
        ".phpz.toml",
        "[extensions]\nenable = [\"gd\"]\n",
    );

    let project = find(root.path()).unwrap().unwrap();
    assert_eq!(project.php.unwrap().file, root.path().join(".php-version"));
    assert_eq!(project.extensions.enable, names(&["gd"]));
}

#[test]
fn an_empty_version_file_does_not_stop_the_search() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), ".php-version", "8.2\n");
    write(&root.path().join("sub"), ".php-version", "# nothing yet\n");

    let project = find(&root.path().join("sub")).unwrap().unwrap();
    assert_eq!(project.php.unwrap().request, "8.2");
}

#[test]
fn a_broken_file_in_the_way_is_reported_and_not_skipped() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), ".php-version", "8.2\n");
    write(&root.path().join("sub"), ".php-version", "banana\n");
    assert!(find(&root.path().join("sub")).is_err());
}

#[test]
fn only_a_project_file_that_sets_php_blocks_local() {
    let root = tempfile::tempdir().unwrap();
    assert!(!project_file_pins_php(root.path()).unwrap());
    write(
        root.path(),
        ".phpz.toml",
        "[extensions]\nenable = [\"gd\"]\n",
    );
    assert!(!project_file_pins_php(root.path()).unwrap());
    write(root.path(), ".phpz.toml", "php = \"8.3\"\n");
    assert!(project_file_pins_php(root.path()).unwrap());
}
