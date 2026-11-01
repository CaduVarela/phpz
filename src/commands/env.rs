use std::path::Path;

use crate::paths;

/// Print shell code that puts the active version first on PATH.
/// Usage: eval "$(phpz env)", or in PowerShell: phpz env | Invoke-Expression
pub fn run() {
    match paths::shims_directory() {
        Ok(directory) => println!("{}", path_line(&directory, cfg!(windows))),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}

fn path_line(directory: &Path, powershell: bool) -> String {
    if powershell {
        format!("$env:Path = \"{};\" + $env:Path", directory.display())
    } else {
        format!("export PATH=\"{}:$PATH\"", directory.display())
    }
}

#[cfg(test)]
mod tests;
