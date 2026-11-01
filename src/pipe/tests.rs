use super::*;

#[test]
fn failed_prints_are_recognized() {
    assert!(is_failed_print("failed printing to stdout: Broken pipe (os error 32)"));
    assert!(is_failed_print("failed printing to stderr: The pipe is being closed. (os error 232)"));
}

#[test]
fn panics_carrying_a_broken_pipe_error_are_recognized() {
    let message = "failed to write completion file: Os { code: 232, kind: BrokenPipe, message: \"The pipe is being closed.\" }";
    assert!(is_failed_print(message));
}

#[test]
fn other_panics_are_not_swallowed() {
    assert!(!is_failed_print("index out of bounds"));
    assert!(!is_failed_print("called `Option::unwrap()` on a `None` value"));
    assert!(!is_failed_print(""));
}
