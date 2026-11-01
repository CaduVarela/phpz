use super::*;
use std::io::Write;

fn directives(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn only_the_wanted_commented_directives_are_enabled() {
    let template =
        ";extension=curl\n;extension=ldap\n;extension_dir = \"ext\"\nmemory_limit = 128M\n";
    let wanted = directives(&["extension=curl", "extension_dir = \"ext\""]);
    assert_eq!(
        uncomment_directives(template, &wanted),
        "extension=curl\n;extension=ldap\nextension_dir = \"ext\"\nmemory_limit = 128M\n"
    );
}

#[test]
fn already_active_and_unknown_lines_are_untouched() {
    let template = "extension=curl\n; extension=curl\n;; extension=curl\n";
    assert_eq!(
        uncomment_directives(template, &directives(&["extension=curl"])),
        template
    );
}

#[test]
fn crlf_line_endings_survive_enabling_a_directive() {
    let template = ";extension=curl\r\nx=1\r\n";
    assert_eq!(
        uncomment_directives(template, &directives(&["extension=curl"])),
        "extension=curl\r\nx=1\r\n"
    );
}

fn write_zip(path: &Path, entries: &[(&str, &str)]) {
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    let options = zip::write::FileOptions::default();
    for (name, content) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

#[test]
fn zip_is_extracted_with_its_directory_structure() {
    let directory = tempfile::tempdir().unwrap();
    let archive = directory.path().join("php.zip");
    write_zip(
        &archive,
        &[("php.exe", "binary"), ("ext/php_curl.dll", "dll")],
    );

    let destination = directory.path().join("out");
    extract_zip(&archive, &destination).unwrap();

    assert_eq!(
        fs::read_to_string(destination.join("php.exe")).unwrap(),
        "binary"
    );
    assert_eq!(
        fs::read_to_string(destination.join("ext/php_curl.dll")).unwrap(),
        "dll"
    );
}

#[test]
fn zip_entry_escaping_the_destination_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let archive = directory.path().join("evil.zip");
    write_zip(&archive, &[("../escaped.txt", "bad")]);

    let destination = directory.path().join("out");
    fs::create_dir_all(&destination).unwrap();
    let error = extract_zip(&archive, &destination).unwrap_err();

    assert!(error.contains("unsafe path"), "{}", error);
    assert!(!directory.path().join("escaped.txt").exists());
}

#[test]
fn directive_followed_by_a_comment_is_still_enabled_and_keeps_its_comment() {
    let template = ";extension=exif      ; Must be after mbstring as it depends on it\n";
    assert_eq!(
        uncomment_directives(template, &directives(&["extension=exif"])),
        "extension=exif      ; Must be after mbstring as it depends on it\n"
    );
}

#[test]
fn directive_that_only_starts_like_a_wanted_one_is_not_enabled() {
    let template = ";extension=curl_extra\n";
    assert_eq!(
        uncomment_directives(template, &directives(&["extension=curl"])),
        template
    );
}

#[test]
fn old_series_get_dll_names_and_newer_ones_short_names() {
    assert!(windows_directives(true).contains(&"extension=php_curl.dll".to_string()));
    assert!(!windows_directives(true).contains(&"extension=curl".to_string()));
    assert!(windows_directives(false).contains(&"extension=curl".to_string()));
    assert!(windows_directives(false).contains(&"extension_dir = \"ext\"".to_string()));
}

#[test]
fn extension_dir_is_enabled_with_or_without_a_space_after_the_semicolon() {
    let wanted = directives(&["extension_dir = \"ext\""]);
    assert_eq!(
        uncomment_directives("; extension_dir = \"ext\"\n", &wanted),
        "extension_dir = \"ext\"\n"
    );
    assert_eq!(
        uncomment_directives(";extension_dir = \"ext\"\n", &wanted),
        "extension_dir = \"ext\"\n"
    );
}

#[test]
fn a_prose_comment_that_looks_like_an_extension_line_stays_a_comment() {
    let wanted = directives(&["extension=curl"]);
    assert_eq!(
        uncomment_directives("; extension=curl\n", &wanted),
        "; extension=curl\n"
    );
}

#[test]
fn dll_style_lines_with_trailing_comments_are_enabled() {
    let wanted = directives(&["extension=php_exif.dll"]);
    assert_eq!(
        uncomment_directives(
            ";extension=php_exif.dll      ; Must be after mbstring\n",
            &wanted
        ),
        "extension=php_exif.dll      ; Must be after mbstring\n"
    );
}

#[test]
fn the_default_set_activates_gd_under_its_old_name_too() {
    assert!(windows_directives(true).contains(&"extension=php_gd2.dll".to_string()));
    assert!(windows_directives(false).contains(&"extension=gd2".to_string()));
    assert!(windows_directives(false).contains(&"extension=gd".to_string()));
}

#[test]
fn old_template_with_gd2_gets_gd_activated_and_nothing_else_changes() {
    let template = ";extension=gd2\n;extension=ldap\n";
    assert_eq!(
        uncomment_directives(template, &windows_directives(false)),
        "extension=gd2\n;extension=ldap\n"
    );
}

fn version_directory(
    root: &Path,
    name: &str,
    ini: &str,
    conf_files: &[(&str, &str)],
) -> std::path::PathBuf {
    let directory = root.join(name);
    fs::create_dir_all(directory.join("conf.d")).unwrap();
    fs::write(directory.join("php.ini"), ini).unwrap();
    for (file, content) in conf_files {
        fs::write(directory.join("conf.d").join(file), content).unwrap();
    }
    directory
}

#[test]
fn a_new_build_keeps_the_settings_of_the_old_one() {
    let root = tempfile::tempdir().unwrap();
    let old = version_directory(
        root.path(),
        "old",
        "memory_limit = 1G\n",
        &[("extra.ini", "x=1\n")],
    );
    let new = version_directory(
        root.path(),
        "new",
        "memory_limit = 512M\n",
        &[("default.ini", "y=2\n")],
    );

    carry_over_configuration(&old, &new).unwrap();

    assert_eq!(
        fs::read_to_string(new.join("php.ini")).unwrap(),
        "memory_limit = 1G\n"
    );
    assert_eq!(
        fs::read_to_string(new.join("conf.d/extra.ini")).unwrap(),
        "x=1\n"
    );
    assert!(
        new.join("conf.d/default.ini").exists(),
        "files the new build came with stay"
    );
}

#[test]
fn carrying_over_from_a_build_without_conf_d_or_php_ini_is_fine() {
    let root = tempfile::tempdir().unwrap();
    let old = root.path().join("old");
    fs::create_dir_all(&old).unwrap();
    let new = version_directory(root.path(), "new", "memory_limit = 512M\n", &[]);

    carry_over_configuration(&old, &new).unwrap();
    assert_eq!(
        fs::read_to_string(new.join("php.ini")).unwrap(),
        "memory_limit = 512M\n"
    );
}

#[test]
fn swapping_puts_the_new_build_in_place_and_removes_the_old_one() {
    let root = tempfile::tempdir().unwrap();
    let current = version_directory(root.path(), "8.3.32", "old\n", &[]);
    let new = version_directory(root.path(), "8.3.32.tmp", "new\n", &[]);
    let backup = root.path().join("8.3.32.old");

    swap_in(&new, &current, &backup).unwrap();

    assert_eq!(
        fs::read_to_string(current.join("php.ini")).unwrap(),
        "new\n"
    );
    assert!(!new.exists() && !backup.exists());
}

#[test]
fn a_failed_swap_puts_the_old_build_back() {
    let root = tempfile::tempdir().unwrap();
    let current = version_directory(root.path(), "8.3.32", "old\n", &[]);
    let missing_new = root.path().join("does-not-exist");
    let backup = root.path().join("8.3.32.old");

    assert!(swap_in(&missing_new, &current, &backup).is_err());

    assert_eq!(
        fs::read_to_string(current.join("php.ini")).unwrap(),
        "old\n",
        "the old build must be back"
    );
    assert!(!backup.exists());
}

#[test]
fn a_leftover_backup_from_an_earlier_attempt_does_not_block_a_swap() {
    let root = tempfile::tempdir().unwrap();
    let current = version_directory(root.path(), "8.3.32", "old\n", &[]);
    let new = version_directory(root.path(), "8.3.32.tmp", "new\n", &[]);
    let backup = version_directory(root.path(), "8.3.32.old", "stale\n", &[]);

    swap_in(&new, &current, &backup).unwrap();
    assert_eq!(
        fs::read_to_string(current.join("php.ini")).unwrap(),
        "new\n"
    );
}
