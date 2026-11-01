use phpz::release::*;

#[test]
fn versions_are_read_with_or_without_the_leading_v() {
    assert_eq!(parse_version("v0.4.0"), Some((0, 4, 0)));
    assert_eq!(parse_version("1.12.3"), Some((1, 12, 3)));
    assert_eq!(parse_version("v1.0.0-rc1"), None);
    assert_eq!(parse_version("v1.0"), None);
    assert_eq!(parse_version("latest"), None);
}

#[test]
fn versions_compare_as_numbers() {
    assert!(parse_version("v0.10.0") > parse_version("v0.9.0"));
}

#[test]
fn every_published_system_has_a_release_file() {
    assert_eq!(asset_name("linux", "x86_64"), Some("phpz-linux-x64"));
    assert_eq!(asset_name("linux", "aarch64"), Some("phpz-linux-arm64"));
    assert_eq!(asset_name("macos", "x86_64"), Some("phpz-macos-x64"));
    assert_eq!(asset_name("macos", "aarch64"), Some("phpz-macos-arm64"));
    assert_eq!(
        asset_name("windows", "x86_64"),
        Some("phpz-windows-x64.exe")
    );
    assert_eq!(asset_name("windows", "aarch64"), None);
    assert_eq!(asset_name("freebsd", "x86_64"), None);
}

#[test]
fn the_checksum_is_found_by_exact_file_name() {
    let sums = "AAA1  phpz-linux-x64\nbbb2  phpz-linux-arm64\nccc3 *phpz-windows-x64.exe\n";
    assert_eq!(
        checksum_for(sums, "phpz-linux-x64"),
        Some("aaa1".to_string())
    );
    assert_eq!(
        checksum_for(sums, "phpz-windows-x64.exe"),
        Some("ccc3".to_string())
    );
    assert_eq!(checksum_for(sums, "phpz-linux"), None);
}
