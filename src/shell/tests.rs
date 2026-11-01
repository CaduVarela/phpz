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
