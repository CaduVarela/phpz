use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// Run phpz inside `directory` with its own phpz home, so nothing outside the temporary folders is touched.
fn phpz(home: &Path, directory: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_phpz"))
        .args(arguments)
        .env("PHPZ_HOME", home)
        .current_dir(directory)
        .output()
        .unwrap()
}

fn out(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn err(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

/// A phpz home with three "installed" versions and 8.4 as the default.
fn setup() -> (tempfile::TempDir, tempfile::TempDir) {
    let home = tempfile::tempdir().unwrap();
    for version in ["8.2.35", "8.3.32", "8.4.27"] {
        fs::create_dir_all(home.path().join("versions").join(version)).unwrap();
    }
    let work = tempfile::tempdir().unwrap();
    let output = phpz(home.path(), work.path(), &["use", "8.4"]);
    assert!(output.status.success(), "{}", err(&output));
    (home, work)
}

#[test]
fn a_version_file_changes_the_version_for_its_folder_and_below_only() {
    let (home, work) = setup();
    let project = work.path().join("project");
    let nested = project.join("src/deep");
    fs::create_dir_all(&nested).unwrap();
    fs::write(project.join(".php-version"), "8.2\n").unwrap();

    assert_eq!(out(&phpz(home.path(), &nested, &["current"])), "8.2.35");
    assert_eq!(out(&phpz(home.path(), &project, &["current"])), "8.2.35");
    assert_eq!(out(&phpz(home.path(), work.path(), &["current"])), "8.4.27");
}

#[test]
fn local_pins_shows_and_unpins() {
    let (home, work) = setup();

    let pinned = phpz(home.path(), work.path(), &["local", "8.3"]);
    assert!(pinned.status.success(), "{}", err(&pinned));
    assert_eq!(
        fs::read_to_string(work.path().join(".php-version")).unwrap(),
        "8.3\n"
    );
    assert_eq!(out(&phpz(home.path(), work.path(), &["current"])), "8.3.32");

    let shown = out(&phpz(home.path(), work.path(), &["local"]));
    assert!(
        shown.starts_with("PHP 8.3 (set by ") && shown.contains("installed as 8.3.32"),
        "{}",
        shown
    );

    let removed = phpz(home.path(), work.path(), &["local", "--unset"]);
    assert!(removed.status.success());
    assert!(!work.path().join(".php-version").exists());
    assert_eq!(out(&phpz(home.path(), work.path(), &["current"])), "8.4.27");
    assert!(
        out(&phpz(home.path(), work.path(), &["local"])).starts_with("No project version applies")
    );
}

#[test]
fn local_warns_about_a_version_that_is_not_installed_but_still_pins_it() {
    let (home, work) = setup();
    let output = phpz(home.path(), work.path(), &["local", "7.4"]);
    assert!(output.status.success());
    assert!(
        out(&output).contains("not installed yet") && out(&output).contains("phpz install 7.4")
    );
    assert_eq!(
        fs::read_to_string(work.path().join(".php-version")).unwrap(),
        "7.4\n"
    );
}

#[test]
fn local_refuses_nonsense_and_a_project_file_that_already_sets_php() {
    let (home, work) = setup();
    let nonsense = phpz(home.path(), work.path(), &["local", "banana"]);
    assert!(!nonsense.status.success());
    assert!(err(&nonsense).contains("invalid version"));
    assert!(!work.path().join(".php-version").exists());

    fs::write(work.path().join(".phpz.toml"), "php = \"8.3\"\n").unwrap();
    let blocked = phpz(home.path(), work.path(), &["local", "8.2"]);
    assert!(!blocked.status.success());
    assert!(err(&blocked).contains(".phpz.toml"));
}

#[test]
fn the_project_file_beats_the_version_file() {
    let (home, work) = setup();
    fs::write(work.path().join(".php-version"), "8.2\n").unwrap();
    fs::write(work.path().join(".phpz.toml"), "php = \"8.3\"\n").unwrap();
    assert_eq!(out(&phpz(home.path(), work.path(), &["current"])), "8.3.32");
}

#[test]
fn a_pin_that_is_not_installed_is_an_error_and_never_falls_back() {
    let (home, work) = setup();
    fs::write(work.path().join(".php-version"), "7.4\n").unwrap();

    let current = phpz(home.path(), work.path(), &["current"]);
    assert!(!current.status.success());
    assert!(
        err(&current).contains("phpz install 7.4"),
        "{}",
        err(&current)
    );

    let which = phpz(home.path(), work.path(), &["which"]);
    assert!(!which.status.success());
}

#[test]
fn which_prints_the_php_that_would_run_here() {
    let (home, work) = setup();
    fs::write(work.path().join(".php-version"), "8.3\n").unwrap();

    let path = out(&phpz(home.path(), work.path(), &["which"]));
    let expected_name = if cfg!(windows) { "php.exe" } else { "php" };
    assert!(
        path.contains("8.3.32") && path.ends_with(expected_name),
        "{}",
        path
    );
}

#[test]
fn list_marks_what_applies_here_and_the_default() {
    let (home, work) = setup();
    fs::write(work.path().join(".php-version"), "8.2\n").unwrap();

    let listing = out(&phpz(home.path(), work.path(), &["list"]));
    assert!(listing.contains("* 8.2.35 (active, set by "), "{}", listing);
    assert!(listing.contains("  8.4.27 (default)"), "{}", listing);
    assert!(listing.contains("  8.3.32"), "{}", listing);
}

#[test]
fn nothing_selected_is_reported_without_failing_current() {
    let home = tempfile::tempdir().unwrap();
    let work = tempfile::tempdir().unwrap();
    let output = phpz(home.path(), work.path(), &["current"]);
    assert!(output.status.success());
    assert!(out(&output).contains("No PHP version is active"));
}
