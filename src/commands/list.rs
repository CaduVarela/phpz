use crate::installed;

/// List installed versions, marking the active one
pub fn run() {
    let result = installed::installed_versions().and_then(|versions| {
        installed::active_version().map(|active| (versions, active))
    });

    match result {
        Ok((versions, _)) if versions.is_empty() => println!("No versions installed."),
        Ok((versions, active)) => {
            println!("Installed versions:");
            for version in versions {
                if active.as_deref() == Some(version.as_str()) {
                    println!("* {} (active)", version);
                } else {
                    println!("  {}", version);
                }
            }
        }
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
