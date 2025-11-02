use std::fs;
use std::path::PathBuf;
use crate::version;

/// Install a specific PHP version
pub fn run(version: String) {
    println!("Resolving version '{}'...", version);

    let resolved = match version::resolve_version(&version) {
        Some(v) => v,
        None => {
            eprintln!(
                "Error: Could not resolve version '{}'. Version not found or invalid format.",
                version
            );
            eprintln!("Expected format: MAJOR (e.g., '8'), MAJOR.MINOR (e.g., '8.2'), or MAJOR.MINOR.PATCH (e.g., '8.3.9')");
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

/// Get phpz home directory (~/.phpz or %USERPROFILE%\.phpz)
fn phpz_home() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        return home.join(".phpz");
    }
    panic!("Could not determine home directory");
}
