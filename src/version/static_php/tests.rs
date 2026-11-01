use super::*;

fn entry(name: &str, is_dir: bool, size: &str) -> IndexEntry {
    IndexEntry {
        name: name.to_string(),
        is_dir,
        size: size.to_string(),
    }
}

fn index() -> Vec<IndexEntry> {
    vec![
        entry("license/", true, "-"),
        entry("README.txt", false, "654B"),
        entry("php-8.3.32-cli-linux-x86_64.tar.gz", false, "11.2M"),
        entry("php-8.3.32-fpm-linux-x86_64.tar.gz", false, "11.2M"),
        entry("php-8.3.32-micro-linux-x86_64.tar.gz", false, "11.1M"),
        entry("php-8.3.32-cli-linux-aarch64.tar.gz", false, "10.9M"),
        entry("php-8.3.32-cli-macos-aarch64.tar.gz", false, "11.0M"),
        entry("php-8.4.23-cli-linux-x86_64.tar.gz", false, "11.8M"),
    ]
}

#[test]
fn only_cli_builds_of_the_platform_are_kept() {
    let builds = parse_builds(&index(), Variant::Common, "linux-x86_64");
    let names: Vec<String> = builds.iter().map(Build::version_name).collect();
    assert_eq!(names, ["8.3.32", "8.4.23"]);
}

#[test]
fn a_build_knows_its_variant_its_address_and_its_size() {
    let builds = parse_builds(&index(), Variant::GnuBulk, "linux-x86_64");
    assert_eq!(builds[0].variant, Some(Variant::GnuBulk));
    assert_eq!(
        builds[0].url,
        "https://dl.static-php.dev/static-php-cli/gnu-bulk/php-8.3.32-cli-linux-x86_64.tar.gz"
    );
    assert_eq!(builds[0].size.as_deref(), Some("11.2M"));
    assert_eq!(builds[0].sha256, None);
    assert!(!builds[0].expects_checksum);
}

#[test]
fn another_platform_gives_its_own_builds() {
    let builds = parse_builds(&index(), Variant::Common, "macos-aarch64");
    assert_eq!(builds.len(), 1);
    assert!(builds[0]
        .url
        .ends_with("php-8.3.32-cli-macos-aarch64.tar.gz"));
    assert!(parse_builds(&index(), Variant::Common, "linux-riscv64").is_empty());
}

#[test]
fn a_missing_size_is_not_shown() {
    let builds = parse_builds(
        &[entry("php-8.3.32-cli-linux-x86_64.tar.gz", false, "")],
        Variant::Common,
        "linux-x86_64",
    );
    assert_eq!(builds[0].size, None);
}
