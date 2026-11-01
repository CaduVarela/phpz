// Combinations of `install` options that are refused before anything is downloaded.
//
// Not named after "install": Windows asks for administrator rights to run an executable
// whose name has that word (or "setup", or "update") and no manifest, so the test would not run.

use std::process::{Command, Output};

fn phpz(arguments: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_phpz"))
        .args(arguments)
        .env("PHPZ_HOME", home.path())
        .output()
        .unwrap()
}

fn err(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn a_prebuilt_build_and_a_compile_cannot_be_asked_for_together() {
    let output = phpz(&["install", "8.3", "--build", "--variant", "bulk"]);
    assert!(!output.status.success());
    assert!(
        err(&output).contains("cannot be used with"),
        "{}",
        err(&output)
    );
}

#[test]
fn jobs_only_make_sense_when_compiling() {
    let output = phpz(&["install", "8.3", "--jobs", "4"]);
    assert!(!output.status.success());
    assert!(err(&output).contains("--build"), "{}", err(&output));
}

#[test]
fn replace_needs_to_know_what_to_replace_the_version_with() {
    let output = phpz(&["install", "8.3", "--replace"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        err(&output).contains("--replace needs --variant"),
        "{}",
        err(&output)
    );
}

#[test]
fn an_unknown_build_name_lists_the_choices() {
    let output = phpz(&["install", "8.3", "--variant", "huge"]);
    assert!(!output.status.success());
    assert!(
        err(&output).contains("minimal, common, bulk, gnu-bulk"),
        "{}",
        err(&output)
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn compiling_is_for_linux_only_and_says_so_without_asking_the_network() {
    let output = phpz(&["install", "8.3", "--build"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(err(&output).contains("Linux only"), "{}", err(&output));
}
