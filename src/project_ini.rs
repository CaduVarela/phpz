use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::extensions;
use crate::ini::{IniDocument, PHP_SECTION};
use crate::paths;
use crate::settings::ExtensionSettings;

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
    if fs::read_to_string(&target)
        .map(|existing| existing == content)
        .unwrap_or(false)
    {
        return Ok(directory);
    }

    fs::create_dir_all(&directory)
        .map_err(|error| format!("creating {}: {}", directory.display(), error))?;
    let temporary = directory.join("php.ini.tmp");
    fs::write(&temporary, content)
        .and_then(|_| fs::rename(&temporary, &target))
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

#[cfg(test)]
mod tests;
