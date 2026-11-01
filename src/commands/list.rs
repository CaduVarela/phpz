use crate::php::installed;
use crate::php::selection::{self, Origin, Selection};
use crate::php::variant::{self, Origin as BuildOrigin};
use crate::platform::paths;

/// List installed versions, marking the one that applies here and the default
pub fn run() -> Result<(), String> {
    list()
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
    let versions_directory = paths::versions_directory()?;
    for version in &versions {
        // Windows has one build, so there is nothing to tell apart there.
        let build = if cfg!(windows) {
            None
        } else {
            Some(variant::read_origin(&versions_directory.join(version)))
        };
        println!(
            "{}",
            label(version, effective.as_ref(), global.as_deref(), build)
        );
    }
    if let Some(problem) = problem {
        println!("\nNote: {}", problem);
    }
    Ok(())
}

/// One line of the list: "* 8.2.35 (active, set by ...)", "  8.4.27 (default)" or "  8.3.32".
pub fn label(
    version: &str,
    effective: Option<&Selection>,
    global: Option<&str>,
    build: Option<BuildOrigin>,
) -> String {
    let line = base_label(version, effective, global);
    match build {
        Some(build) if build != BuildOrigin::Prebuilt(variant::DEFAULT) => {
            format!("{} [{}]", line, build.name())
        }
        _ => line,
    }
}

fn base_label(version: &str, effective: Option<&Selection>, global: Option<&str>) -> String {
    let is_effective = effective
        .map(|selection| selection.version == version)
        .unwrap_or(false);
    if is_effective {
        return match effective.map(|selection| &selection.origin) {
            Some(Origin::Project(file)) => {
                format!("* {} (active, set by {})", version, file.display())
            }
            _ => format!("* {} (active)", version),
        };
    }
    if global == Some(version) {
        return format!("  {} (default)", version);
    }
    format!("  {}", version)
}
