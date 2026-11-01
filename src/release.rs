use std::path::Path;

use crate::download;

const REPOSITORY: &str = "CaduVarela/phpz";

/// (major, minor, patch)
pub type Version = (u32, u32, u32);

/// "v0.4.0" or "0.4.0" as numbers; None for anything else, such as a pre-release.
pub fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.trim().trim_start_matches('v').split('.');
    let version = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(version)
}

/// The name of the release file for a system, as the release workflow publishes it.
pub fn asset_name(os: &str, architecture: &str) -> Option<&'static str> {
    match (os, architecture) {
        ("linux", "x86_64") => Some("phpz-linux-x64"),
        ("linux", "aarch64") => Some("phpz-linux-arm64"),
        ("macos", "x86_64") => Some("phpz-macos-x64"),
        ("macos", "aarch64") => Some("phpz-macos-arm64"),
        ("windows", "x86_64") => Some("phpz-windows-x64.exe"),
        _ => None,
    }
}

/// The checksum listed for `asset` in a `SHA256SUMS` file.
pub fn checksum_for(sums: &str, asset: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.split_once(char::is_whitespace)?;
        (name.trim().trim_start_matches('*') == asset).then(|| hash.to_lowercase())
    })
}

/// The tag of the newest release, from the `tag_name` of the GitHub API answer.
pub fn latest_tag() -> Result<String, String> {
    let url = std::env::var("PHPZ_LATEST_URL").unwrap_or_else(|_| {
        format!(
            "https://api.github.com/repos/{}/releases/latest",
            REPOSITORY
        )
    });
    let client = reqwest::blocking::Client::builder()
        .user_agent(concat!("phpz/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?;
    let answer: serde_json::Value = client
        .get(&url)
        .send()
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.json())
        .map_err(|error| format!("could not look up the latest release: {}", error))?;
    answer
        .get("tag_name")
        .and_then(|tag| tag.as_str())
        .map(str::to_string)
        .ok_or_else(|| "the latest release has no tag".to_string())
}

fn base_url(tag: &str) -> String {
    std::env::var("PHPZ_RELEASE_URL").unwrap_or_else(|_| {
        format!(
            "https://github.com/{}/releases/download/{}",
            REPOSITORY, tag
        )
    })
}

/// Download the release file for this system into `destination`, checked against `SHA256SUMS`.
pub fn download_binary(tag: &str, destination: &Path) -> Result<(), String> {
    let asset = asset_name(std::env::consts::OS, std::env::consts::ARCH).ok_or_else(|| {
        format!(
            "no release is published for {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let base = base_url(tag);

    let sums = reqwest::blocking::get(format!("{}/SHA256SUMS", base))
        .and_then(|response| response.error_for_status())
        .and_then(|response| response.text())
        .map_err(|error| format!("could not download the checksum list: {}", error))?;
    let expected =
        checksum_for(&sums, asset).ok_or_else(|| format!("no checksum is listed for {}", asset))?;

    download::fetch(&format!("{}/{}", base, asset), destination, Some(&expected))?;
    println!("Verified SHA-256 checksum");
    Ok(())
}
