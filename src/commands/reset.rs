use crate::{link, paths};

/// Deactivate phpz's PHP: remove the `current` link.
pub fn run() {
    let result = paths::current_link().and_then(|current| link::remove_link(&current));

    match result {
        Ok(true) => println!("Deactivated the active PHP version."),
        Ok(false) => println!("No PHP version is active."),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
