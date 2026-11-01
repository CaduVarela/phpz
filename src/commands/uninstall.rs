use std::fs;

use crate::php::installed;
use crate::platform::paths;

/// Remove an installed PHP version
pub fn run(version: &str) -> Result<(), String> {
    uninstall(version)
}

fn uninstall(version: &str) -> Result<(), String> {
    // Deleting needs an exact match, so "8.3" never removes a version by accident.
    if !installed::installed_versions()?
        .iter()
        .any(|name| name == version)
    {
        return Err(format!(
            "PHP {} is not installed. Use the exact version shown by 'phpz list'.",
            version
        ));
    }

    if installed::active_version()?.as_deref() == Some(version) {
        return Err(format!(
            "PHP {} is the active version. Run 'phpz use <other version>' or 'phpz reset' first.",
            version
        ));
    }

    let directory = paths::versions_directory()?.join(version);
    fs::remove_dir_all(&directory)
        .map_err(|error| format!("removing {}: {}", directory.display(), error))?;

    println!("Uninstalled PHP {}", version);
    Ok(())
}
