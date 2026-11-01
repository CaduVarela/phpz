use std::fs::{self, File};
use std::io;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::version::Build;
use crate::{download, extensions, interaction, paths, shell, version};

const DEFAULT_INI: &str = "\
; php.ini managed by phpz
[PHP]
memory_limit = 512M
error_reporting = E_ALL
display_errors = On
date.timezone = UTC
";

/// Install a specific PHP version
pub fn run(version: String, allow_unverified: bool) {
    if let Err(error) = install(&version, allow_unverified) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn install(requested_version: &str, allow_unverified: bool) -> Result<(), String> {
    println!("Resolving version '{}'...", requested_version);
    let build = version::resolve_build(requested_version)?;
    let resolved_version = build.version_name();

    let versions_directory = paths::versions_directory()?;
    let final_directory = versions_directory.join(&resolved_version);
    let temporary_directory = versions_directory.join(format!("{}.tmp", resolved_version));

    fs::create_dir_all(&versions_directory)
        .map_err(|error| format!("creating versions directory: {}", error))?;

    if final_directory.exists() {
        println!("PHP {} is already installed.", resolved_version);
        return Ok(());
    }

    confirm_unverified(&build, allow_unverified)?;

    // Clean up leftovers from a previous failed attempt
    if temporary_directory.exists() {
        let _ = fs::remove_dir_all(&temporary_directory);
    }
    fs::create_dir_all(&temporary_directory)
        .map_err(|error| format!("creating temporary directory: {}", error))?;

    println!("Installing PHP {}", resolved_version);
    let result = populate(&build, &temporary_directory).and_then(|_| {
        apply_global_extension_settings(&temporary_directory);
        fs::rename(&temporary_directory, &final_directory)
            .map_err(|error| format!("finalizing installation: {}", error))
    });

    if let Err(error) = result {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error);
    }

    println!("Successfully installed PHP {}", resolved_version);
    shell::offer_setup();
    Ok(())
}

/// Best effort: a problem with the saved settings must never fail an install.
fn apply_global_extension_settings(directory: &Path) {
    match extensions::apply_global_settings(directory) {
        Ok(Some(report)) if !report.describe().is_empty() => {
            println!("Applied your global extension settings:");
            for line in report.describe() {
                println!("  {}", line);
            }
        }
        Ok(_) => {}
        Err(error) => eprintln!("Note: could not apply your global extension settings: {}", error),
    }
}

/// What to do about a build whose checksum is missing.
#[derive(Debug, PartialEq)]
enum Consent {
    NotNeeded,
    Given,
    Ask,
    Refused,
}

/// Only a build that should have had a checksum needs consent. Scripts cannot answer a question,
/// so they must opt in with `--allow-unverified`.
fn consent_for(missing_checksum_is_unusual: bool, allow_unverified: bool, interactive: bool) -> Consent {
    if !missing_checksum_is_unusual {
        Consent::NotNeeded
    } else if allow_unverified {
        Consent::Given
    } else if interactive {
        Consent::Ask
    } else {
        Consent::Refused
    }
}

fn confirm_unverified(build: &Build, allow_unverified: bool) -> Result<(), String> {
    let unusual = build.expects_checksum && build.sha256.is_none();
    let version = build.version_name();

    match consent_for(unusual, allow_unverified, interaction::is_interactive()) {
        Consent::NotNeeded | Consent::Given => Ok(()),
        Consent::Refused => Err(format!(
            "PHP {} has no published checksum, so the download cannot be verified. \
             Run phpz in a terminal to confirm, or pass --allow-unverified.",
            version
        )),
        Consent::Ask => {
            eprintln!("PHP {} has no published checksum, so the download cannot be verified.", version);
            eprintln!("It is fetched over HTTPS from the official site, but nothing proves the file is intact.");
            match interaction::confirm("Install it anyway?", false) {
                Some(true) => Ok(()),
                _ => Err("installation cancelled".to_string()),
            }
        }
    }
}

fn populate(build: &Build, directory: &Path) -> Result<(), String> {
    if cfg!(windows) {
        lay_out_windows_build(build, directory)
    } else {
        lay_out_static_build(build, directory)
    }
}

/// Extensions enabled by default in the Windows php.ini, when the build ships them.
/// "gd2" is how PHP 7.0 to 7.3 name GD; only the spelling the template has gets activated.
const WINDOWS_DEFAULT_EXTENSIONS: &[&str] = &[
    "bz2", "curl", "exif", "fileinfo", "gd", "gd2", "gmp", "intl", "mbstring", "mysqli", "openssl",
    "pdo_mysql", "pdo_pgsql", "pdo_sqlite", "pgsql", "sockets", "sodium", "sqlite3", "zip",
];

