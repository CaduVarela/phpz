/// Platform-specific download URL resolver for PHP versions

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct PhpRelease {
    version: String,
}

/// Resolve version input to full version string
/// - "8" -> latest 8.x.y
/// - "8.2" -> latest 8.2.y
/// - "8.3.9" -> 8.3.9 (if exists)
pub fn resolve_version(input: &str) -> Option<String> {
    let parts: Vec<&str> = input.split('.').collect();

    // Validate all parts are numeric
    if !parts.iter().all(|p| p.parse::<u32>().is_ok()) {
        return None;
    }

    match parts.len() {
        1 => {
            // Major version only: find latest minor.patch
            fetch_latest_version(parts[0], None)
        }
        2 => {
            // Major.minor: find latest patch
            fetch_latest_version(parts[0], Some(parts[1]))
        }
        3 => {
            // Full version: validate it exists
            if validate_version_exists(input) {
                Some(input.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Fetch latest version from php.net for given major[.minor]
fn fetch_latest_version(major: &str, minor: Option<&str>) -> Option<String> {
    if let Some(min) = minor {
        // Specific major.minor: fetch the latest patch version
        let url = format!("https://www.php.net/releases/index.php?json&version={}.{}", major, min);
        let response = reqwest::blocking::get(&url).ok()?;
        let release: PhpRelease = response.json().ok()?;
        return Some(release.version);
    }

    // Major version only: fetch latest from supported versions
    let url = "https://www.php.net/releases/index.php?json&max=100";
    let response = reqwest::blocking::get(url).ok()?;
    let releases: HashMap<String, PhpRelease> = response.json().ok()?;

    // Get the release for this major version
    let major_release = releases.get(major)?;
    Some(major_release.version.clone())
}

/// Validate if a specific full version exists
fn validate_version_exists(version: &str) -> bool {
    let url = format!("https://www.php.net/distributions/php-{}.tar.gz", version);

    // Try HEAD request to check if file exists
    if let Ok(response) = reqwest::blocking::Client::new().head(&url).send() {
        return response.status().is_success();
    }

    false
}

#[cfg(target_os = "windows")]
pub fn get_download_url(version: &str) -> Option<String> {
    println!("Resolving Windows download URL for {}", version);

    // TODO: fetch https://windows.php.net/downloads/releases/releases.json
    // TODO: parse JSON with serde
    // TODO: select build (default: x64 + non-thread-safe)

    None
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn get_download_url(version: &str) -> Option<String> {
    Some(format!(
        "https://www.php.net/distributions/php-{}.tar.gz",
        version
    ))
}
