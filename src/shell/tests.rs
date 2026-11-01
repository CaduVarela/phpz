use super::*;

const BLOCK: &str = "# >>> phpz >>>\nexport PATH=\"$HOME/.phpz/current/bin:$PATH\"\n# <<< phpz <<<\n";

#[test]
fn block_is_added_to_an_empty_profile() {
    assert_eq!(add_block("").unwrap(), BLOCK);
}

#[test]
fn block_is_separated_from_existing_content() {
    assert_eq!(add_block("alias a=b\n").unwrap(), format!("alias a=b\n\n{}", BLOCK));
}

#[test]
fn missing_trailing_newline_is_handled() {
    assert_eq!(add_block("alias a=b").unwrap(), format!("alias a=b\n\n{}", BLOCK));
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
