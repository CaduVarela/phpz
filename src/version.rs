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
    /// Where the SHA-256 can be fetched when the listing does not carry it (Windows archives).
    sbom_url: Option<String>,
    /// The source normally publishes a checksum, so a build without one is out of the ordinary.
    pub expects_checksum: bool,
}

impl Build {
    /// Whether this build is at least `major.minor`.
    pub fn at_least(&self, major: u32, minor: u32) -> bool {
        (self.version.0, self.version.1) >= (major, minor)
    }

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
///
/// Windows uses the official windows.php.net builds. Its cheap list holds the newest release of
/// each series; `include_history` also reads the large archive page. Other systems use
/// static-php-cli, whose single index already holds every version.
fn available_builds(include_history: bool) -> Result<Vec<Build>, String> {
    let mut builds = if cfg!(windows) {
        windows_php::builds(include_history)?
    } else {
        static_php::builds()?
    };
    builds.sort_by_key(|build| build.version);
    builds.dedup_by_key(|build| build.version);
    Ok(builds)
}

/// Versions available for download, as "MAJOR.MINOR.PATCH" strings, sorted ascending.
/// With `include_history`, Windows also lists the older patches.
pub fn available_version_names(include_history: bool) -> Result<Vec<String>, String> {
    Ok(available_builds(include_history)?
        .iter()
        .map(Build::version_name)
        .collect())
}

/// Resolve "8", "8.2" or "8.3.9" to the newest matching build, with its checksum if one is published.
pub fn resolve_build(input: &str) -> Result<Build, String> {
    let requested = parse_request(input)?;
    let no_build = || format!("no PHP build found for version '{}'", input);

    // A series or minor always resolves to its newest release, which the cheap list holds.
    // Only an exact older patch needs the archive page.
    let mut build = match find_build(&available_builds(false)?, &requested) {
        Some(build) => build.clone(),
        None if cfg!(windows) => find_build(&available_builds(true)?, &requested)
            .ok_or_else(no_build)?
            .clone(),
        None => return Err(no_build()),
    };

    if build.sha256.is_none() {
        if let Some(sbom_url) = &build.sbom_url {
            build.sha256 = windows_php::fetch_checksum(sbom_url)?;
        }
    }
    Ok(build)
}

/// "8", "8.2" or "8.3.9" as numbers.
fn parse_request(input: &str) -> Result<Vec<u32>, String> {
    let invalid_format_message = format!(
        "invalid version '{}'. Expected MAJOR, MAJOR.MINOR or MAJOR.MINOR.PATCH",
        input
    );

    let parts: Vec<u32> = input
        .split('.')
        .map(|part| part.parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| invalid_format_message.clone())?;
    if parts.len() > 3 {
        return Err(invalid_format_message);
    }
    Ok(parts)
}

/// The newest build in `available` that starts with the requested numbers.
fn find_build<'a>(available: &'a [Build], requested: &[u32]) -> Option<&'a Build> {
    available
        .iter()
        .filter(|build| {
            let (major, minor, patch) = build.version;
            [major, minor, patch].starts_with(requested)
        })
        .max_by_key(|build| build.version)
}

#[cfg(test)]
mod tests;
