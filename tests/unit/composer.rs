use phpz::composer::*;

#[test]
fn a_php_version_becomes_the_number_composer_uses() {
    assert_eq!(php_id("7.2.5"), Some(70205));
    assert_eq!(php_id("8.3.32"), Some(80332));
    assert_eq!(php_id("8.3"), Some(80300));
    assert_eq!(php_id("eight"), None);
    assert_eq!(php_id("8.x.1"), None);
}

#[test]
fn old_php_gets_the_long_term_support_line_and_the_rest_the_newest() {
    assert_eq!(channel_for_version("8.4.1"), Some("2"));
    assert_eq!(channel_for_version("7.2.5"), Some("2"));
    assert_eq!(channel_for_version("7.2.4"), Some("2.2"));
    assert_eq!(channel_for_version("5.6.40"), Some("2.2"));
    assert_eq!(channel_for_version("5.3.2"), Some("2.2"));
    assert_eq!(channel_for_version("5.3.1"), None);
}

#[test]
fn the_download_path_of_a_line_comes_from_the_versions_list() {
    let versions = r#"{
        "stable": [{"path": "/download/2.11.1/composer.phar", "version": "2.11.1"}],
        "2.2": [{"path": "/download/2.2.31/composer.phar", "version": "2.2.31"}],
        "2": [{"path": "/download/2.11.1/composer.phar", "version": "2.11.1"}]
    }"#;
    assert_eq!(
        download_path(versions, "2").as_deref(),
        Some("/download/2.11.1/composer.phar")
    );
    assert_eq!(
        download_path(versions, "2.2").as_deref(),
        Some("/download/2.2.31/composer.phar")
    );
    assert_eq!(download_path(versions, "3"), None);
    assert_eq!(download_path("not json", "2"), None);
}

#[test]
fn the_checksum_is_the_hash_in_the_published_file() {
    let hash = "471f2d857abf0ec18af7b055e61472214d91adb24f9bdbbb864c1c64faad7dd6";
    assert_eq!(
        checksum_from(&format!("{}  composer.phar\n", hash)).as_deref(),
        Some(hash)
    );
    assert_eq!(
        checksum_from(&format!("{}  composer.phar", hash.to_uppercase())).as_deref(),
        Some(hash)
    );
    assert_eq!(checksum_from("<html>not found</html>"), None);
    assert_eq!(checksum_from(""), None);
}

#[cfg(unix)]
mod user_composer {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fn program(directory: &std::path::Path, name: &str, mode: u32) {
        fs::create_dir_all(directory).unwrap();
        let path = directory.join(name);
        fs::write(&path, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn a_composer_of_the_user_is_found_but_not_the_one_in_the_shims_folder() {
        let root = tempfile::tempdir().unwrap();
        let shims = root.path().join("shims");
        let first = root.path().join("first");
        let second = root.path().join("second");
        program(&shims, "composer", 0o755);
        program(&first, "composer", 0o644);
        program(&second, "composer", 0o755);
        let path = std::env::join_paths([&shims, &first, &second]).unwrap();

        assert_eq!(
            find_in_path(&path, &shims, &["composer"]),
            Some(second.join("composer"))
        );
        assert_eq!(find_in_path(&path, &shims, &["composer.exe"]), None);
    }
}
