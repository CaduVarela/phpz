use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;

use crate::{paths, version};

const DEFAULT_INI: &str = "\
; php.ini managed by phpz
[PHP]
memory_limit = 512M
error_reporting = E_ALL
display_errors = On
date.timezone = UTC
";

pub fn run(version: String) {
    if let Err(error) = install(&version) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn install(requested_version: &str) -> Result<(), String> {
    println!("Resolving version '{}'...", requested_version);
    let resolved_version = version::resolve_version(requested_version)?;

    let versions_directory = paths::versions_directory()?;
    let final_directory = versions_directory.join(&resolved_version);
    let temporary_directory = versions_directory.join(format!("{}.tmp", resolved_version));

    fs::create_dir_all(&versions_directory)
        .map_err(|error| format!("creating versions directory: {}", error))?;

    if final_directory.exists() {
        println!("PHP {} is already installed.", resolved_version);
        return Ok(());
    }

    if temporary_directory.exists() {
        let _ = fs::remove_dir_all(&temporary_directory);
    }
    fs::create_dir_all(&temporary_directory)
        .map_err(|error| format!("creating temporary directory: {}", error))?;

    println!("Installing PHP {}", resolved_version);
    let result = populate(&resolved_version, &temporary_directory).and_then(|_| {
        fs::rename(&temporary_directory, &final_directory)
            .map_err(|error| format!("finalizing installation: {}", error))
    });

    if let Err(error) = result {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error);
    }

    println!("Successfully installed PHP {}", resolved_version);
    Ok(())
}

/// Download and lay out a version inside `directory`:
///   libexec/php   the real static binary
///   bin/php       wrapper script that points PHP at this version's php.ini
///   php.ini       configuration
///   conf.d/       additional ini files
fn populate(resolved_version: &str, directory: &Path) -> Result<(), String> {
    let download_url = version::get_download_url(resolved_version).ok_or_else(|| {
        "unsupported platform (Windows support is not implemented yet)".to_string()
    })?;

    let archive_path = directory.join("download.tar.gz");
    download(&download_url, &archive_path)?;

    let libexec_directory = directory.join("libexec");
    fs::create_dir_all(&libexec_directory).map_err(|error| error.to_string())?;
    extract_php_binary(&archive_path, &libexec_directory.join("php"))?;
    let _ = fs::remove_file(&archive_path);

    // TODO: verify integrity (static-php does not publish checksums; consider pinning hashes)

    let bin_directory = directory.join("bin");
    fs::create_dir_all(&bin_directory).map_err(|error| error.to_string())?;
    write_wrapper_script(&bin_directory.join("php"))?;

    fs::write(directory.join("php.ini"), DEFAULT_INI)
        .map_err(|error| format!("writing php.ini: {}", error))?;
    fs::create_dir_all(directory.join("conf.d")).map_err(|error| error.to_string())?;
    Ok(())
}

fn download(url: &str, destination: &Path) -> Result<(), String> {
    println!("Downloading {}", url);
    let mut response = reqwest::blocking::get(url)
        .and_then(|response| response.error_for_status())
        .map_err(|error| format!("download failed: {}", error))?;

    let total_bytes = response.content_length();
    let mut file = File::create(destination)
        .map_err(|error| format!("creating {}: {}", destination.display(), error))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut downloaded_bytes: u64 = 0;

    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|error| format!("download failed: {}", error))?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .map_err(|error| error.to_string())?;
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
    Ok(())
}

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