/// Official Windows zip, extracted as it comes:
///   php.exe, DLLs, ext\   from the zip
///   php.ini               php.ini-development with extension_dir and common extensions enabled
fn lay_out_windows_build(build: &Build, directory: &Path) -> Result<(), String> {
    let archive_path = directory.join("download.zip");
    download::fetch(&build.url, &archive_path, build.sha256.as_deref())?;
    download::announce_checksum(build.sha256.is_some());
    extract_zip(&archive_path, directory)?;
    let _ = fs::remove_file(&archive_path);

    let template_path = directory.join("php.ini-development");
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("reading {}: {}", template_path.display(), error))?;

    // PHP 7.0 and 7.1 name extensions by DLL ("extension=php_curl.dll"); 7.2 and later use short names.
    let directives = windows_directives(!build.at_least(7, 2));
    fs::write(directory.join("php.ini"), uncomment_directives(&template, &directives))
        .map_err(|error| format!("writing php.ini: {}", error))
}

/// The php.ini lines to activate: extension_dir and the default extensions.
fn windows_directives(uses_dll_names: bool) -> Vec<String> {
    let mut directives = vec!["extension_dir = \"ext\"".to_string()];
    directives.extend(WINDOWS_DEFAULT_EXTENSIONS.iter().map(|name| {
        if uses_dll_names {
            format!("extension=php_{}.dll", name)
        } else {
            format!("extension={}", name)
        }
    }));
    directives
}

/// Turn `;directive` lines into active `directive` lines, ignoring a trailing `; comment`
/// when matching. PHP 7.0 to 7.2 write `; extension_dir` with a space, so that one directive
/// also matches that way; other lines must match exactly so prose comments stay comments.
/// Anything else is left alone.
fn uncomment_directives(content: &str, directives: &[String]) -> String {
    content
        .split_inclusive('\n')
        .map(|line| {
            let text = line.trim_end_matches(['\r', '\n']);
            let is_wanted = text
                .strip_prefix(';')
                .map(|rest| rest.split(';').next().unwrap_or("").trim_end())
                .map(|rest| {
                    directives.iter().any(|wanted| {
                        wanted == rest
                            || (wanted.starts_with("extension_dir") && wanted == rest.trim_start())
                    })
                })
                .unwrap_or(false);
            if is_wanted {
                line[1..].trim_start().to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}

/// Extract every entry of the zip below `destination`, refusing paths that escape it.
fn extract_zip(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("reading archive: {}", error))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("reading archive: {}", error))?;
        let relative_path = entry
            .enclosed_name()
            .map(Path::to_path_buf)
            .ok_or_else(|| format!("unsafe path in archive: {}", entry.name()))?;
        let target = destination.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = File::create(&target)
            .map_err(|error| format!("creating {}: {}", target.display(), error))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("extracting {}: {}", target.display(), error))?;
    }
    Ok(())
}

/// Static build from static-php-cli (Linux and macOS):
///   libexec/php   the real static binary
///   bin/php       wrapper script that points PHP at this version's php.ini
///   php.ini       configuration
///   conf.d/       additional ini files
fn lay_out_static_build(build: &Build, directory: &Path) -> Result<(), String> {
    let archive_path = directory.join("download.tar.gz");
    download::fetch(&build.url, &archive_path, build.sha256.as_deref())?;
    download::announce_checksum(build.sha256.is_some());

    let libexec_directory = directory.join("libexec");
    fs::create_dir_all(&libexec_directory).map_err(|error| error.to_string())?;
    extract_php_binary(&archive_path, &libexec_directory.join("php"))?;
    let _ = fs::remove_file(&archive_path);

    let bin_directory = directory.join("bin");
    fs::create_dir_all(&bin_directory).map_err(|error| error.to_string())?;
    write_wrapper_script(&bin_directory.join("php"))?;

    fs::write(directory.join("php.ini"), DEFAULT_INI)
        .map_err(|error| format!("writing php.ini: {}", error))?;
    fs::create_dir_all(directory.join("conf.d")).map_err(|error| error.to_string())?;
    Ok(())
}

/// Extract the `php` executable from the archive to `destination`.
fn extract_php_binary(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive = Archive::new(GzDecoder::new(file));

    let entries = archive
        .entries()
        .map_err(|error| format!("reading archive: {}", error))?;

    for entry in entries {
        let mut entry = entry.map_err(|error| format!("reading archive: {}", error))?;
        let is_php_binary = entry
            .path()
            .map(|path| path.file_name().map(|name| name == "php").unwrap_or(false))
            .unwrap_or(false);
        if is_php_binary {
            entry
                .unpack(destination)
                .map_err(|error| format!("extracting php binary: {}", error))?;
            set_executable(destination)?;
            return Ok(());
        }
    }
    Err("archive does not contain a 'php' binary".to_string())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// The static binary has its php.ini path compiled in (/usr/local/etc/php),
/// so a small wrapper script points PHP at this version's own configuration.
fn write_wrapper_script(path: &Path) -> Result<(), String> {
    let script = "\
#!/bin/sh
DIR=\"$(cd \"$(dirname \"$0\")/..\" && pwd -P)\"
export PHPRC=\"$DIR\"
export PHP_INI_SCAN_DIR=\"$DIR/conf.d\"
exec \"$DIR/libexec/php\" \"$@\"
";
    fs::write(path, script).map_err(|error| format!("writing wrapper: {}", error))?;
    set_executable(path)
}

#[cfg(test)]
mod tests;
