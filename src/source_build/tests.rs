use super::*;
use std::ffi::OsString;

const RELEASE: &str = include_str!("../../tests/fixtures/php-release-8.3.36.json");

fn everything(_: &str) -> bool {
    true
}

fn nothing(_: &str) -> bool {
    false
}

#[test]
fn the_release_comes_with_the_tar_gz_and_the_checksum_php_net_publishes() {
    let release = parse_release(RELEASE).unwrap();
    assert_eq!(release.version, "8.3.36");
    assert_eq!(release.url, "https://www.php.net/distributions/php-8.3.36.tar.gz");
    assert_eq!(release.signature_url(), "https://www.php.net/distributions/php-8.3.36.tar.gz.asc");
    assert_eq!(release.sha256.len(), 64);
    assert!(release.sha256.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

#[test]
fn the_series_is_major_and_minor() {
    assert_eq!(parse_release(RELEASE).unwrap().series(), (8, 3));
}

#[test]
fn data_without_a_release_or_a_source_is_an_error() {
    assert!(parse_release("not json").is_err());
    assert!(parse_release(r#"{"error":"Unknown version"}"#).unwrap_err().contains("no release"));
    assert!(parse_release(r#"{"version":"8.3.1","source":[]}"#).unwrap_err().contains("tar.gz"));
    assert!(parse_release(r#"{"version":"8.3.1","source":[{"filename":"php-8.3.1.tar.gz"}]}"#).unwrap_err().contains("SHA-256"));
}

#[test]
fn a_bad_request_is_refused_before_asking_php_net() {
    for input in ["", "latest", "8.x", "8.3.1.2"] {
        assert!(resolve(input).unwrap_err().contains("invalid version"), "{}", input);
    }
}

#[test]
fn with_every_library_everything_is_built() {
    let plan = plan((8, 3), everything);
    assert!(plan.skipped.is_empty());
    assert_eq!(plan.built_in, ["openssl", "curl", "zlib", "mbstring", "xml", "sqlite", "zip", "gd"]);
    assert_eq!(plan.shared, ["opcache", "intl", "soap", "sodium"]);
    assert!(plan.flags.contains(&"--with-openssl".to_string()));
    assert!(plan.flags.contains(&"--enable-intl=shared".to_string()));
    assert!(plan.flags.contains(&"--enable-opcache=shared".to_string()));
}

#[test]
fn without_libraries_only_what_needs_none_is_built() {
    let plan = plan((8, 3), nothing);
    assert!(plan.built_in.is_empty());
    assert_eq!(plan.shared, ["opcache"]);
    assert_eq!(plan.skipped.len(), 11, "every optional component");
    assert!(plan.flags.contains(&"--disable-all".to_string()) && plan.flags.contains(&"--enable-cli".to_string()));
    assert!(!plan.flags.iter().any(|flag| flag.contains("openssl") || flag.contains("intl")));
}

#[test]
fn a_missing_library_skips_only_what_needs_it_and_names_the_package() {
    let plan = plan((8, 3), |package| package != "icu-i18n");
    assert_eq!(plan.skipped, [Skipped { name: "intl", hint: "libicu-dev" }]);
    assert!(plan.built_in.contains(&"openssl"));
    assert!(!plan.flags.contains(&"--enable-intl=shared".to_string()));
}

#[test]
fn an_extension_needing_several_libraries_waits_for_all_of_them() {
    let plan = plan((8, 3), |package| package != "freetype2");
    assert!(plan.skipped.iter().any(|skipped| skipped.name == "gd" && skipped.hint.contains("libfreetype-dev")));
    assert!(!plan.flags.contains(&"--enable-gd".to_string()));
}

#[test]
fn the_flags_start_with_the_ones_that_need_no_library() {
    let flags = plan((8, 3), nothing).flags;
    assert_eq!(flags[0], "--disable-all");
    assert!(flags.contains(&"--with-pdo-mysql=mysqlnd".to_string()));
}

#[test]
fn a_compiler_and_make_are_required() {
    assert!(missing_tools(|_| true).is_empty());
    assert!(missing_tools(|name| name == "gcc" || name == "make").is_empty(), "gcc alone is a compiler");
    let missing = missing_tools(|_| false);
    assert_eq!(missing.len(), 2);
    assert!(missing.iter().all(|(_, package)| *package == "build-essential"));
    assert_eq!(missing_tools(|name| name == "cc").len(), 1);
}

#[test]
fn a_tool_is_found_in_the_path_only_when_it_is_executable() {
    let directory = tempfile::tempdir().unwrap();
    let tool = directory.path().join("sometool");
    fs::write(&tool, "#!/bin/sh\n").unwrap();
    let path = OsString::from(directory.path());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(find_in_path("sometool", &path), None, "not executable yet");
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    }
    assert_eq!(find_in_path("sometool", &path), Some(tool));
    assert_eq!(find_in_path("othertool", &path), None);
}

#[test]
fn the_end_of_a_log_is_what_an_error_shows() {
    assert_eq!(tail_lines("a\nb\nc\nd", 2), "c\nd");
    assert_eq!(tail_lines("a\nb", 5), "a\nb");
    assert_eq!(tail_lines("", 3), "");
}

#[test]
fn gpg_output_is_read_as_good_bad_or_unknown() {
    assert_eq!(signature_verdict("gpg: Good signature from \"Eric A Mann\" [unknown]"), Verdict::Good);
    assert_eq!(signature_verdict("gpg: BAD signature from \"x\""), Verdict::Bad);
    assert_eq!(signature_verdict("gpg: Can't check signature: No public key"), Verdict::Unknown);
    assert_eq!(signature_verdict(""), Verdict::Unknown);
}

#[test]
fn a_bad_signature_wins_over_a_good_looking_line() {
    assert_eq!(signature_verdict("Good signature\nBAD signature"), Verdict::Bad);
}

#[test]
fn what_an_extension_depends_on_is_always_enabled_with_it() {
    let flags = plan((8, 3), everything).flags;
    // With --disable-all, configure refuses these unless the dependency is named (found by a real build).
    for (needs_it, dependency) in [
        ("--enable-dom", "--with-libxml"),
        ("--enable-simplexml", "--with-libxml"),
        ("--enable-soap=shared", "--with-libxml"),
        ("--enable-gd", "--with-zlib"),
    ] {
        assert!(flags.contains(&needs_it.to_string()), "{}", needs_it);
        assert!(flags.contains(&dependency.to_string()), "{} needs {}", needs_it, dependency);
    }
}

#[test]
fn gd_is_skipped_when_zlib_is_missing_because_it_needs_it() {
    let plan = plan((8, 3), |package| package != "zlib");
    assert!(plan.skipped.iter().any(|skipped| skipped.name == "gd"));
    assert!(!plan.flags.contains(&"--enable-gd".to_string()));
}

#[test]
fn a_failed_build_shows_its_first_errors_not_the_warnings_that_follow() {
    let log = "CC a.c\nb.c:10:5: error: unknown type name 'DH'\nCC c.c\nc.c:3:1: warning: 'DH_free' is deprecated\nmake: *** [b.lo] Error 1\nd.c:9:1: warning: x is deprecated\n";
    let (what, excerpt) = failure_excerpt(log);
    assert_eq!(what, "The first errors in");
    assert!(excerpt.contains("unknown type name 'DH'") && excerpt.contains("*** [b.lo] Error 1"), "{}", excerpt);
    assert!(!excerpt.contains("deprecated"), "{}", excerpt);
}

#[test]
fn a_configure_failure_is_recognized() {
    let (what, excerpt) = failure_excerpt("checking for foo... no\nconfigure: error: DOM extension requires LIBXML extension, add --with-libxml\n");
    assert_eq!(what, "The first errors in");
    assert!(excerpt.contains("requires LIBXML"));
}

#[test]
fn only_the_first_few_errors_are_shown() {
    let log: String = (1..=20).map(|n| format!("f{n}.c:1:1: error: problem {n}\n")).collect();
    let (_, excerpt) = failure_excerpt(&log);
    assert_eq!(excerpt.lines().count(), 6);
    assert!(excerpt.contains("problem 1") && !excerpt.contains("problem 7"));
}

#[test]
fn a_log_without_recognizable_errors_shows_its_end() {
    let (what, excerpt) = failure_excerpt("one\ntwo\nthree\n");
    assert_eq!(what, "The end of");
    assert_eq!(excerpt, "one\ntwo\nthree");
}

#[test]
fn opcache_is_a_switchable_file_before_8_5_and_part_of_php_from_then_on() {
    for series in [(7, 4), (8, 1), (8, 4)] {
        let plan = plan(series, nothing);
        assert!(plan.shared.contains(&"opcache"), "{:?}", series);
        assert!(plan.flags.contains(&"--enable-opcache=shared".to_string()), "{:?}", series);
    }
    let plan = plan((8, 5), everything);
    assert!(!plan.shared.contains(&"opcache"));
    assert!(!plan.flags.iter().any(|flag| flag.contains("opcache")));
    assert_eq!(plan.shared, ["intl", "soap", "sodium"]);
}
