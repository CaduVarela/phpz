use std::collections::BTreeMap;

use serde_json::Value;

use super::{parse_version, Build, Version};

const RELEASES_URL: &str = "https://windows.php.net/downloads/releases/releases.json";
const DOWNLOAD_BASE_URL: &str = "https://windows.php.net/downloads/releases";
const ARCHIVES_URL: &str = "https://windows.php.net/downloads/releases/archives/";

/// 7.0 is the oldest series that runs with the current Visual C++ redistributable (VC14).
/// 5.x needs VC11, which a modern machine usually lacks.
const OLDEST_SUPPORTED: Version = (7, 0, 0);

/// Official builds (non thread safe, x64). The newest release of each series comes from
/// `releases.json` with its SHA-256; with `include_history` the older patches found in the
/// archives are added, and a release listed in both places keeps the `releases.json` entry.
pub fn builds(include_history: bool) -> Result<Vec<Build>, String> {
    if std::env::consts::ARCH != "x86_64" {
        return Err(format!(
            "unsupported platform: {}-{} (official Windows builds exist for x64 only)",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }

    let newest = parse_releases(&fetch_text(RELEASES_URL, "release list")?)?;
    if !include_history {
        return Ok(newest);
    }

    let history = parse_archive_listing(&fetch_text(ARCHIVES_URL, "archive list")?);
    Ok(merge(history, newest))
}

fn fetch_text(url: &str, what: &str) -> Result<String, String> {
    reqwest::blocking::get(url)
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|error| format!("failed to fetch {}: {}", what, error))
}

/// One build per version; entries of `preferred` win over entries of `base`.
fn merge(base: Vec<Build>, preferred: Vec<Build>) -> Vec<Build> {
    let mut by_version: BTreeMap<Version, Build> = BTreeMap::new();
    for build in base.into_iter().chain(preferred) {
        by_version.insert(build.version, build);
    }
    by_version.into_values().collect()
}

fn parse_releases(json: &str) -> Result<Vec<Build>, String> {
    let releases: Value =
        serde_json::from_str(json).map_err(|error| format!("invalid release list: {}", error))?;
    let series = releases
        .as_object()
        .ok_or("invalid release list: expected an object")?;

    Ok(series.values().filter_map(parse_series).collect())
}

fn parse_series(series: &Value) -> Option<Build> {
    // Versions can carry a packaging revision, e.g. "8.2.35-1".
    let version_text = series.get("version")?.as_str()?;
    let version = parse_version(version_text.split('-').next()?)?;

    let (_, build) = series
        .as_object()?
        .iter()
        .find(|(key, _)| key.starts_with("nts-") && key.ends_with("-x64"))?;
    let zip = build.get("zip")?;

    Some(Build {
        version,
        url: format!("{}/{}", DOWNLOAD_BASE_URL, zip.get("path")?.as_str()?),
        sha256: zip
            .get("sha256")
            .and_then(Value::as_str)
            .map(str::to_lowercase),
        sbom_url: None,
        expects_checksum: true,
        variant: None,
        size: None,
    })
}

/// Version and packaging revision of a file name such as `php-8.5.12-1-nts-Win32-vs17-x64.zip`.
/// Anything that is not a non-thread-safe x64 PHP zip (packs, thread-safe, x86, SBOMs) is None.
fn parse_archive_name(name: &str) -> Option<(Version, u32)> {
    let middle = name.strip_prefix("php-")?.strip_suffix("-x64.zip")?;
    let (release, _toolchain) = middle.split_once("-nts-Win32-")?;
    let (version_text, revision) = match release.split_once('-') {
        Some((version_text, revision)) => (version_text, revision.parse().ok()?),
        None => (release, 0),
    };
    Some((parse_version(version_text)?, revision))
}

/// Every supported build in the archive page, keeping the newest rebuild of each version.
/// Archive builds carry no checksum, but newer ones have an SBOM that does.
fn parse_archive_listing(html: &str) -> Vec<Build> {
    let mut newest_rebuild: BTreeMap<Version, (u32, &str)> = BTreeMap::new();

    let file_names = html
        .split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next());
    for name in file_names {
        let Some((version, revision)) = parse_archive_name(name) else {
            continue;
        };
        if version < OLDEST_SUPPORTED {
            continue;
        }
        let is_newer = newest_rebuild
            .get(&version)
            .map(|(known, _)| revision > *known)
            .unwrap_or(true);
        if is_newer {
            newest_rebuild.insert(version, (revision, name));
        }
    }

    newest_rebuild
        .into_iter()
        .map(|(version, (_, name))| {
            let url = format!("{}{}", ARCHIVES_URL, name);
            Build {
                version,
                sbom_url: Some(format!("{}.cdx.json", url)),
                url,
                sha256: None,
                expects_checksum: true,
                variant: None,
                size: None,
            }
        })
        .collect()
}

/// The SHA-256 of the zip, from its CycloneDX SBOM. Ok(None) when no SBOM is published.
/// Other failures are errors on purpose: a flaky connection must not silently skip verification.
pub fn fetch_checksum(sbom_url: &str) -> Result<Option<String>, String> {
    let response = reqwest::blocking::get(sbom_url)
        .map_err(|error| format!("failed to fetch the checksum: {}", error))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let body = response
        .error_for_status()
        .and_then(|response| response.text())
        .map_err(|error| format!("failed to fetch the checksum: {}", error))?;
    Ok(parse_sbom_checksum(&body))
}

/// `metadata.component.hashes` of a CycloneDX SBOM describes the zip itself.
fn parse_sbom_checksum(json: &str) -> Option<String> {
    let sbom: Value = serde_json::from_str(json).ok()?;
    sbom.get("metadata")?
        .get("component")?
        .get("hashes")?
        .as_array()?
        .iter()
        .find(|hash| hash.get("alg").and_then(Value::as_str) == Some("SHA-256"))?
        .get("content")?
        .as_str()
        .map(str::to_lowercase)
}

#[cfg(test)]
mod tests;
