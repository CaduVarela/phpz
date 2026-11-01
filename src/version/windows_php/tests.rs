use super::*;

const RELEASES: &str = include_str!("../../../tests/fixtures/releases.json");

fn find(builds: &[Build], version: &str) -> Option<Build> {
    builds
        .iter()
        .find(|build| build.version_name() == version)
        .cloned()
}

#[test]
fn newest_release_of_each_series_with_an_x64_build_is_listed() {
    let builds = parse_releases(RELEASES).unwrap();
    let mut names: Vec<String> = builds.iter().map(Build::version_name).collect();
    names.sort();
    assert_eq!(names, ["7.4.33", "8.3.36", "8.4.27"]);
}

#[test]
fn packaging_revision_is_dropped_from_the_version_but_kept_in_the_file_name() {
    let build = find(&parse_releases(RELEASES).unwrap(), "8.3.36").unwrap();
    assert_eq!(
        build.url,
        "https://windows.php.net/downloads/releases/php-8.3.36-1-nts-Win32-vs16-x64.zip"
    );
}

#[test]
fn non_thread_safe_x64_build_is_chosen() {
    let build = find(&parse_releases(RELEASES).unwrap(), "8.4.27").unwrap();
    assert_eq!(
        build.url,
        "https://windows.php.net/downloads/releases/php-8.4.27-1-nts-Win32-vs17-x64.zip"
    );
}

#[test]
fn checksum_is_kept_as_lowercase_hex() {
    let build = find(&parse_releases(RELEASES).unwrap(), "7.4.33").unwrap();
    assert_eq!(
        build.sha256.as_deref(),
        Some("14ae3250d4447c8ccfc4c45a70d90adfbcd61e728d85f0be56a7ddf8f9c8aace")
    );
}

#[test]
fn invalid_json_is_an_error() {
    assert!(parse_releases("not json").is_err());
    assert!(parse_releases("[]").is_err());
}

const LISTING: &str = include_str!("../../../tests/fixtures/archives-listing.html");
const SBOM: &str = include_str!("../../../tests/fixtures/sbom.cdx.json");

fn names(builds: &[Build]) -> Vec<String> {
    builds.iter().map(Build::version_name).collect()
}

#[test]
fn archive_name_gives_version_and_revision() {
    assert_eq!(
        parse_archive_name("php-8.5.12-nts-Win32-vs17-x64.zip"),
        Some(((8, 5, 12), 0))
    );
    assert_eq!(
        parse_archive_name("php-8.5.12-1-nts-Win32-vs17-x64.zip"),
        Some(((8, 5, 12), 1))
    );
    assert_eq!(
        parse_archive_name("php-7.4.33-nts-Win32-vc15-x64.zip"),
        Some(((7, 4, 33), 0))
    );
}

#[test]
fn files_that_are_not_non_thread_safe_x64_builds_are_ignored() {
    for name in [
        "php-8.5.11-Win32-vs17-x64.zip",
        "php-8.5.11-nts-Win32-vs17-x86.zip",
        "php-devel-pack-8.5.11-nts-Win32-vs17-x64.zip",
        "php-debug-pack-8.5.11-nts-Win32-vs17-x64.zip",
        "php-test-pack-8.5.11.zip",
        "php-8.5.11-nts-Win32-vs17-x64.zip.cdx.json",
        "sha1sum.txt",
    ] {
        assert_eq!(parse_archive_name(name), None, "{}", name);
    }
}

#[test]
fn archive_listing_keeps_supported_versions_only() {
    let builds = parse_archive_listing(LISTING);
    assert_eq!(
        names(&builds),
        ["7.3.33", "7.4.33", "8.0.30", "8.3.12", "8.5.11", "8.5.12"]
    );
}

#[test]
fn archive_listing_prefers_the_newest_rebuild_of_a_version() {
    let builds = parse_archive_listing(LISTING);
    let build = builds
        .iter()
        .find(|build| build.version_name() == "8.5.12")
        .unwrap();
    assert_eq!(
        build.url,
        "https://windows.php.net/downloads/releases/archives/php-8.5.12-1-nts-Win32-vs17-x64.zip"
    );
}

#[test]
fn archive_builds_point_to_their_sbom_and_have_no_checksum_yet() {
    let build = &parse_archive_listing(LISTING)[0];
    assert_eq!(build.sha256, None);
    assert_eq!(
        build.sbom_url.as_deref(),
        Some(format!("{}.cdx.json", build.url).as_str())
    );
}

#[test]
fn releases_json_entry_wins_over_the_archive_entry_of_the_same_version() {
    let merged = merge(
        parse_archive_listing(LISTING),
        parse_releases(RELEASES).unwrap(),
    );
    assert_eq!(
        names(&merged),
        ["7.3.33", "7.4.33", "8.0.30", "8.3.12", "8.3.36", "8.4.27", "8.5.11", "8.5.12"]
    );

    let build = find(&merged, "7.4.33").unwrap();
    assert!(build.url.contains("/releases/php-7.4.33-"), "{}", build.url);
    assert!(build.sha256.is_some());
    assert_eq!(build.sbom_url, None);
}

#[test]
fn checksum_is_read_from_the_sbom_component_that_describes_the_zip() {
    assert_eq!(
        parse_sbom_checksum(SBOM).as_deref(),
        Some("03249b5c9414c6dbe30276f4a7598bd9d2a7417ee81f709b06b99e9c4a2aff4f")
    );
}

#[test]
fn sbom_without_a_usable_sha256_gives_no_checksum() {
    assert_eq!(parse_sbom_checksum("not json"), None);
    assert_eq!(parse_sbom_checksum("{}"), None);
    assert_eq!(
        parse_sbom_checksum(r#"{"metadata":{"component":{"hashes":[]}}}"#),
        None
    );
    assert_eq!(
        parse_sbom_checksum(
            r#"{"metadata":{"component":{"hashes":[{"alg":"MD5","content":"ab"}]}}}"#
        ),
        None
    );
}

#[test]
fn sbom_checksum_is_lowercased() {
    let json = r#"{"metadata":{"component":{"hashes":[{"alg":"SHA-256","content":"ABCDEF"}]}}}"#;
    assert_eq!(parse_sbom_checksum(json).as_deref(), Some("abcdef"));
}
