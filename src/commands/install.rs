use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;

use crate::version::Build;
use crate::{paths, shell, version};

const DEFAULT_INI: &str = "\
; php.ini managed by phpz
[PHP]
memory_limit = 512M
error_reporting = E_ALL
display_errors = On
date.timezone = UTC
";

/// Install a specific PHP version
pub fn run(version: String) {
    if let Err(error) = install(&version) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn install(requested_version: &str) -> Result<(), String> {
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

    // Clean up leftovers from a previous failed attempt
    if temporary_directory.exists() {
        let _ = fs::remove_dir_all(&temporary_directory);
    }
    fs::create_dir_all(&temporary_directory)
        .map_err(|error| format!("creating temporary directory: {}", error))?;

    println!("Installing PHP {}", resolved_version);
    let result = populate(&build, &temporary_directory).and_then(|_| {
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

fn populate(build: &Build, directory: &Path) -> Result<(), String> {
    if cfg!(windows) {
        lay_out_windows_build(build, directory)
    } else {
        lay_out_static_build(build, directory)
    }
}

/// Extensions enabled by default in the Windows php.ini, when the build ships them.
const WINDOWS_DEFAULT_EXTENSIONS: &[&str] = &[
    "bz2", "curl", "exif", "fileinfo", "gd", "gmp", "intl", "mbstring", "mysqli", "openssl",
    "pdo_mysql", "pdo_pgsql", "pdo_sqlite", "pgsql", "sockets", "sodium", "sqlite3", "zip",
];

/// Official Windows zip, extracted as it comes:
///   php.exe, DLLs, ext\   from the zip
///   php.ini               php.ini-development with extension_dir and common extensions enabled
fn lay_out_windows_build(build: &Build, directory: &Path) -> Result<(), String> {
    let archive_path = directory.join("download.zip");
    download(&build.url, &archive_path, build.sha256.as_deref())?;
    extract_zip(&archive_path, directory)?;
    let _ = fs::remove_file(&archive_path);

    let template_path = directory.join("php.ini-development");
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("reading {}: {}", template_path.display(), error))?;

    let mut directives = vec!["extension_dir = \"ext\"".to_string()];
    directives.extend(
        WINDOWS_DEFAULT_EXTENSIONS
            .iter()
            .map(|name| format!("extension={}", name)),
    );
    fs::write(directory.join("php.ini"), uncomment_directives(&template, &directives))
        .map_err(|error| format!("writing php.ini: {}", error))
}

/// Turn `;directive` lines into active `directive` lines. Anything else is left alone.
fn uncomment_directives(content: &str, directives: &[String]) -> String {
    content
        .split_inclusive('\n')
        .map(|line| {
            let text = line.trim_end_matches(['\r', '\n']);
            let is_wanted = text
                .strip_prefix(';')
                .map(|directive| directives.iter().any(|wanted| wanted == directive))
                .unwrap_or(false);
            if is_wanted {
                line[1..].to_string()
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
    download(&build.url, &archive_path, build.sha256.as_deref())?;

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

/// Download `url` to `destination`, checking the SHA-256 when one is known.
fn download(url: &str, destination: &Path, expected_sha256: Option<&str>) -> Result<(), String> {
    println!("Downloading {}", url);
    let mut response = reqwest::blocking::get(url)
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("download failed: {}", error))?;

    let total_bytes = response.content_length();
    let mut file = File::create(destination)
        .map_err(|error| format!("creating {}: {}", destination.display(), error))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded_bytes: u64 = 0;
    let mut hasher = Sha256::new();

    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|error| format!("download failed: {}", error))?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .map_err(|error| error.to_string())?;
        hasher.update(&buffer[..bytes_read]);
        downloaded_bytes += bytes_read as u64;

        let megabytes = downloaded_bytes as f64 / 1e6;
        match total_bytes {
            Some(total) if total > 0 => {
                eprint!("\r  {:>3}% ({:.1} MB)", downloaded_bytes * 100 / total, megabytes)
            }
            _ => eprint!("\r  {:.1} MB", megabytes),
        }
    }
    eprintln!();

    if let Some(total) = total_bytes {
        if downloaded_bytes != total {
            return Err(format!(
                "incomplete download ({} of {} bytes)",
                downloaded_bytes, total
            ));
        }
    }

    match expected_sha256 {
        Some(expected) => {
            check_sha256(&format!("{:x}", hasher.finalize()), expected)?;
            println!("Verified SHA-256 checksum");
        }
        None => println!("Note: no checksum is published for this build, so it was not verified."),
    }
    Ok(())
}

fn check_sha256(actual: &str, expected: &str) -> Result<(), String> {
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "checksum mismatch (expected {}, got {}). The download was discarded.",
            expected, actual
        ))
    }
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
