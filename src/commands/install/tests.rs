use super::*;

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn matching_checksum_is_accepted_regardless_of_case() {
    assert!(check_sha256(ABC_SHA256, ABC_SHA256).is_ok());
    assert!(check_sha256(ABC_SHA256, &ABC_SHA256.to_uppercase()).is_ok());
}

#[test]
fn different_checksum_is_rejected_with_both_values() {
    let error = check_sha256(ABC_SHA256, "00").unwrap_err();
    assert!(error.contains("checksum mismatch"));
    assert!(error.contains(ABC_SHA256) && error.contains("00"));
}

#[test]
fn sha256_of_known_input_matches_the_standard_test_vector() {
    assert_eq!(format!("{:x}", Sha256::digest(b"abc")), ABC_SHA256);
}

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
fn builds_without_a_checksum_from_sources_that_never_publish_one_need_no_consent() {
    for allow in [false, true] {
        for interactive in [false, true] {
            assert_eq!(consent_for(false, allow, interactive), Consent::NotNeeded);
        }
    }
}

#[test]
fn unusual_missing_checksum_asks_a_person_at_a_terminal() {
    assert_eq!(consent_for(true, false, true), Consent::Ask);
}

#[test]
fn unusual_missing_checksum_is_refused_in_scripts() {
    assert_eq!(consent_for(true, false, false), Consent::Refused);
}

#[test]
fn the_flag_gives_consent_with_or_without_a_terminal() {
    assert_eq!(consent_for(true, true, false), Consent::Given);
    assert_eq!(consent_for(true, true, true), Consent::Given);
}
