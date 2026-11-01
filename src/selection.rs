use std::path::{Path, PathBuf};

use crate::project::{self, Project};
use crate::{installed, paths};

/// Where the chosen version came from.
#[derive(Debug, PartialEq, Clone)]
pub enum Origin {
    /// A project file, which is the file named here.
    Project(PathBuf),
    /// The default set with `phpz use`.
    Global,
}

pub struct Selection {
    /// The full installed version, such as "8.3.32".
    pub version: String,
    pub origin: Origin,
    /// The project whose files applied, even when it pins no version (it may only set extensions).
    pub project: Option<Project>,
}

impl Selection {
    pub fn directory(&self) -> Result<PathBuf, String> {
        Ok(paths::versions_directory()?.join(&self.version))
    }
}

pub const NOTHING_SELECTED: &str =
    "no PHP version is active. Run 'phpz use <version>', or pin one here with 'phpz local <version>'.";

/// The project's pin if there is one, otherwise the global default. A pin that is not installed is
/// an error and does not fall back, so the wrong PHP never runs silently.
fn choose(
    project: Option<&Project>,
    installed_versions: &[String],
    global: Option<String>,
) -> Result<Option<(String, Origin)>, String> {
    if let Some(pinned) = project.and_then(|project| project.php.as_ref()) {
        let version = installed::select_installed(installed_versions, &pinned.request).map_err(|_| {
            format!(
                "PHP {} is required by {} but is not installed. Run 'phpz install {}'.",
                pinned.request,
                pinned.file.display(),
                pinned.request
            )
        })?;
        return Ok(Some((version, Origin::Project(pinned.file.clone()))));
    }
    Ok(global.map(|version| (version, Origin::Global)))
}

/// The version that applies in `start`, or None when nothing is selected at all.
pub fn select(start: &Path) -> Result<Option<Selection>, String> {
    let project = project::find(start)?;
    let versions = installed::installed_versions()?;
    let global = installed::active_version()?;

    Ok(choose(project.as_ref(), &versions, global)?
        .map(|(version, origin)| Selection { version, origin, project }))
}

pub fn select_here() -> Result<Option<Selection>, String> {
    let here = std::env::current_dir().map_err(|error| format!("could not read the current directory: {}", error))?;
    select(&here)
}

/// Like `select_here`, but nothing selected is an error.
pub fn require_here() -> Result<Selection, String> {
    select_here()?.ok_or_else(|| NOTHING_SELECTED.to_string())
}

#[cfg(test)]
mod tests;
