use std::fs;
use std::path::PathBuf;

/// Install a specific PHP version
pub fn run(version: String) {
    let resolved = match resolve_version(&version) {
        Some(v) => v,
        None => {
            eprintln!(
                "Invalid version format: '{}'. Expected MAJOR, MAJOR.MINOR or MAJOR.MINOR.PATCH",
                version
            );
            return;
        }
    };

    println!("Installing PHP {}", resolved);

    let versions_dir = phpz_home().join("versions");
    let final_dir = versions_dir.join(&resolved);
    let tmp_dir = versions_dir.join(format!("{}.tmp", resolved));

    // Ensure base directory exists
    if let Err(e) = fs::create_dir_all(&versions_dir) {
        eprintln!("Error creating versions directory: {}", e);
        return;
    }

    // Check if version is already installed
    if final_dir.exists() {
        println!("PHP {} is already installed.", resolved);
        return;
    }

    // Clean up any previous tmp installation
    if tmp_dir.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    // Create temporary directory
    if let Err(e) = fs::create_dir_all(&tmp_dir) {
        eprintln!("Error creating temporary directory: {}", e);
        return;
    }

    // TODO: resolve correct download URL (version.rs)
    // TODO: download PHP binary package into tmp_dir
    // TODO: extract files into tmp_dir
    // TODO: verify integrity (hash/signature)
    // TODO: place php.ini template in tmp_dir

    // If everything succeeds, move tmp -> final
    if let Err(e) = fs::rename(&tmp_dir, &final_dir) {
        eprintln!("Error finalizing installation: {}", e);
        let _ = fs::remove_dir_all(&tmp_dir);
        return;
    }

    println!("Successfully installed PHP {}", resolved);
}

/// Resolve user input into a full version string
/// - "8"   -> "8.x"
/// - "8.2" -> "8.2.x"
/// - "8.3.9" -> "8.3.9"
fn resolve_version(input: &str) -> Option<String> {
    let parts: Vec<&str> = input.split('.').collect();

    // Validate all parts are numeric
    if !parts.iter().all(|p| p.parse::<u32>().is_ok()) {
        return None;
    }

    match parts.len() {
        1 => Some(format!("{}.x", parts[0])),
        2 => Some(format!("{}.{}.x", parts[0], parts[1])),
        3 => Some(input.to_string()),
        _ => None,
    }
}

/// Get phpz home directory (~/.phpz or %USERPROFILE%\.phpz)
fn phpz_home() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        return home.join(".phpz");
    }
    panic!("Could not determine home directory");
}
