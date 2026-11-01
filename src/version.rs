use serde::Deserialize;

const INDEX_URL: &str = "https://dl.static-php.dev/static-php-cli/common/?format=json";
const BASE_URL: &str = "https://dl.static-php.dev/static-php-cli/common";

#[derive(Debug, Deserialize)]
struct IndexEntry {
    name: String,
    is_dir: bool,
}

/// (major, minor, patch)
type Version = (u32, u32, u32);

/// Platform suffix used in static-php file names, e.g. "linux-x86_64".
/// Returns None for unsupported platforms (currently Windows).
pub fn platform() -> Option<String> {
    let operating_system = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        _ => return None,
    };
    let architecture = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    Some(format!("{}-{}", operating_system, architecture))
}

fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.split('.');
    let version = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    if parts.next().is_some() {
        return None;
    }
    Some(version)
}

/// All versions that have a CLI build for the current platform, sorted ascending.
fn available_versions() -> Result<Vec<Version>, String> {
    let platform = platform().ok_or_else(|| {
        format!(
            "unsupported platform: {}-{} (Windows support is not implemented yet)",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let file_suffix = format!("-cli-{}.tar.gz", platform);

    let entries: Vec<IndexEntry> = reqwest::blocking::get(INDEX_URL)
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|error| format!("failed to fetch version index: {}", error))?;

    let mut versions: Vec<Version> = entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .filter_map(|entry| entry.name.strip_prefix("php-")?.strip_suffix(&file_suffix))
        .filter_map(parse_version)
        .collect();
    versions.sort();
    versions.dedup();
    Ok(versions)
}

/// Versions available for download, as "MAJOR.MINOR.PATCH" strings, sorted ascending.
pub fn available_version_names() -> Result<Vec<String>, String> {
    Ok(available_versions()?
        .into_iter()
        .map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
        .collect())
}

/// Resolve user input to a full version string.
/// - "8"     -> latest 8.x.y
/// - "8.2"   -> latest 8.2.y
/// - "8.3.9" -> 8.3.9 (if a build exists)
pub fn resolve_version(input: &str) -> Result<String, String> {
    select_version(&available_versions()?, input)
}

/// Pick the newest version in `available` that matches `input`.
fn select_version(available: &[Version], input: &str) -> Result<String, String> {
    let invalid_format_message = format!(
        "invalid version '{}'. Expected MAJOR, MAJOR.MINOR or MAJOR.MINOR.PATCH",
        input
    );

    let requested_parts: Vec<u32> = input
        .split('.')
        .map(|part| part.parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| invalid_format_message.clone())?;
    if requested_parts.len() > 3 {
        return Err(invalid_format_message);
    }

    let latest_match = available
        .iter()
        .copied()
        .filter(|&(major, minor, patch)| [major, minor, patch].starts_with(&requested_parts))
        .max();

    match latest_match {
        Some((major, minor, patch)) => Ok(format!("{}.{}.{}", major, minor, patch)),
        None => Err(format!("no PHP build found for version '{}'", input)),
    }
}

/// Download URL of the CLI build for `version` (always a full MAJOR.MINOR.PATCH).
pub fn get_download_url(version: &str) -> Option<String> {
    Some(format!(
        "{}/php-{}-cli-{}.tar.gz",
        BASE_URL,
        version,
        platform()?
    ))
}

#[cfg(test)]
mod tests;
