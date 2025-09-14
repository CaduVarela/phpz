use std::fs;
use std::path::PathBuf;

pub fn run() {
    let versions_dir = phpz_home().join("versions");

    if !versions_dir.exists() {
        println!("No versions installed.");
        return;
    }

    println!("Installed versions:");

    match fs::read_dir(&versions_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        println!("- {}", name);
                    }
                }
            }
        }
        Err(err) => {
            eprintln!("Error reading versions directory: {}", err);
        }
    }
}

/// Get phpz home directory (~/.phpz or %USERPROFILE%\.phpz)
fn phpz_home() -> PathBuf {
    if let Some(home) = dirs::home_dir() {
        return home.join(".phpz");
    }
    panic!("Could not determine home directory");
}
