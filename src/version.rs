mod static_php;
mod windows_php;

/// (major, minor, patch)
type Version = (u32, u32, u32);

/// A downloadable PHP build.
#[derive(Debug, Clone)]
pub struct Build {
    version: Version,
    pub url: String,
    /// Expected SHA-256 as lowercase hex, when the source publishes one.
    pub sha256: Option<String>,
}

impl Build {
    /// "MAJOR.MINOR.PATCH"
    pub fn version_name(&self) -> String {
        let (major, minor, patch) = self.version;
        format!("{}.{}.{}", major, minor, patch)
    }
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

/// Builds for the current platform, one per version, sorted ascending.
/// Windows uses the official windows.php.net builds, other systems use static-php-cli.
fn available_builds() -> Result<Vec<Build>, String> {
    let mut builds = if cfg!(windows) {
        windows_php::builds()?
    } else {
        static_php::builds()?
    };
    builds.sort_by_key(|build| build.version);
    builds.dedup_by_key(|build| build.version);
    Ok(builds)
}

/// Versions available for download, as "MAJOR.MINOR.PATCH" strings, sorted ascending.
pub fn available_version_names() -> Result<Vec<String>, String> {
    Ok(available_builds()?
        .iter()
        .map(Build::version_name)
        .collect())
}

/// Resolve "8", "8.2" or "8.3.9" to the newest matching build.
pub fn resolve_build(input: &str) -> Result<Build, String> {
    select_build(&available_builds()?, input).cloned()
}

/// Pick the newest build in `available` that matches `input`.
fn select_build<'a>(available: &'a [Build], input: &str) -> Result<&'a Build, String> {
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

    available
        .iter()
        .filter(|build| {
            let (major, minor, patch) = build.version;
            [major, minor, patch].starts_with(&requested_parts)
        })
        .max_by_key(|build| build.version)
        .ok_or_else(|| format!("no PHP build found for version '{}'", input))
}

#[cfg(test)]
mod tests;
