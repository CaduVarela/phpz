use super::*;

const AVAILABLE: &[Version] = &[(8, 2, 9), (8, 2, 30), (8, 3, 9), (8, 3, 10), (8, 4, 1)];

#[test]
fn major_resolves_to_newest_version() {
    assert_eq!(select_version(AVAILABLE, "8").unwrap(), "8.4.1");
}

#[test]
fn minor_resolves_to_newest_patch_comparing_numbers_not_text() {
    assert_eq!(select_version(AVAILABLE, "8.2").unwrap(), "8.2.30");
    assert_eq!(select_version(AVAILABLE, "8.3").unwrap(), "8.3.10");
}

#[test]
fn full_version_must_exist() {
    assert_eq!(select_version(AVAILABLE, "8.3.9").unwrap(), "8.3.9");
    assert!(select_version(AVAILABLE, "8.3.8").is_err());
}

#[test]
fn unknown_version_is_an_error() {
    assert!(select_version(AVAILABLE, "9").is_err());
    assert!(select_version(AVAILABLE, "8.9").is_err());
}

#[test]
fn malformed_input_is_rejected() {
    for input in ["", "abc", "8.x", "8.3.9.1", "-1"] {
        assert!(select_version(AVAILABLE, input).is_err(), "{}", input);
    }
}

#[test]
fn parse_version_requires_exactly_three_numbers() {
    assert_eq!(parse_version("8.3.32"), Some((8, 3, 32)));
    assert_eq!(parse_version("8.3"), None);
    assert_eq!(parse_version("8.3.32.1"), None);
    assert_eq!(parse_version("8.3.x"), None);
}
