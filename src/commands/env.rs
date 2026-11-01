use std::path::Path;

use crate::platform::paths;

/// Print shell code that puts the active version first on PATH.
pub fn run() -> Result<(), String> {
    let directory = paths::shims_directory()?;
    println!("{}", path_line(&directory, cfg!(windows)));
    Ok(())
}

pub fn path_line(directory: &Path, powershell: bool) -> String {
    if powershell {
        format!("$env:Path = \"{};\" + $env:Path", directory.display())
    } else {
        format!("export PATH=\"{}:$PATH\"", directory.display())
    }
}
