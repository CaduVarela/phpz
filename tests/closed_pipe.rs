use std::process::{Command, Stdio};

/// Closing our end of the pipe before the program writes must not produce a panic message.
#[test]
fn closed_output_pipe_does_not_print_a_panic() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_phpz"))
        .args(["completions", "bash"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());

    let output = child.wait_with_output().unwrap();
    let errors = String::from_utf8_lossy(&output.stderr);

    assert!(!errors.contains("panicked"), "{}", errors);
    assert!(!errors.contains("Broken pipe"), "{}", errors);
}
