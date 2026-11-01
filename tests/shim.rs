// The `php` shim, run for real: the phpz binary under the name `php`, in front of "versions" that
// are shell scripts. Unix only, because a script cannot stand in for php.exe.
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
    assert!(
        out(&output).starts_with("php 8.4.27 args=[-v]"),
        "{}",
        out(&output)
    );
}

#[test]
fn a_project_pin_decides_which_version_runs() {
    let world = world();
    let project = world.work.path().join("app");
    let nested = project.join("src");
    fs::create_dir_all(&nested).unwrap();
    fs::write(project.join(".php-version"), "8.2\n").unwrap();

    assert!(
        out(&php(&world, &nested, &["-r", "echo 1;"])).starts_with("php 8.2.35 args=[-r echo 1;]")
    );
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
    assert!(
        line.contains(&format!("PHPRC={}/versions/8.3.32 ", home)),
        "{}",
        line
    );
    assert!(
        line.contains(&format!("SCAN={}/versions/8.3.32/conf.d", home)),
        "{}",
        line
    );
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
    assert!(
        out(&output).contains("args=[-r echo 'a b'; last]"),
        "{}",
        out(&output)
    );
}

#[test]
fn a_pin_that_is_not_installed_stops_with_a_clear_message() {
    let world = world();
    fs::write(world.work.path().join(".php-version"), "7.4\n").unwrap();

    let output = php(&world, world.work.path(), &["-v"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        err(&output).contains("phpz install 7.4"),
        "{}",
        err(&output)
    );
    assert!(out(&output).is_empty(), "no php may run: {}", out(&output));
}

#[test]
fn nothing_selected_stops_with_a_clear_message() {
    let world = world();
    fs::remove_file(world.home.path().join("current")).unwrap();

    let output = php(&world, world.work.path(), &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        err(&output).contains("no PHP version is active"),
        "{}",
        err(&output)
    );
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
    assert_eq!(
        fs::read_link(&link).unwrap(),
        PathBuf::from(env!("CARGO_BIN_EXE_phpz"))
    );
}

/// Give 8.3.32 an ext folder with intl and curl DLLs, a php.ini, and a php that prints the
/// extension lines of the php.ini it was told to read.
fn prepare_extension_version(home: &Path) {
    let version = home.join("versions/8.3.32");
    fs::create_dir_all(version.join("ext")).unwrap();
    for name in ["intl", "curl"] {
        fs::write(version.join("ext").join(format!("php_{}.dll", name)), "").unwrap();
    }
    fs::write(
        version.join("php.ini"),
        "[PHP]\nextension_dir = \"ext\"\n;extension=intl\nextension=curl\n",
    )
    .unwrap();

    let script = "#!/bin/sh\necho \"PHPRC=$PHPRC\"\ngrep -h '^extension' \"$PHPRC/php.ini\"\n";
    for relative in ["libexec/php", "bin/php"] {
        let program = version.join(relative);
        fs::create_dir_all(program.parent().unwrap()).unwrap();
        fs::write(&program, script).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
fn a_projects_extension_choices_reach_php_through_a_generated_ini() {
    let world = world();
    prepare_extension_version(world.home.path());
    fs::write(
        world.work.path().join(".phpz.toml"),
        "php = \"8.3\"\n\n[extensions]\nenable = [\"intl\"]\ndisable = [\"curl\"]\n",
    )
    .unwrap();

    let lines = out(&php(&world, world.work.path(), &[]));
    assert!(
        lines.contains("/projects/"),
        "php must read a generated ini: {}",
        lines
    );
    assert!(lines.contains("extension=intl"), "{}", lines);
    assert!(!lines.contains("extension=curl"), "{}", lines);

    let version_ini =
        fs::read_to_string(world.home.path().join("versions/8.3.32/php.ini")).unwrap();
    assert!(
        version_ini.contains("\nextension=curl\n") && version_ini.contains(";extension=intl"),
        "the version's own php.ini must stay as it was"
    );
}

#[test]
fn a_project_that_only_pins_a_version_keeps_the_versions_own_ini() {
    let world = world();
    prepare_extension_version(world.home.path());
    fs::write(world.work.path().join(".php-version"), "8.3\n").unwrap();

    let lines = out(&php(&world, world.work.path(), &[]));
    assert!(
        lines.contains("PHPRC=") && lines.contains("/versions/8.3.32"),
        "{}",
        lines
    );
    assert!(!lines.contains("/projects/"), "{}", lines);
}

#[test]
fn another_folder_does_not_see_the_projects_choices() {
    let world = world();
    prepare_extension_version(world.home.path());
    fs::write(
        world.work.path().join(".phpz.toml"),
        "php = \"8.3\"\n[extensions]\nenable = [\"intl\"]\n",
    )
    .unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    fs::write(elsewhere.path().join(".php-version"), "8.3\n").unwrap();

    let lines = out(&php(&world, elsewhere.path(), &[]));
    assert!(!lines.contains("extension=intl"), "{}", lines);
}

#[test]
fn sync_project_reports_what_the_version_lacks() {
    let world = world();
    prepare_extension_version(world.home.path());
    fs::write(
        world.work.path().join(".phpz.toml"),
        "php = \"8.3\"\n[extensions]\nenable = [\"intl\", \"redis\"]\n",
    )
    .unwrap();

    let run = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_phpz"))
            .args(arguments)
            .env("PHPZ_HOME", world.home.path())
            .current_dir(world.work.path())
            .output()
            .unwrap()
    };

    let report = run(&["ext", "sync", "--project"]);
    assert!(report.status.success(), "{}", err(&report));
    assert!(out(&report).contains("lacks: redis"), "{}", out(&report));

    let install = run(&["ext", "sync", "--project", "--install"]);
    assert!(!install.status.success());
    assert!(
        err(&install).contains("only available on Windows"),
        "{}",
        err(&install)
    );

    let list = out(&run(&["ext", "list"]));
    assert!(list.contains("enables intl, redis"), "{}", list);
}

#[test]
fn sync_project_without_extension_choices_says_so() {
    let world = world();
    fs::write(world.work.path().join(".php-version"), "8.3\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_phpz"))
        .args(["ext", "sync", "--project"])
        .env("PHPZ_HOME", world.home.path())
        .current_dir(world.work.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        out(&output).contains("No extension choices apply here"),
        "{}",
        out(&output)
    );
}
