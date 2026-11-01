//! The `php` shim, run for real: the phpz binary under the name `php`, in front of "versions" that
//! are shell scripts. Unix only, because a script cannot stand in for php.exe.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A phpz home with fake versions, a shims folder with the link, and a work folder.
struct World {
    home: tempfile::TempDir,
    work: tempfile::TempDir,
    shim: PathBuf,
}

/// A "php" that reports who it is, what it was given and how it was configured.
fn fake_php(version: &str) -> String {
    format!(
        "#!/bin/sh\necho \"php {version} args=[$*] PHPRC=$PHPRC SCAN=$PHP_INI_SCAN_DIR\"\nexit ${{FAKE_EXIT:-0}}\n"
    )
}

fn install_fake(home: &Path, version: &str) {
    let directory = home.join("versions").join(version);
    fs::create_dir_all(directory.join("libexec")).unwrap();
    let program = directory.join("libexec").join("php");
    fs::write(&program, fake_php(version)).unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
}

fn world() -> World {
    let home = tempfile::tempdir().unwrap();
    for version in ["8.2.35", "8.3.32", "8.4.27"] {
        install_fake(home.path(), version);
    }
    let work = tempfile::tempdir().unwrap();
    let shims = home.path().join("shims");
    fs::create_dir_all(&shims).unwrap();
    let shim = shims.join("php");
    symlink(env!("CARGO_BIN_EXE_phpz"), &shim).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_phpz"))
        .args(["use", "8.4"])
        .env("PHPZ_HOME", home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    World { home, work, shim }
}

fn php(world: &World, directory: &Path, arguments: &[&str]) -> Output {
    Command::new(&world.shim)
        .args(arguments)
        .env("PHPZ_HOME", world.home.path())
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

#[test]
fn without_a_project_the_default_version_runs() {
    let world = world();
    let output = php(&world, world.work.path(), &["-v"]);
    assert!(out(&output).starts_with("php 8.4.27 args=[-v]"), "{}", out(&output));
}

#[test]
fn a_project_pin_decides_which_version_runs() {
    let world = world();
    let project = world.work.path().join("app");
    let nested = project.join("src");
    fs::create_dir_all(&nested).unwrap();
    fs::write(project.join(".php-version"), "8.2\n").unwrap();

    assert!(out(&php(&world, &nested, &["-r", "echo 1;"])).starts_with("php 8.2.35 args=[-r echo 1;]"));
    assert!(out(&php(&world, world.work.path(), &[])).starts_with("php 8.4.27"));
}

#[test]
fn the_version_reads_its_own_ini_whatever_the_environment_says() {
    let world = world();
    fs::write(world.work.path().join(".php-version"), "8.3\n").unwrap();

    let output = Command::new(&world.shim)
        .env("PHPZ_HOME", world.home.path())
        .env("PHPRC", "/somewhere/else")
        .env("PHP_INI_SCAN_DIR", "/scoop/conf.d")
        .current_dir(world.work.path())
        .output()
        .unwrap();

    let home = world.home.path().display();
    let line = out(&output);
    assert!(line.contains(&format!("PHPRC={}/versions/8.3.32 ", home)), "{}", line);
    assert!(line.contains(&format!("SCAN={}/versions/8.3.32/conf.d", home)), "{}", line);
}

#[test]
fn arguments_with_spaces_and_the_exit_code_pass_through() {
    let world = world();
    let output = Command::new(&world.shim)
        .args(["-r", "echo 'a b';", "last"])
        .env("PHPZ_HOME", world.home.path())
        .env("FAKE_EXIT", "7")
        .current_dir(world.work.path())
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(out(&output).contains("args=[-r echo 'a b'; last]"), "{}", out(&output));
}

#[test]
fn a_pin_that_is_not_installed_stops_with_a_clear_message() {
    let world = world();
    fs::write(world.work.path().join(".php-version"), "7.4\n").unwrap();

    let output = php(&world, world.work.path(), &["-v"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(err(&output).contains("phpz install 7.4"), "{}", err(&output));
    assert!(out(&output).is_empty(), "no php may run: {}", out(&output));
}

#[test]
fn nothing_selected_stops_with_a_clear_message() {
    let world = world();
    fs::remove_file(world.home.path().join("current")).unwrap();

    let output = php(&world, world.work.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(err(&output).contains("no PHP version is active"), "{}", err(&output));
}

#[test]
fn setup_style_refresh_points_the_link_at_this_binary() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_phpz"))
        .args(["use", "8.4"])
        .env("PHPZ_HOME", home.path())
        .output()
        .unwrap();
    // No version is installed, so `use` fails, but the shims folder must not appear from a failed command.
    assert!(!output.status.success());
    assert!(!home.path().join("shims").exists());

    install_fake(home.path(), "8.4.27");
    let output = Command::new(env!("CARGO_BIN_EXE_phpz"))
        .args(["use", "8.4"])
        .env("PHPZ_HOME", home.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let link = home.path().join("shims/php");
    assert_eq!(fs::read_link(&link).unwrap(), PathBuf::from(env!("CARGO_BIN_EXE_phpz")));
}
