use serde::Deserialize;

use super::{parse_version, Build};

const INDEX_URL: &str = "https://dl.static-php.dev/static-php-cli/common/?format=json";
const BASE_URL: &str = "https://dl.static-php.dev/static-php-cli/common";

#[derive(Debug, Deserialize)]
struct IndexEntry {
    name: String,
    is_dir: bool,
}

/// Platform suffix of static-php file names, e.g. "linux-x86_64".
fn platform() -> Option<String> {
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

/// Every CLI build published for the current platform.
pub fn builds() -> Result<Vec<Build>, String> {
    let platform = platform().ok_or_else(|| {
        format!(
            "unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let file_suffix = format!("-cli-{}.tar.gz", platform);

    let entries: Vec<IndexEntry> = reqwest::blocking::get(INDEX_URL)
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|error| format!("failed to fetch version index: {}", error))?;

    Ok(entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .filter_map(|entry| {
            let version = entry.name.strip_prefix("php-")?.strip_suffix(&file_suffix)?;
            Some(Build {
                version: parse_version(version)?,
                url: format!("{}/{}", BASE_URL, entry.name),
                sha256: None,
                sbom_url: None,
            })
        })
        .collect())
}
