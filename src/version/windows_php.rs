use serde_json::Value;

use super::{parse_version, Build};

const RELEASES_URL: &str = "https://windows.php.net/downloads/releases/releases.json";
const DOWNLOAD_BASE_URL: &str = "https://windows.php.net/downloads/releases";

/// Official builds (non thread safe, x64) of the newest release of each PHP series.
pub fn builds() -> Result<Vec<Build>, String> {
    if std::env::consts::ARCH != "x86_64" {
        return Err(format!(
            "unsupported platform: {}-{} (official Windows builds exist for x64 only)",
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }

    let body = reqwest::blocking::get(RELEASES_URL)
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|error| format!("failed to fetch release list: {}", error))?;
    parse_releases(&body)
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
    })
}

#[cfg(test)]
mod tests;
