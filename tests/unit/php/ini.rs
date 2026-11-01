use phpz::php::ini::*;

const SAMPLE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/sample.ini"
));

#[test]
fn render_round_trips_untouched_content() {
    assert_eq!(IniDocument::parse(SAMPLE).render(), SAMPLE);
}

#[test]
fn get_strips_inline_comments_and_quotes() {
    let document = IniDocument::parse("[PHP]\na = 1 ; note\nb = \"two words\"\n");
    assert_eq!(document.get("PHP", "a").as_deref(), Some("1"));
    assert_eq!(document.get("PHP", "b").as_deref(), Some("two words"));
    assert_eq!(document.get("PHP", "missing"), None);
}

#[test]
fn get_ignores_commented_lines_and_other_sections() {
    let document = IniDocument::parse("[PHP]\n;a = 1\n[Date]\na = 2\n");
    assert_eq!(document.get("PHP", "a"), None);
    assert_eq!(document.get("Date", "a").as_deref(), Some("2"));
}

#[test]
fn set_replaces_existing_line_and_keeps_everything_else() {
    let mut document = IniDocument::parse(SAMPLE);
    document.set("PHP", "memory_limit", "512M");
    let expected = SAMPLE.replace("memory_limit = 128M ; old value", "memory_limit = 512M");
    assert_eq!(document.render(), expected);
}

#[test]
fn set_inserts_new_key_at_end_of_section() {
    let mut document = IniDocument::parse(SAMPLE);
    document.set("PHP", "max_execution_time", "60");
    let expected = SAMPLE.replace(
        "display_errors = Off\n",
        "display_errors = Off\nmax_execution_time = 60\n",
    );
    assert_eq!(document.render(), expected);
}

#[test]
fn set_creates_missing_section() {
    let mut document = IniDocument::parse("[Date]\ndate.timezone = UTC\n");
    document.set("PHP", "memory_limit", "1G");
    assert_eq!(
        document.render(),
        "[Date]\ndate.timezone = UTC\n\n[PHP]\nmemory_limit = 1G\n"
    );
}

#[test]
fn remove_deletes_all_repeated_definitions() {
    let mut document = IniDocument::parse("[PHP]\nextension=a\nx=1\nextension=a\n");
    assert_eq!(document.remove("PHP", "extension"), 2);
    assert_eq!(document.render(), "[PHP]\nx=1\n");
}

#[test]
fn crlf_line_endings_are_preserved() {
    let mut document = IniDocument::parse("[PHP]\r\na = 1\r\n");
    document.set("PHP", "b", "2");
    assert_eq!(document.render(), "[PHP]\r\na = 1\r\nb = 2\r\n");
}

const EXTENSIONS: &str = "\
[PHP]
; Dynamic Extensions
;extension=bz2
;extension=curl
extension=gd
; extension=ldap is only prose
;extension=mbstring
zend_extension=opcache
;zend_extension=xdebug

[Date]
;extension=notmine
";

fn is(name: &str) -> impl Fn(&str) -> bool + '_ {
    move |value| value == name
}

#[test]
fn values_lists_every_active_definition_in_order() {
    let document = IniDocument::parse(EXTENSIONS);
    assert_eq!(document.values("PHP", "extension"), ["gd"]);
    assert_eq!(document.values("PHP", "zend_extension"), ["opcache"]);
    assert!(document.values("Date", "extension").is_empty());
}

#[test]
fn uncomment_activates_the_matching_commented_line_only() {
    let mut document = IniDocument::parse(EXTENSIONS);
    assert!(document.uncomment_entry("PHP", "extension", is("curl")));
    assert_eq!(document.values("PHP", "extension"), ["curl", "gd"]);
    assert!(document
        .render()
        .contains(";extension=bz2\nextension=curl\nextension=gd\n"));
}

#[test]
fn uncomment_never_activates_prose_comments_or_other_sections() {
    let mut document = IniDocument::parse(EXTENSIONS);
    assert!(!document.uncomment_entry("PHP", "extension", is("ldap")));
    assert!(!document.uncomment_entry("PHP", "extension", is("notmine")));
    assert!(!document.uncomment_entry("PHP", "extension", is("missing")));
    assert_eq!(document.render(), EXTENSIONS);
}

#[test]
fn uncomment_works_for_zend_extension_keys() {
    let mut document = IniDocument::parse(EXTENSIONS);
    assert!(document.uncomment_entry("PHP", "zend_extension", is("xdebug")));
    assert_eq!(
        document.values("PHP", "zend_extension"),
        ["opcache", "xdebug"]
    );
}

#[test]
fn comment_out_deactivates_only_the_matching_lines() {
    let mut document = IniDocument::parse(EXTENSIONS);
    assert_eq!(
        document.comment_out_entries("PHP", "extension", is("gd")),
        1
    );
    assert!(document.values("PHP", "extension").is_empty());
    assert!(document.render().contains(";extension=gd\n"));
    assert_eq!(
        document.comment_out_entries("PHP", "extension", is("gd")),
        0
    );
}

#[test]
fn comment_out_then_uncomment_restores_the_file() {
    let mut document = IniDocument::parse(EXTENSIONS);
    document.comment_out_entries("PHP", "extension", is("gd"));
    assert!(document.uncomment_entry("PHP", "extension", is("gd")));
    assert_eq!(document.render(), EXTENSIONS);
}

#[test]
fn new_entry_goes_after_the_last_line_of_the_same_key() {
    let mut document = IniDocument::parse(EXTENSIONS);
    document.add_entry_near("PHP", "extension", "ffi");
    assert!(document
        .render()
        .contains(";extension=mbstring\nextension=ffi\nzend_extension=opcache\n"));
}

#[test]
fn new_entry_without_siblings_goes_to_the_end_of_its_section() {
    let mut document = IniDocument::parse("[PHP]\nmemory_limit=1G\n\n[Date]\nx=1\n");
    document.add_entry_near("PHP", "extension", "ffi");
    assert_eq!(
        document.render(),
        "[PHP]\nmemory_limit=1G\nextension=ffi\n\n[Date]\nx=1\n"
    );
}

#[test]
fn new_entry_creates_the_section_when_missing() {
    let mut document = IniDocument::parse("[Date]\nx=1\n");
    document.add_entry_near("PHP", "extension", "ffi");
    assert_eq!(document.render(), "[Date]\nx=1\n\n[PHP]\nextension=ffi\n");
}

#[test]
fn extension_edits_keep_crlf_line_endings() {
    let mut document = IniDocument::parse("[PHP]\r\n;extension=curl\r\n");
    document.uncomment_entry("PHP", "extension", is("curl"));
    document.add_entry_near("PHP", "extension", "gd");
    assert_eq!(
        document.render(),
        "[PHP]\r\nextension=curl\r\nextension=gd\r\n"
    );
}
