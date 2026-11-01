use crate::installed;
use crate::selection::{self, Origin, Selection};

/// List installed versions, marking the one that applies here and the default
pub fn run() {
    if let Err(error) = list() {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn list() -> Result<(), String> {
    let versions = installed::installed_versions()?;
    if versions.is_empty() {
        println!("No versions installed.");
        return Ok(());
    }

    // A pin that cannot be honored is reported below the list, not instead of it.
    let (effective, problem) = match selection::select_here() {
        Ok(selection) => (selection, None),
        Err(error) => (None, Some(error)),
    };
    let global = installed::active_version()?;

    println!("Installed versions:");
    for version in &versions {
        println!("{}", label(version, effective.as_ref(), global.as_deref()));
    }
    if let Some(problem) = problem {
        println!("\nNote: {}", problem);
    }
    Ok(())
}

/// One line of the list: "* 8.2.35 (active, set by ...)", "  8.4.27 (default)" or "  8.3.32".
fn label(version: &str, effective: Option<&Selection>, global: Option<&str>) -> String {
    let is_effective = effective.map(|selection| selection.version == version).unwrap_or(false);
    if is_effective {
        return match effective.map(|selection| &selection.origin) {
            Some(Origin::Project(file)) => format!("* {} (active, set by {})", version, file.display()),
            _ => format!("* {} (active)", version),
        };
    }
    if global == Some(version) {
        return format!("  {} (default)", version);
    }
    format!("  {}", version)
}

#[cfg(test)]
mod tests;
