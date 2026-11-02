use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::download;
use crate::php::installed;
use crate::platform::paths;

/// Composer release lines, newest first, with the oldest PHP each runs on (7.2.5 is 70205).
const CHANNELS: &[(&str, u32)] = &[("2", 70205), ("2.2", 50302)];

const DEFAULT_BASE_URL: &str = "https://getcomposer.org";

/// A PHP version as the number Composer uses for its minimum: "7.2.5" is 70205.
pub fn php_id(version: &str) -> Option<u32> {
    let mut parts = version.split('.');
    let major: u32 = parts.next()?.parse().ok()?;
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch: u32 = parts.next().map_or(Some(0), |part| part.parse().ok())?;
    Some(major * 10000 + minor * 100 + patch)
}

/// The newest Composer line that runs on this PHP.
pub fn channel_for(php_id: u32) -> Option<&'static str> {
    CHANNELS
        .iter()
        .find(|(_, minimum)| php_id >= *minimum)
        .map(|(name, _)| *name)
}

pub fn channel_for_version(version: &str) -> Option<&'static str> {
    php_id(version).and_then(channel_for)
}

fn composer_directory() -> Result<PathBuf, String> {
    Ok(paths::phpz_home()?.join("composer"))
}

fn phar_path(channel: &str) -> Result<PathBuf, String> {
    Ok(composer_directory()?.join(format!("composer-{}.phar", channel)))
}

/// The Composer that runs on this PHP version, or what to do when it is not there.
pub fn phar_for(version: &str) -> Result<PathBuf, String> {
    let channel = channel_for_version(version)
        .ok_or_else(|| format!("no Composer release runs on PHP {}", version))?;
    let path = phar_path(channel)?;
    if path.is_file() {
        Ok(path)
    } else {
        Err(format!(
            "Composer {} (the line for PHP {}) is not installed. Run 'phpz composer install'.",
            channel, version
        ))
    }
}

/// Whether any Composer has been installed, which is what puts the `composer` shim in place.
pub fn any_installed() -> bool {
    CHANNELS
        .iter()
        .any(|(name, _)| phar_path(name).map(|path| path.is_file()).unwrap_or(false))
}

/// The download path of a line, from the answer of getcomposer.org/versions.
pub fn download_path(versions: &str, channel: &str) -> Option<String> {
    let answer: serde_json::Value = serde_json::from_str(versions).ok()?;
    answer
        .get(channel)?
        .get(0)?
        .get("path")?
        .as_str()
        .map(str::to_string)
}

/// The hash in a `.sha256sum` file ("hash  composer.phar").
pub fn checksum_from(text: &str) -> Option<String> {
    let hash = text.split_whitespace().next()?;
    let is_hash = hash.len() == 64 && hash.chars().all(|character| character.is_ascii_hexdigit());
    is_hash.then(|| hash.to_lowercase())
}

fn base_url() -> String {
    std::env::var("PHPZ_COMPOSER_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string())
}

fn get_text(url: &str) -> Result<String, String> {
    reqwest::blocking::get(url)
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|error| format!("could not read {}: {}", url, error))
}

/// Download the newest Composer of a line, checked against the SHA-256 getcomposer.org publishes.
pub fn install(channel: &str) -> Result<(), String> {
    let base = base_url();
    let versions = get_text(&format!("{}/versions", base))?;
    let path = download_path(&versions, channel)
        .ok_or_else(|| format!("getcomposer.org lists no Composer {}", channel))?;
    if !path.starts_with('/') || path.starts_with("//") {
        return Err(format!("getcomposer.org gave an unexpected path: {}", path));
    }
    let url = format!("{}{}", base, path);
    let expected = checksum_from(&get_text(&format!("{}.sha256sum", url))?)
        .ok_or_else(|| "getcomposer.org published no usable checksum".to_string())?;

    let target = phar_path(channel)?;
    let directory = target.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(directory)
        .map_err(|error| format!("creating {}: {}", directory.display(), error))?;
    let partial = target.with_extension("phar.part");
    let result = download::fetch(&url, &partial, Some(&expected))
        .and_then(|_| fs::rename(&partial, &target).map_err(|error| error.to_string()));
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result?;
    download::announce_checksum(true);
    Ok(())
}

/// Install the Composer lines the installed PHP versions need; with `refresh` also newer ones of lines already there.
pub fn install_for_installed_versions(refresh: bool) -> Result<(), String> {
    let mut channels: Vec<&str> = installed::installed_versions()?
        .iter()
        .filter_map(|version| channel_for_version(version))
        .collect();
    if channels.is_empty() {
        channels.push(CHANNELS[0].0);
    }
    channels.sort();
    channels.dedup();

    for channel in channels {
        if !refresh && phar_path(channel)?.is_file() {
            continue;
        }
        println!("Installing Composer {}", channel);
        install(channel)?;
    }
    Ok(())
}

/// After installing a PHP, add the Composer line it needs when the user already uses Composer here.
pub fn ensure_for(version: &str) {
    let Some(channel) = channel_for_version(version) else {
        return;
    };
    let missing = phar_path(channel)
        .map(|path| !path.is_file())
        .unwrap_or(false);
    if !any_installed() || !missing {
        return;
    }
    println!("Installing Composer {} for PHP {}", channel, version);
    if let Err(error) = install(channel) {
        eprintln!(
            "Note: could not install Composer {} ({}). Run 'phpz composer install' to try again.",
            channel, error
        );
    }
}

/// The first program with one of `names` in a `PATH`-style list, ignoring the `skip` folder.
pub fn find_in_path(path: &OsStr, skip: &Path, names: &[&str]) -> Option<PathBuf> {
    std::env::split_paths(path)
        .filter(|directory| directory != skip)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|candidate| is_program(candidate))
}

#[cfg(unix)]
fn is_program(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_program(path: &Path) -> bool {
    path.is_file()
}

/// A Composer the user already has, outside phpz's own shims folder.
fn existing_composer() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["composer.exe", "composer.bat", "composer.cmd"]
    } else {
        &["composer"]
    };
    find_in_path(
        &std::env::var_os("PATH")?,
        &paths::shims_directory().ok()?,
        names,
    )
}

/// Best effort: a failed Composer download never fails what the user asked for.
/// A Composer the user already has is left alone; `phpz composer install` replaces it on request.
pub fn install_if_missing() {
    if !any_installed() {
        if let Some(existing) = existing_composer() {
            println!(
                "Composer found at {}, so phpz leaves it alone. It runs on the PHP of each project through the php shim. 'phpz composer install' gives you one managed by phpz.",
                existing.display()
            );
            return;
        }
    }
    if let Err(error) = install_for_installed_versions(false) {
        eprintln!(
            "Note: could not install Composer ({}). Run 'phpz composer install' to try again.",
            error
        );
    }
    let _ = crate::project::shim::ensure();
}
