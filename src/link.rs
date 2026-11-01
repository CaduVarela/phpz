use std::fs;
use std::io::ErrorKind;
use std::path::Path;

/// Atomically point `link` at the directory `target` (create a temporary link, then rename over it).
#[cfg(unix)]
pub fn replace_link(target: &Path, link: &Path) -> Result<(), String> {
    let temporary_link = link.with_extension("tmp");
    let _ = fs::remove_file(&temporary_link);
    std::os::unix::fs::symlink(target, &temporary_link)
        .map_err(|error| format!("creating symlink: {}", error))?;
    fs::rename(&temporary_link, link).map_err(|error| {
        let _ = fs::remove_file(&temporary_link);
        format!("updating symlink: {}", error)
    })
}

/// Point `link` at `target` with a junction, which needs no administrator rights.
#[cfg(windows)]
pub fn replace_link(target: &Path, link: &Path) -> Result<(), String> {
    remove_link(link)?;
    junction::create(target, link).map_err(|error| format!("creating junction: {}", error))
}

/// Remove `link` without touching the directory it points to. Ok(false) when there is none.
pub fn remove_link(link: &Path) -> Result<bool, String> {
    // On Windows a junction is removed like an (empty) directory.
    let result = if cfg!(windows) {
        fs::remove_dir(link)
    } else {
        fs::remove_file(link)
    };

    match result {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("removing {}: {}", link.display(), error)),
    }
}

#[cfg(test)]
mod tests;
