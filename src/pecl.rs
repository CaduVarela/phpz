use std::fs;
use std::io;
use std::path::Path;

const BASE_URL: &str = "https://downloads.php.net/~windows/pecl/releases";

/// How many releases to look through, newest first, for one that has a build for the PHP in use.
const MAX_RELEASES_TRIED: usize = 12;

/// A PHP series such as (8, 3).
pub type Series = (u32, u32);

#[derive(Debug, PartialEq, Clone)]
pub struct Package {
    pub version: String,
    pub url: String,
}

/// Extension names are lowercase letters, digits and underscores; nothing else may reach a URL.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// "8.3.36" is the series (8, 3).
pub fn series_of(php_version: &str) -> Option<Series> {
    let mut parts = php_version.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

fn links(html: &str) -> impl Iterator<Item = &str> {
    html.split("href=\"").skip(1).filter_map(|rest| rest.split('"').next())
}

/// Stable releases (digits and dots only, so no alpha, beta or RC) listed on an extension's page,
/// newest first. Versions are compared as numbers: 3.10.0 is newer than 3.9.0.
pub fn stable_versions(html: &str) -> Vec<String> {
    let mut versions: Vec<(Vec<u32>, String)> = links(html)
        .filter_map(|link| {
            let version = link.strip_suffix('/')?;
            if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return None;
            }
            let numbers: Option<Vec<u32>> = version.split('.').map(|part| part.parse().ok()).collect();
            Some((numbers?, version.to_string()))
        })
        .collect();
    versions.sort_by(|left, right| right.0.cmp(&left.0));
    versions.dedup_by(|left, right| left.0 == right.0);
    versions.into_iter().map(|(_, version)| version).collect()
}

/// The non-thread-safe x64 zip for `series` among the files of one release.
pub fn find_zip(html: &str, extension: &str, version: &str, series: Series) -> Option<String> {
    let prefix = format!("php_{}-{}-{}.{}-nts-", extension, version, series.0, series.1);
    links(html)
        .find(|name| name.starts_with(&prefix) && name.ends_with("-x64.zip"))
        .map(str::to_string)
}

/// The page at `url`, or None when the server has no such page.
fn fetch_listing(url: &str) -> Result<Option<String>, String> {
    let response = reqwest::blocking::get(url).map_err(|error| format!("failed to reach the PECL server: {}", error))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response
        .error_for_status()
        .and_then(|response| response.text())
        .map(Some)
        .map_err(|error| format!("failed to read the PECL server: {}", error))
}

/// The newest stable release of `extension` that has a build for `series`, or the release asked for.
pub fn find(extension: &str, series: Series, requested: Option<&str>) -> Result<Package, String> {
    let versions = match requested {
        Some(version) => {
            if !version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') || version.is_empty() {
                return Err(format!("invalid version '{}'", version));
            }
            vec![version.to_string()]
        }
        None => {
            let page = fetch_listing(&format!("{}/{}/", BASE_URL, extension))?.ok_or_else(|| {
                format!("'{}' is not an extension available for Windows on the PECL server", extension)
            })?;
            stable_versions(&page)
        }
    };

    for version in versions.iter().take(MAX_RELEASES_TRIED) {
        let Some(page) = fetch_listing(&format!("{}/{}/{}/", BASE_URL, extension, version))? else {
            continue;
        };
        if let Some(file) = find_zip(&page, extension, version, series) {
            return Ok(Package {
                version: version.clone(),
                url: format!("{}/{}/{}/{}", BASE_URL, extension, version, file),
            });
        }
    }
    Err(format!(
        "no build of {} for PHP {}.{} (non thread safe, x64) was found{}",
        extension,
        series.0,
        series.1,
        requested.map(|v| format!(" in release {}", v)).unwrap_or_default()
    ))
}

/// Files that come with an extension but do not matter to PHP.
fn is_harmless(file: &str) -> bool {
    let lower = file.to_lowercase();
    [".pdb", ".ini", ".txt", ".md"].iter().any(|suffix| lower.ends_with(suffix))
        || ["license", "readme", "credits", "notice", "changelog", "changes", "copying"]
            .iter()
            .any(|prefix| lower.starts_with(prefix))
}

/// Put `php_<name>.dll` from the zip into `ext_directory`. A package that carries anything else
/// that matters (more libraries, programs, folders) is refused, because dropping unknown files
/// next to PHP could replace one of its own.
pub fn install_dll(archive_path: &Path, name: &str, ext_directory: &Path) -> Result<(), String> {
    let wanted = format!("php_{}.dll", name);
    let file = fs::File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| format!("reading archive: {}", error))?;

    let mut found = None;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|error| format!("reading archive: {}", error))?;
        let entry_name = entry.name().to_string();
        let unsupported = |what: &str| {
            Err(format!(
                "{} comes with {} ({}), which phpz cannot install yet.",
                name, what, entry_name
            ))
        };

        if entry_name.contains('/') || entry_name.contains('\\') {
            return unsupported("folders");
        }
        if entry_name.eq_ignore_ascii_case(&wanted) {
            found = Some(index);
        } else if entry_name.to_lowercase().ends_with(".dll") || entry_name.to_lowercase().ends_with(".exe") {
            return unsupported("extra libraries");
        } else if !is_harmless(&entry_name) {
            return unsupported("files it does not know");
        }
    }

    let index = found.ok_or_else(|| format!("the archive does not contain {}", wanted))?;
    let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;

    fs::create_dir_all(ext_directory).map_err(|error| error.to_string())?;
    let temporary_path = ext_directory.join(format!(".{}.tmp", wanted));
    let mut output = fs::File::create(&temporary_path)
        .map_err(|error| format!("creating {}: {}", temporary_path.display(), error))?;
    io::copy(&mut entry, &mut output).map_err(|error| format!("extracting {}: {}", wanted, error))?;
    drop(output);
    fs::rename(&temporary_path, ext_directory.join(&wanted))
        .map_err(|error| format!("installing {}: {}", wanted, error))
}

#[cfg(test)]
mod tests;
