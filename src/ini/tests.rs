use super::*;

const SAMPLE: &str = include_str!("../../tests/fixtures/sample.ini");

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
