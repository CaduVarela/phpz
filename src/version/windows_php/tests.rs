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
