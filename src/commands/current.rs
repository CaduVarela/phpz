use std::fs;

use crate::paths;

/// Show the active PHP version
pub fn run() {
    match active_version() {
        Ok(Some(version)) => println!("{}", version),
        Ok(None) => println!("No PHP version is active. Run 'phpz use <version>'."),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}

fn active_version() -> Result<Option<String>, String> {
    let link = paths::current_link()?;
    match fs::read_link(&link) {
        Ok(target) => Ok(target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("reading {}: {}", link.display(), error)),
    }
}
