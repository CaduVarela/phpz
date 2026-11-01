use phpz::platform::user_path::*;

const ENTRY: &str = r"C:\Users\me\.phpz\current";

#[test]
fn entry_is_placed_first() {
    assert_eq!(
        add_entry(r"C:\Windows;C:\Tools", ENTRY).unwrap(),
        format!(r"{};C:\Windows;C:\Tools", ENTRY)
    );
}

#[test]
fn entry_is_added_to_an_empty_path() {
    assert_eq!(add_entry("", ENTRY).unwrap(), ENTRY);
}

#[test]
fn entry_already_present_is_not_added_again() {
    assert_eq!(add_entry(&format!(r"C:\Windows;{}", ENTRY), ENTRY), None);
}

#[test]
fn matching_ignores_case_and_a_trailing_backslash() {
    assert_eq!(
        add_entry(r"c:\users\ME\.PHPZ\current\;C:\Tools", ENTRY),
        None
    );
}

#[test]
fn similar_looking_entries_are_not_confused() {
    assert!(add_entry(r"C:\Users\me\.phpz\current-old;C:\Users\me\.phpz", ENTRY).is_some());
}

#[test]
fn removing_the_entry_keeps_every_other_entry_in_order() {
    let path = format!(r"{};C:\Windows;C:\Tools", ENTRY);
    assert_eq!(remove_entry(&path, ENTRY).unwrap(), r"C:\Windows;C:\Tools");
}

#[test]
fn removing_a_missing_entry_changes_nothing() {
    assert_eq!(remove_entry(r"C:\Windows;C:\Tools", ENTRY), None);
}

#[test]
fn add_then_remove_restores_the_original_path_exactly() {
    for original in [
        "",
        r"C:\Windows",
        r"C:\Windows;C:\Tools",
        r"C:\Windows;C:\Tools;",
        r"%USERPROFILE%\bin;C:\Tools",
        r"C:\a;;C:\b",
    ] {
        let added = add_entry(original, ENTRY).unwrap();
        assert_eq!(
            remove_entry(&added, ENTRY).unwrap(),
            original,
            "{:?}",
            original
        );
    }
}

#[test]
fn variables_like_userprofile_are_left_untouched() {
    let path = r"%USERPROFILE%\scoop\shims;%SystemRoot%\system32";
    let updated = add_entry(path, ENTRY).unwrap();
    assert!(updated.ends_with(path));
}
