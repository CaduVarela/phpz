use phpz::download::lock::*;

const URL: &str = "https://example.test/php_x-1.0-8.3-nts-vs16-x64.zip";

#[test]
fn nothing_is_known_at_first() {
    assert_eq!(DownloadLock::default().get(URL), None);
    assert_eq!(DownloadLock::parse("").unwrap(), DownloadLock::default());
}

#[test]
fn a_recorded_checksum_is_returned_in_lowercase() {
    let mut lock = DownloadLock::default();
    lock.record(URL, "ABCDEF");
    assert_eq!(lock.get(URL), Some("abcdef"));
    assert_eq!(lock.get("https://example.test/other.zip"), None);
}

#[test]
fn the_file_survives_a_render_and_parse_round_trip() {
    let mut lock = DownloadLock::default();
    lock.record(URL, "abcdef");
    lock.record("https://example.test/b.zip", "123456");
    assert_eq!(DownloadLock::parse(&lock.render()).unwrap(), lock);
    assert!(lock.render().starts_with("# SHA-256"));
}

#[test]
fn an_invalid_file_is_an_error() {
    assert!(DownloadLock::parse("[downloads\nx =").is_err());
}
