use std::path::Path;

use crate::platform::paths;

/// Print shell code that puts the active version first on PATH.
pub fn run() {
    match paths::shims_directory() {
        Ok(directory) => println!("{}", path_line(&directory, cfg!(windows))),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}

pub fn path_line(directory: &Path, powershell: bool) -> String {
    if powershell {
        format!("$env:Path = \"{};\" + $env:Path", directory.display())
    } else {
        format!("export PATH=\"{}:$PATH\"", directory.display())
    }
}
