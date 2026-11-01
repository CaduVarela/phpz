//! Queries about versions installed on disk and the active version.

use std::fs;

use crate::paths;

/// Names of installed versions (finished installations only), sorted ascending.
pub fn installed_versions() -> Result<Vec<String>, String> {
    let versions_directory = paths::versions_directory()?;
    if !versions_directory.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&versions_directory)
        .map_err(|error| format!("reading versions directory: {}", error))?;

    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_type()
                .map(|file_type| file_type.is_dir())
                .unwrap_or(false)
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.ends_with(".tmp"))
        .collect();
    names.sort_by_key(|name| version_sort_key(name));
    Ok(names)
}

pub fn version_sort_key(name: &str) -> Vec<u32> {
    name.split('.')
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Find the newest installed version matching "8", "8.3" or "8.3.32".
pub fn find_installed(requested: &str) -> Result<String, String> {
    select_installed(&installed_versions()?, requested)
}

/// Pick the newest of `installed` (sorted ascending) that matches `requested`.
fn select_installed(installed: &[String], requested: &str) -> Result<String, String> {
    let requested_parts: Vec<u32> = requested
        .split('.')
        .map(|part| part.parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("invalid version '{}'", requested))?;

    installed
        .iter()
        .rfind(|name| version_sort_key(name).starts_with(&requested_parts))
        .cloned()
        .ok_or_else(|| {
            format!(
                "PHP {} is not installed. Run 'phpz install {}' first.",
                requested, requested
            )
        })
}

/// Name of the active version, or None when nothing is active.
pub fn active_version() -> Result<Option<String>, String> {
    let link = paths::current_link()?;
    match fs::read_link(&link) {
        Ok(target) => Ok(target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("reading {}: {}", link.display(), error)),
    }
}

/// Path of the active version's php.ini.
pub fn active_ini_path() -> Result<std::path::PathBuf, String> {
    match active_version()? {
        Some(version) => Ok(paths::versions_directory()?.join(version).join("php.ini")),
        None => Err("no PHP version is active. Run 'phpz use <version>'.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn sort_key_orders_numerically() {
        let mut versions = names(&["8.3.10", "8.3.9", "8.10.0", "8.2.30"]);
        versions.sort_by_key(|name| version_sort_key(name));
        assert_eq!(versions, names(&["8.2.30", "8.3.9", "8.3.10", "8.10.0"]));
    }

    #[test]
    fn partial_request_selects_newest_matching_version() {
        let installed = names(&["8.2.30", "8.3.9", "8.3.10"]);
        assert_eq!(select_installed(&installed, "8").unwrap(), "8.3.10");
        assert_eq!(select_installed(&installed, "8.2").unwrap(), "8.2.30");
        assert_eq!(select_installed(&installed, "8.3.9").unwrap(), "8.3.9");
    }

    #[test]
    fn missing_or_invalid_request_is_an_error() {
        let installed = names(&["8.3.10"]);
        assert!(select_installed(&installed, "7").is_err());
        assert!(select_installed(&installed, "8.3.1").is_err());
        assert!(select_installed(&installed, "abc").is_err());
    }
}
