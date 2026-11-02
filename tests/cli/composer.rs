// The `composer` shim and `phpz composer install`, run for real against fake versions and a small
// local server standing in for getcomposer.org. Unix only, because a script cannot stand in for php.exe.
#![cfg(unix)]

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

fn install_fake_php(home: &Path, version: &str) {
    let directory = home.join("versions").join(version);
    fs::create_dir_all(directory.join("libexec")).unwrap();
    let program = directory.join("libexec").join("php");
    fs::write(
        &program,
        format!("#!/bin/sh\necho \"php {version} args=[$*]\"\n"),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
}

fn phpz(home: &Path, arguments: &[&str], extra: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_phpz"));
    command.args(arguments).env("PHPZ_HOME", home);
    for (name, value) in extra {
        command.env(name, value);
    }
    let mut output = command.output().unwrap();
    for _ in 0..10 {
        if !String::from_utf8_lossy(&output.stderr).contains("Text file busy") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        output = command.output().unwrap();
    }
    output
}

fn out(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn err(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

fn composer_shim(home: &Path) -> std::path::PathBuf {
    let shims = home.join("shims");
    fs::create_dir_all(&shims).unwrap();
    let shim = shims.join("composer");
    symlink(env!("CARGO_BIN_EXE_phpz"), &shim).unwrap();
    shim
}

/// Answers every request with the file for its path, or 404, until the test ends.
fn serve(files: Vec<(&'static str, Vec<u8>)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buffer = [0u8; 4096];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            let path = request.split_whitespace().nth(1).unwrap_or("");
            let (status, body) = match files.iter().find(|(name, _)| *name == path) {
                Some((_, body)) => ("200 OK", body.clone()),
                None => ("404 Not Found", Vec::new()),
            };
            let head = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                status,
                body.len()
            );
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&body);
        }
    });
    format!("http://{}", address)
}

fn versions_list() -> Vec<u8> {
    br#"{"2": [{"path": "/download/2.9.9/composer.phar"}], "2.2": [{"path": "/download/2.2.31/composer.phar"}]}"#
        .to_vec()
}

#[test]
fn composer_runs_on_the_php_of_the_project() {
    let home = tempfile::tempdir().unwrap();
    install_fake_php(home.path(), "8.4.27");
    install_fake_php(home.path(), "8.2.35");
    let phars = home.path().join("composer");
    fs::create_dir_all(&phars).unwrap();
    fs::write(phars.join("composer-2.phar"), "phar").unwrap();
    let shim = composer_shim(home.path());
    assert!(phpz(home.path(), &["use", "8.4"], &[]).status.success());

    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join(".php-version"), "8.2\n").unwrap();
    let output = Command::new(&shim)
        .args(["install", "--no-dev"])
        .env("PHPZ_HOME", home.path())
        .current_dir(work.path())
        .output()
        .unwrap();

    let phar = phars.join("composer-2.phar");
    assert_eq!(
        out(&output),
        format!("php 8.2.35 args=[{} install --no-dev]", phar.display())
    );
}

#[test]
fn composer_that_is_not_installed_says_how_to_install_it() {
    let home = tempfile::tempdir().unwrap();
    install_fake_php(home.path(), "8.4.27");
    let shim = composer_shim(home.path());
    assert!(phpz(home.path(), &["use", "8.4"], &[]).status.success());

    let output = Command::new(&shim)
        .env("PHPZ_HOME", home.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        err(&output).contains("phpz composer install"),
        "{}",
        err(&output)
    );
}

#[test]
fn installing_downloads_a_verified_composer_and_puts_the_shim_in_place() {
    let home = tempfile::tempdir().unwrap();
    install_fake_php(home.path(), "8.4.27");
    let phar = b"the phar".to_vec();
    let hash = format!("{:x}", Sha256::digest(&phar));
    let base = serve(vec![
        ("/versions", versions_list()),
        ("/download/2.9.9/composer.phar", phar.clone()),
        (
            "/download/2.9.9/composer.phar.sha256sum",
            format!("{}  composer.phar\n", hash).into_bytes(),
        ),
    ]);

    let output = phpz(
        home.path(),
        &["composer", "install"],
        &[("PHPZ_COMPOSER_URL", &base)],
    );

    assert!(output.status.success(), "{}", err(&output));
    assert_eq!(
        fs::read(home.path().join("composer").join("composer-2.phar")).unwrap(),
        phar
    );
    assert!(home.path().join("shims").join("composer").exists());
}

#[test]
fn a_download_that_does_not_match_its_checksum_is_discarded() {
    let home = tempfile::tempdir().unwrap();
    install_fake_php(home.path(), "8.4.27");
    let base = serve(vec![
        ("/versions", versions_list()),
        ("/download/2.9.9/composer.phar", b"tampered".to_vec()),
        (
            "/download/2.9.9/composer.phar.sha256sum",
            format!("{}  composer.phar\n", "0".repeat(64)).into_bytes(),
        ),
    ]);

    let output = phpz(
        home.path(),
        &["composer", "install"],
        &[("PHPZ_COMPOSER_URL", &base)],
    );

    assert!(!output.status.success());
    assert!(
        err(&output).contains("checksum mismatch"),
        "{}",
        err(&output)
    );
    assert!(!home
        .path()
        .join("composer")
        .join("composer-2.phar")
        .exists());
    assert!(!home.path().join("shims").join("composer").exists());
}

#[test]
fn an_old_php_gets_the_long_term_support_line() {
    let home = tempfile::tempdir().unwrap();
    install_fake_php(home.path(), "7.1.33");
    let phar = b"old phar".to_vec();
    let hash = format!("{:x}", Sha256::digest(&phar));
    let base = serve(vec![
        ("/versions", versions_list()),
        ("/download/2.2.31/composer.phar", phar),
        (
            "/download/2.2.31/composer.phar.sha256sum",
            format!("{}  composer.phar\n", hash).into_bytes(),
        ),
    ]);

    let output = phpz(
        home.path(),
        &["composer", "install"],
        &[("PHPZ_COMPOSER_URL", &base)],
    );

    assert!(output.status.success(), "{}", err(&output));
    assert!(home
        .path()
        .join("composer")
        .join("composer-2.2.phar")
        .exists());
    assert!(!home
        .path()
        .join("composer")
        .join("composer-2.phar")
        .exists());
}
