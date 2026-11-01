use crate::project_ini;

/// Remove generated files of projects that no longer exist
pub fn run(dry_run: bool) {
    match project_ini::prune(dry_run) {
        Ok(removed) if removed.is_empty() => println!("Nothing to remove."),
        Ok(removed) => {
            let verb = if dry_run { "Would remove" } else { "Removed" };
            for project in &removed {
                println!("{} the settings generated for {}", verb, project);
            }
        }
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
