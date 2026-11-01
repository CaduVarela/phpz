use std::fs;

use crate::paths;

/// Deactivate phpz's PHP: remove the `current` link.
pub fn run() {
    let link = match paths::current_link() {
        Ok(link) => link,
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    };

    match fs::remove_file(&link) {
        Ok(()) => println!("Deactivated the active PHP version."),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!("No PHP version is active.")
        }
        Err(error) => {
            eprintln!("Error: removing {}: {}", link.display(), error);
            std::process::exit(1);
        }
    }
}
