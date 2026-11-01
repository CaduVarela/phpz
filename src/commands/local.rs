use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::php::installed;
use crate::project::{self, PROJECT_FILE, VERSION_FILE};

/// Pin the PHP version of the current directory, or show it
pub fn run(version: Option<String>, unset: bool) {
    if let Err(error) = execute(version, unset) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn execute(version: Option<String>, unset: bool) -> Result<(), String> {
    let here = std::env::current_dir()
        .map_err(|error| format!("could not read the current directory: {}", error))?;
    match (version, unset) {
        (Some(version), false) => pin(&here, &version),
        (None, true) => unpin(&here),
        (None, false) => show(&here),
        (Some(_), true) => Err("give either a version or --unset, not both".to_string()),
    }
}

fn pin(directory: &Path, requested: &str) -> Result<(), String> {
    let version = project::normalize_request(requested).ok_or_else(|| {
        format!(
            "invalid version '{}'. Expected MAJOR, MAJOR.MINOR or MAJOR.MINOR.PATCH",
            requested
        )
    })?;
    if project::project_file_pins_php(directory)? {
        return Err(format!(
            "{} already sets 'php' here; edit that file instead.",
            PROJECT_FILE
        ));
    }

    let path = directory.join(VERSION_FILE);
    fs::write(&path, format!("{}\n", version))
        .map_err(|error| format!("writing {}: {}", path.display(), error))?;
    println!(
        "PHP {} will be used in {} (written to {})",
        version,
        directory.display(),
        VERSION_FILE
    );

    if installed::select_installed(&installed::installed_versions()?, &version).is_err() {
        println!(
            "Note: PHP {} is not installed yet. Run 'phpz install {}'.",
            version, version
        );
    }
    Ok(())
}

fn unpin(directory: &Path) -> Result<(), String> {
    let path = directory.join(VERSION_FILE);
    match fs::remove_file(&path) {
        Ok(()) => println!("Removed {}", path.display()),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            println!("There is no {} in this directory.", VERSION_FILE)
        }
        Err(error) => return Err(format!("removing {}: {}", path.display(), error)),
    }
    Ok(())
}

fn show(directory: &Path) -> Result<(), String> {
    match project::find(directory)?.and_then(|project| project.php) {
        Some(pinned) => {
            let versions = installed::installed_versions()?;
            let state = match installed::select_installed(&versions, &pinned.request) {
                Ok(installed) => format!("installed as {}", installed),
                Err(_) => "not installed".to_string(),
            };
            println!("PHP {} (set by {}, {})", pinned.request, pinned.file.display(), state);
        }
        None => println!("No project version applies here, so the default is used. Pin one with 'phpz local <version>'."),
    }
    Ok(())
}
