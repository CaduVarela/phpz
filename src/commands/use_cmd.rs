use crate::environment;
use crate::installed::find_installed;
use crate::link;
use crate::paths;
use crate::shell;

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
    if let Some(note) = shell::path_note() {
        println!("{}", note);
    }
    Ok(())
}
