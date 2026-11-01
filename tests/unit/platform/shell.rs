use phpz::platform::shell::*;
use std::path::Path;

const BLOCK: &str = "# >>> phpz >>>\nexport PATH=\"$HOME/.phpz/shims:$PATH\"\n# <<< phpz <<<\n";

#[test]
fn block_is_added_to_an_empty_profile() {
    assert_eq!(add_block("").unwrap(), BLOCK);
}

#[test]
fn block_is_separated_from_existing_content() {
    assert_eq!(
        add_block("alias a=b\n").unwrap(),
        format!("alias a=b\n\n{}", BLOCK)
    );
}

#[test]
fn missing_trailing_newline_is_handled() {
    assert_eq!(
        add_block("alias a=b").unwrap(),
        format!("alias a=b\n\n{}", BLOCK)
    );
}

#[test]
fn block_is_never_added_twice() {
    let once = add_block("alias a=b\n").unwrap();
    assert_eq!(add_block(&once), None);
}

#[test]
fn removing_the_block_restores_the_original_content() {
    for original in ["", "alias a=b\n", "alias a=b", "a\n\nb\n"] {
        let with_block = add_block(original).unwrap();
        let restored = remove_block(&with_block).unwrap().unwrap();
        let expected = if original.is_empty() || original.ends_with('\n') {
            original.to_string()
        } else {
            format!("{}\n", original)
        };
        assert_eq!(restored, expected, "original: {:?}", original);
    }
}

#[test]
fn content_after_the_block_is_kept() {
    let content = format!("before\n\n{}after\n", BLOCK);
    assert_eq!(remove_block(&content).unwrap().unwrap(), "before\nafter\n");
}

#[test]
fn removing_without_a_block_changes_nothing() {
    assert_eq!(remove_block("alias a=b\n").unwrap(), None);
}

#[test]
fn unclosed_block_is_an_error_and_is_not_touched() {
    assert!(remove_block("x\n# >>> phpz >>>\nexport PATH=a\n").is_err());
}

#[test]
fn home_directory_is_shown_as_tilde() {
    let home = Path::new("/home/user");
    assert_eq!(
        tilde_path_with_home(Path::new("/home/user/.bashrc"), Some(home)),
        "~/.bashrc"
    );
    assert_eq!(
        tilde_path_with_home(Path::new("/etc/profile"), Some(home)),
        "/etc/profile"
    );
    assert_eq!(
        tilde_path_with_home(Path::new("/etc/profile"), None),
        "/etc/profile"
    );
}

const LEGACY_BLOCK: &str =
    "# >>> phpz >>>\nexport PATH=\"$HOME/.phpz/current/bin:$PATH\"\n# <<< phpz <<<\n";

#[test]
fn a_block_from_phpz_0_2_is_brought_up_to_date_in_place() {
    let before = format!("alias a=b\n\n{}export EDITOR=vim\n", LEGACY_BLOCK);
    let after = refresh_block(&before).unwrap().unwrap();
    assert_eq!(after, format!("alias a=b\n\n{}export EDITOR=vim\n", BLOCK));
}

#[test]
fn a_current_block_is_left_alone() {
    assert_eq!(refresh_block(&format!("x\n\n{}", BLOCK)).unwrap(), None);
    assert_eq!(refresh_block("no block here\n").unwrap(), None);
}

#[test]
fn refreshing_twice_changes_nothing_the_second_time() {
    let once = refresh_block(LEGACY_BLOCK).unwrap().unwrap();
    assert_eq!(refresh_block(&once).unwrap(), None);
}

#[test]
fn a_hand_edited_block_body_is_replaced_too() {
    let edited =
        "# >>> phpz >>>\nexport PATH=\"/somewhere/else:$PATH\"\nexport FOO=1\n# <<< phpz <<<\n";
    assert_eq!(refresh_block(edited).unwrap().unwrap(), BLOCK);
}

#[test]
fn crlf_profiles_keep_their_line_endings_when_refreshed() {
    let legacy = LEGACY_BLOCK.replace('\n', "\r\n");
    let after = refresh_block(&legacy).unwrap().unwrap();
    assert_eq!(after, BLOCK.replace('\n', "\r\n"));
}

#[test]
fn an_unclosed_block_is_an_error_when_refreshing() {
    assert!(refresh_block("# >>> phpz >>>\nexport PATH=a\n").is_err());
}

#[test]
fn undo_after_an_upgrade_restores_the_original_profile_byte_for_byte() {
    let original = "alias a=b\nexport EDITOR=vim\n";
    let with_legacy = format!("{}\n{}", original, LEGACY_BLOCK);
    let upgraded = refresh_block(&with_legacy).unwrap().unwrap();
    assert_eq!(remove_block(&upgraded).unwrap().unwrap(), original);
}

#[test]
fn only_the_current_block_counts_as_configured() {
    assert!(block_is_current(BLOCK));
    assert!(!block_is_current(LEGACY_BLOCK));
    assert!(!block_is_current("nothing\n"));
}
