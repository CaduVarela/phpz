use super::*;
use std::io::Write;

fn directives(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn only_the_wanted_commented_directives_are_enabled() {
    let template = ";extension=curl\n;extension=ldap\n;extension_dir = \"ext\"\nmemory_limit = 128M\n";
    let wanted = directives(&["extension=curl", "extension_dir = \"ext\""]);
    assert_eq!(
        uncomment_directives(template, &wanted),
        "extension=curl\n;extension=ldap\nextension_dir = \"ext\"\nmemory_limit = 128M\n"
    );
}

#[test]
fn already_active_and_unknown_lines_are_untouched() {
    let template = "extension=curl\n; extension=curl\n;; extension=curl\n";
    assert_eq!(uncomment_directives(template, &directives(&["extension=curl"])), template);
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
    write_zip(&archive, &[("php.exe", "binary"), ("ext/php_curl.dll", "dll")]);

    let destination = directory.path().join("out");
    extract_zip(&archive, &destination).unwrap();

    assert_eq!(fs::read_to_string(destination.join("php.exe")).unwrap(), "binary");
    assert_eq!(fs::read_to_string(destination.join("ext/php_curl.dll")).unwrap(), "dll");
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
    assert_eq!(uncomment_directives(template, &directives(&["extension=curl"])), template);
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
    assert_eq!(uncomment_directives("; extension_dir = \"ext\"\n", &wanted), "extension_dir = \"ext\"\n");
    assert_eq!(uncomment_directives(";extension_dir = \"ext\"\n", &wanted), "extension_dir = \"ext\"\n");
}

#[test]
fn a_prose_comment_that_looks_like_an_extension_line_stays_a_comment() {
    let wanted = directives(&["extension=curl"]);
    assert_eq!(uncomment_directives("; extension=curl\n", &wanted), "; extension=curl\n");
}

#[test]
fn dll_style_lines_with_trailing_comments_are_enabled() {
    let wanted = directives(&["extension=php_exif.dll"]);
    assert_eq!(
        uncomment_directives(";extension=php_exif.dll      ; Must be after mbstring\n", &wanted),
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
