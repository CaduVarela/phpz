use crate::php::installed::find_installed;
use crate::php::selection::{self, Origin};
use crate::platform::environment;
use crate::platform::link;
use crate::platform::paths;
use crate::platform::shell;
use crate::project;
use crate::project::shim;

/// Switch the active PHP version, or the one the project pins when none is given
pub fn run(version: Option<String>) -> Result<(), String> {
    match version {
        Some(version) => activate(&version),
        None => pinned_request().and_then(|request| activate(&request)),
    }
}

fn pinned_request() -> Result<String, String> {
    let here = std::env::current_dir()
        .map_err(|error| format!("could not read the current directory: {}", error))?;
    project::find(&here)?
        .and_then(|project| project.php)
        .map(|pinned| pinned.request)
        .ok_or_else(|| {
            "no version given and no .php-version or .phpz.toml with a version found here"
                .to_string()
        })
}

fn activate(requested: &str) -> Result<(), String> {
    let version = find_installed(requested)?;
    let target = paths::versions_directory()?.join(&version);
    let link = paths::current_link()?;

    link::replace_link(&target, &link)?;

    println!("Now using PHP {}", version);
    let _ = shim::ensure();
    if let Ok(Some(selection)) = selection::select_here() {
        if let Origin::Project(file) = &selection.origin {
            if selection.version != version {
                println!(
                    "Note: {} pins PHP {} in this folder, which wins over this default.",
                    file.display(),
                    selection.version
                );
            }
        }
    }
    environment::warn_about_ini_overrides();
    if let Some(note) = shell::path_note() {
        println!("{}", note);
    }
    Ok(())
}
