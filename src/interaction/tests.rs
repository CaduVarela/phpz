use super::*;

#[test]
fn empty_answer_picks_the_default() {
    assert!(interpret_answer("\n", true));
    assert!(!interpret_answer("\n", false));
    assert!(interpret_answer("   ", true));
}

#[test]
fn yes_in_any_case_is_accepted() {
    for answer in ["y", "Y", "yes", "YES", " Yes \n"] {
        assert!(interpret_answer(answer, false), "{:?}", answer);
    }
}

#[test]
fn anything_else_is_a_no_even_when_yes_is_the_default() {
    for answer in ["n", "no", "nope", "yes please", "1"] {
        assert!(!interpret_answer(answer, true), "{:?}", answer);
    }
}
