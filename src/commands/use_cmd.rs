use std::fs;
use std::path::Path;

use crate::installed::find_installed;
use crate::paths;
use crate::shell::{self, Shell};

/// Switch the active PHP version
pub fn run(version: String) {
    if let Err(error) = activate(&version) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn activate(requested: &str) -> Result<(), String> {
    let version = find_installed(requested)?;
    let target = paths::versions_directory()?.join(&version);
    let link = paths::current_link()?;

    replace_link(&target, &link)?;

    println!("Now using PHP {}", version);
    if !shell::path_is_active() {
        match Shell::detect() {
            Some(shell) if shell::is_configured(&shell) => {
                println!("Note: open a new terminal to pick up the change.")
            }
            _ => println!("Note: run 'phpz setup' to add phpz to your PATH."),
        }
    }
    Ok(())
}

/// Atomically point `link` at `target` (create a temporary link, then rename over it).
#[cfg(unix)]
fn replace_link(target: &Path, link: &Path) -> Result<(), String> {
    let temporary_link = link.with_extension("tmp");
    let _ = fs::remove_file(&temporary_link);
    std::os::unix::fs::symlink(target, &temporary_link)
        .map_err(|error| format!("creating symlink: {}", error))?;
    fs::rename(&temporary_link, link).map_err(|error| {
        let _ = fs::remove_file(&temporary_link);
        format!("updating symlink: {}", error)
    })
}

#[cfg(not(unix))]
fn replace_link(_target: &Path, _link: &Path) -> Result<(), String> {
    Err("switching versions is not implemented on this platform yet".to_string())
}
