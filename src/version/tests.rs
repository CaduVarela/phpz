use super::*;

const AVAILABLE: &[Version] = &[(8, 2, 9), (8, 2, 30), (8, 3, 9), (8, 3, 10), (8, 4, 1)];

fn builds(versions: &[Version]) -> Vec<Build> {
    versions
        .iter()
        .map(|&version| Build {
            version,
            url: String::new(),
            sha256: None,
            sbom_url: None,
            expects_checksum: false,
            variant: None,
            size: None,
        })
        .collect()
}

fn selected(input: &str) -> Result<String, String> {
    let requested = parse_request(input)?;
    find_build(&builds(AVAILABLE), &requested)
        .map(Build::version_name)
        .ok_or_else(|| format!("no PHP build found for version '{}'", input))
}

#[test]
fn major_resolves_to_newest_version() {
    assert_eq!(selected("8").unwrap(), "8.4.1");
}

#[test]
fn minor_resolves_to_newest_patch_comparing_numbers_not_text() {
    assert_eq!(selected("8.2").unwrap(), "8.2.30");
    assert_eq!(selected("8.3").unwrap(), "8.3.10");
}

#[test]
fn full_version_must_exist() {
    assert_eq!(selected("8.3.9").unwrap(), "8.3.9");
    assert!(selected("8.3.8").is_err());
}

#[test]
fn unknown_version_is_an_error() {
    assert!(selected("9").is_err());
    assert!(selected("8.9").is_err());
}

#[test]
fn malformed_input_is_rejected() {
    for input in ["", "abc", "8.x", "8.3.9.1", "-1"] {
        assert!(selected(input).is_err(), "{}", input);
    }
}

#[test]
fn selected_build_keeps_its_url_and_checksum() {
    let available = vec![Build {
        version: (8, 3, 9),
        url: "https://example.test/php.zip".to_string(),
        sha256: Some("abc".to_string()),
        sbom_url: None,
        expects_checksum: false,
        variant: None,
        size: None,
    }];
    let build = find_build(&available, &[8, 3]).unwrap();
    assert_eq!(build.url, "https://example.test/php.zip");
    assert_eq!(build.sha256.as_deref(), Some("abc"));
}

#[test]
fn parse_version_requires_exactly_three_numbers() {
    assert_eq!(parse_version("8.3.32"), Some((8, 3, 32)));
    assert_eq!(parse_version("8.3"), None);
    assert_eq!(parse_version("8.3.32.1"), None);
    assert_eq!(parse_version("8.3.x"), None);
}
