use phpz::extensions::pecl::*;
use std::fs;
use std::io::Write;
use std::path::Path;

const VERSIONS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pecl-xdebug-versions.html"
));
const FILES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/pecl-xdebug-3.5.3.html"
));

#[test]
fn only_plain_extension_names_are_valid() {
    for name in ["xdebug", "pdo_sqlsrv", "ds", "oci8_19"] {
        assert!(is_valid_name(name), "{}", name);
    }
    for name in ["", "XDebug", "../x", "a/b", "a b", "php_x.dll", "x?y"] {
        assert!(!is_valid_name(name), "{}", name);
    }
}

#[test]
fn the_series_comes_from_the_php_version() {
    assert_eq!(series_of("8.3.36"), Some((8, 3)));
    assert_eq!(series_of("7.0.33"), Some((7, 0)));
    assert_eq!(series_of("8"), None);
    assert_eq!(series_of("abc"), None);
}

#[test]
fn stable_versions_skip_prereleases_and_come_newest_first() {
    assert_eq!(
        stable_versions(VERSIONS),
        ["3.5.3", "3.4.0", "3.3.0", "2.9.8"]
    );
}

#[test]
fn versions_are_ordered_as_numbers() {
    let page = r#"<a href="3.9.0/">x</a><a href="3.10.0/">x</a><a href="3.2.1/">x</a><a href="/~windows/">x</a><a href="logs/">x</a><a href="?C=N;O=D">x</a>"#;
    assert_eq!(stable_versions(page), ["3.10.0", "3.9.0", "3.2.1"]);
}

#[test]
fn the_zip_for_a_series_is_the_non_thread_safe_x64_one() {
    assert_eq!(
        find_zip(FILES, "xdebug", "3.5.3", (8, 3)).as_deref(),
        Some("php_xdebug-3.5.3-8.3-nts-vs16-x64.zip")
    );
    assert_eq!(
        find_zip(FILES, "xdebug", "3.5.3", (8, 4)).as_deref(),
        Some("php_xdebug-3.5.3-8.4-nts-vs17-x64.zip")
    );
}

#[test]
fn a_series_without_a_build_gives_nothing() {
    assert_eq!(find_zip(FILES, "xdebug", "3.5.3", (7, 4)), None);
    assert_eq!(find_zip(FILES, "xdebug", "9.9.9", (8, 3)), None);
    assert_eq!(find_zip(FILES, "redis", "3.5.3", (8, 3)), None);
}

#[test]
fn a_minor_version_is_not_confused_with_a_longer_one() {
    let page = r#"<a href="php_x-1.0-8.30-nts-vs16-x64.zip">x</a>"#;
    assert_eq!(find_zip(page, "x", "1.0", (8, 3)), None);
}

fn write_zip(path: &Path, entries: &[(&str, &str)]) {
    let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let options = zip::write::FileOptions::default();
    for (name, content) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

fn install(entries: &[(&str, &str)]) -> (tempfile::TempDir, Result<(), String>) {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("package.zip");
    write_zip(&archive, entries);
    let result = install_dll(&archive, "xdebug", &root.path().join("ext"));
    (root, result)
}

#[test]
fn the_dll_is_installed_and_everything_else_in_the_package_is_left_alone() {
    let (root, result) = install(&[
        ("php_xdebug.dll", "the dll"),
        ("php_xdebug.pdb", "symbols"),
        ("xdebug.ini", "settings"),
        ("LICENSE", "text"),
        ("contrib/tracefile-analyser.php", "a sample script"),
    ]);
    result.unwrap();

    let ext = root.path().join("ext");
    assert_eq!(
        fs::read_to_string(ext.join("php_xdebug.dll")).unwrap(),
        "the dll"
    );
    let names: Vec<String> = fs::read_dir(&ext)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().into_string().unwrap())
        .collect();
    assert_eq!(
        names,
        ["php_xdebug.dll"],
        "nothing else, not even a temporary file"
    );
}

#[test]
fn the_file_name_is_matched_ignoring_case() {
    let (root, result) = install(&[("PHP_XDEBUG.DLL", "the dll")]);
    result.unwrap();
    assert!(root.path().join("ext/php_xdebug.dll").exists());
}

#[test]
fn a_package_with_extra_libraries_is_refused_and_nothing_is_written() {
    let (root, result) = install(&[
        ("php_xdebug.dll", "x"),
        ("config/colors.xml", "folders alone are fine"),
        ("CORE_RL_magick_.dll", "y"),
    ]);
    let error = result.unwrap_err();
    assert!(
        error.contains("extra libraries") && error.contains("CORE_RL_magick_.dll"),
        "{}",
        error
    );
    assert!(!root.path().join("ext").exists());
}

#[test]
fn libraries_and_programs_are_refused_wherever_they_are() {
    for entry in [
        "tool.exe",
        "lib/helper.dll",
        "bin\\helper.dll",
        "../php_xdebug.dll",
    ] {
        let (_root, result) = install(&[("php_xdebug.dll", "x"), (entry, "y")]);
        assert!(result.is_err(), "{}", entry);
    }
}

#[test]
fn an_extension_dll_hidden_in_a_folder_is_not_taken_for_the_real_one() {
    let (_root, result) = install(&[("bin/php_xdebug.dll", "x")]);
    assert!(result.is_err());
}

#[test]
fn a_package_without_the_dll_is_refused() {
    let (_root, result) = install(&[("xdebug.ini", "settings")]);
    assert!(result
        .unwrap_err()
        .contains("does not contain php_xdebug.dll"));
}

#[test]
fn rest_listing_gives_stable_versions_newest_first() {
    let xml = "<a><r><v>3.10.0</v><s>stable</s></r><r><v>3.11.0beta1</v><s>beta</s></r><r><v>3.9.0</v><s>stable</s></r></a>";
    assert_eq!(stable_versions_from_rest(xml), vec!["3.10.0", "3.9.0"]);
}
