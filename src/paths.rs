use std::path::PathBuf;

/// phpz home directory (~/.phpz or %USERPROFILE%\.phpz)
pub fn phpz_home() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".phpz"))
        .ok_or_else(|| "could not determine home directory".to_string())
}

pub fn versions_directory() -> Result<PathBuf, String> {
    Ok(phpz_home()?.join("versions"))
}

/// Symlink pointing at the active version's directory.
pub fn current_link() -> Result<PathBuf, String> {
    Ok(phpz_home()?.join("current"))
}
