use crate::project;

/// Remove generated files of projects that no longer exist
pub fn run(dry_run: bool) -> Result<(), String> {
    let removed = project::ini::prune(dry_run)?;
    if removed.is_empty() {
        println!("Nothing to remove.");
    }
    let verb = if dry_run { "Would remove" } else { "Removed" };
    for project in &removed {
        println!("{} the settings generated for {}", verb, project);
    }
    Ok(())
}
