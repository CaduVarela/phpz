use std::collections::BTreeSet;

use serde::Deserialize;

use super::{parse_version, Build};
use crate::extensions::extension_name;
use crate::variant::Variant;

const BASE_URL: &str = "https://dl.static-php.dev/static-php-cli";

#[derive(Debug, Deserialize)]
struct IndexEntry {
    name: String,
    is_dir: bool,
    #[serde(default)]
    size: String,
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

fn fetch_json<T: serde::de::DeserializeOwned>(url: &str, what: &str) -> Result<T, String> {
    reqwest::blocking::get(url)
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|error| format!("failed to fetch {}: {}", what, error))
}

/// Every CLI build of one variant published for the current platform.
pub fn builds(variant: Variant) -> Result<Vec<Build>, String> {
    let platform = platform().ok_or_else(|| {
        format!(
            "unsupported platform: {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let url = format!("{}/{}/?format=json", BASE_URL, variant.name());
    let entries: Vec<IndexEntry> = fetch_json(&url, "version index")?;

    let builds = parse_builds(&entries, variant, &platform);
    if builds.is_empty() {
        return Err(format!(
            "the {} build is not published for {}",
            variant.name(),
            platform
        ));
    }
    Ok(builds)
}

fn parse_builds(entries: &[IndexEntry], variant: Variant, platform: &str) -> Vec<Build> {
    let file_suffix = format!("-cli-{}.tar.gz", platform);
    entries
        .iter()
        .filter(|entry| !entry.is_dir)
        .filter_map(|entry| {
            let version = entry.name.strip_prefix("php-")?.strip_suffix(&file_suffix)?;
            Some(Build {
                version: parse_version(version)?,
                url: format!("{}/{}/{}", BASE_URL, variant.name(), entry.name),
                sha256: None,
                sbom_url: None,
                expects_checksum: false,
                variant: Some(variant),
                size: Some(entry.size.clone()).filter(|size| !size.is_empty()),
            })
        })
        .collect()
}

/// The extensions one variant was built with.
pub fn extensions(variant: Variant) -> Result<BTreeSet<String>, String> {
    let url = format!("{}/{}/build-extensions.json", BASE_URL, variant.name());
    let names: Vec<String> = fetch_json(&url, &format!("the {} extension list", variant.name()))?;
    Ok(names.iter().map(|name| extension_name(name)).collect())
}

#[cfg(test)]
mod tests;
