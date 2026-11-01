use std::fs;

use crate::platform::paths;

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
        .filter(|name| !name.ends_with(".tmp") && !name.ends_with(".old"))
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
pub fn select_installed(installed: &[String], requested: &str) -> Result<String, String> {
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
