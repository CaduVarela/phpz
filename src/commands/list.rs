use std::fs;
use crate::paths;

pub fn run() {
    let versions_directory = match paths::versions_directory() {
        Ok(directory) => directory,
        Err(error) => {
            eprintln!("Error: {}", error);
            return;
        }
    };

    if !versions_directory.exists() {
        println!("No versions installed.");
        return;
    }

    println!("Installed versions:");

    match fs::read_dir(&versions_directory) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if entry.file_type().map(|file_type| file_type.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        println!("- {}", name);
                    }
                }
            }
        }
        Err(error) => {
            eprintln!("Error reading versions directory: {}", error);
        }
    }
}
