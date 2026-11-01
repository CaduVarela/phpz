use crate::environment;
use crate::installed::find_installed;
use crate::link;
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

    link::replace_link(&target, &link)?;

    println!("Now using PHP {}", version);
    environment::warn_about_ini_overrides();
    if !shell::path_is_active() {
        if cfg!(windows) {
            println!("Note: add {} to your PATH.", paths::active_bin_directory()?.display());
            return Ok(());
        }
        match Shell::detect() {
            Some(shell) if shell::is_configured(&shell) => {
                if let Ok(profile) = shell.profile_path() {
                    println!("Note: {}", shell::apply_hint(&profile));
                }
            }
            _ => println!("Note: run 'phpz setup' to add phpz to your PATH."),
        }
    }
    Ok(())
}
