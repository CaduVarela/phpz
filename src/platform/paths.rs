use std::ffi::OsString;
use std::path::PathBuf;

/// phpz home directory: $PHPZ_HOME when set, otherwise ~/.phpz (%USERPROFILE%\.phpz on Windows)
pub fn phpz_home() -> Result<PathBuf, String> {
    resolve_home(std::env::var_os("PHPZ_HOME"), dirs::home_dir())
}

pub fn resolve_home(
    override_value: Option<OsString>,
    home: Option<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(value) = override_value.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(value));
    }
    home.map(|home| home.join(".phpz"))
        .ok_or_else(|| "could not determine home directory".to_string())
}

pub fn versions_directory() -> Result<PathBuf, String> {
    Ok(phpz_home()?.join("versions"))
}

/// Symlink pointing at the active version's directory.
pub fn current_link() -> Result<PathBuf, String> {
    Ok(phpz_home()?.join("current"))
}

/// Directory with the `php` shim, the one that goes on PATH.
pub fn shims_directory() -> Result<PathBuf, String> {
    Ok(phpz_home()?.join("shims"))
}

/// The directory phpz 0.2 put on PATH, kept to recognize older setups.
pub fn active_bin_directory() -> Result<PathBuf, String> {
    let link = current_link()?;
    Ok(if cfg!(windows) {
        link
    } else {
        link.join("bin")
    })
}
