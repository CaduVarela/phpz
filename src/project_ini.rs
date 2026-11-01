use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::extensions;
use crate::ini::{IniDocument, PHP_SECTION};
use crate::paths;
use crate::settings::ExtensionSettings;

/// Records the project folder and version a generated directory belongs to.
const ORIGIN_FILE: &str = "origin";

/// The directory PHP should read php.ini from: the version's own or a generated one.
pub fn ini_directory(
    version_directory: &Path,
    version: &str,
    project_directory: &Path,
    wanted: &ExtensionSettings,
) -> Result<PathBuf, String> {
    let projects_directory = paths::phpz_home()?.join("projects");
    prepare(
        version_directory,
        version,
        project_directory,
        wanted,
        &projects_directory,
    )
}

fn prepare(
    version_directory: &Path,
    version: &str,
    project_directory: &Path,
    wanted: &ExtensionSettings,
    projects_directory: &Path,
) -> Result<PathBuf, String> {
    let unchanged = || Ok(version_directory.to_path_buf());
    if wanted.is_empty() {
        return unchanged();
    }
    let Ok(mut ini) = IniDocument::load(&version_directory.join("php.ini")) else {
        return unchanged();
    };

    // Built-in extensions are not looked up, since `php -m` is too slow here.
    let available = extensions::available_shared(version_directory);
    let report = extensions::apply_settings(&mut ini, wanted, &available, || Ok(BTreeSet::new()))?;
    if !report.changed() {
        return unchanged();
    }
    make_extension_dir_absolute(&mut ini, version_directory);

    let directory = projects_directory.join(key(project_directory, version));
    let target = directory.join("php.ini");
    let content = ini.render();
    let origin = format!("{}\n{}\n", project_directory.display(), version);
    let origin_target = directory.join(ORIGIN_FILE);
    let same = |path: &Path, expected: &str| {
        fs::read_to_string(path)
            .map(|existing| existing == expected)
            .unwrap_or(false)
    };
    if same(&target, &content) && same(&origin_target, &origin) {
        return Ok(directory);
    }

    fs::create_dir_all(&directory)
        .map_err(|error| format!("creating {}: {}", directory.display(), error))?;
    let temporary = directory.join("php.ini.tmp");
    fs::write(&temporary, content)
        .and_then(|_| fs::rename(&temporary, &target))
        .and_then(|_| fs::write(&origin_target, origin))
        .map_err(|error| format!("writing {}: {}", target.display(), error))?;
    Ok(directory)
}

/// Pin `extension_dir` to the version's directory, as a relative one would break.
fn make_extension_dir_absolute(ini: &mut IniDocument, version_directory: &Path) {
    let Some(value) = ini.get(PHP_SECTION, "extension_dir") else {
        return;
    };
    let path = Path::new(&value);
    if path.is_relative() {
        let absolute = version_directory.join(path);
        ini.set(
            PHP_SECTION,
            "extension_dir",
            &format!("\"{}\"", absolute.display()),
        );
    }
}

/// One generated directory per project folder and version.
fn key(project_directory: &Path, version: &str) -> String {
    let digest = Sha256::digest(format!("{}\0{}", project_directory.display(), version));
    format!("{:x}", digest)[..16].to_string()
}

/// Remove generated directories whose project folder is gone or whose version is no longer
/// installed (and those with no record of either). Returns what was, or with `dry_run` would be, removed.
pub fn prune(dry_run: bool) -> Result<Vec<String>, String> {
    let home = paths::phpz_home()?;
    prune_in(&home.join("projects"), &home.join("versions"), dry_run)
}

fn prune_in(
    projects_directory: &Path,
    versions_directory: &Path,
    dry_run: bool,
) -> Result<Vec<String>, String> {
    let Ok(entries) = fs::read_dir(projects_directory) else {
        return Ok(Vec::new());
    };

    let mut removed = Vec::new();
    for entry in entries.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let origin = fs::read_to_string(directory.join(ORIGIN_FILE)).unwrap_or_default();
        let mut lines = origin.lines();
        let stale = match (lines.next(), lines.next()) {
            (Some(project), Some(version)) => {
                !Path::new(project).is_dir() || !versions_directory.join(version).is_dir()
            }
            _ => true,
        };
        if !stale {
            continue;
        }
        if !dry_run {
            fs::remove_dir_all(&directory)
                .map_err(|error| format!("removing {}: {}", directory.display(), error))?;
        }
        removed.push(match origin.lines().next() {
            Some(project) => project.to_string(),
            None => directory.display().to_string(),
        });
    }
    removed.sort();
    Ok(removed)
}

#[cfg(test)]
mod tests;
