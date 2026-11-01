use std::fs;
use std::path::Path;

use crate::project::shim;
use crate::release;

/// Update phpz itself to the newest release
pub fn run(check_only: bool, version: Option<String>) -> Result<(), String> {
    update(check_only, version)
}

fn update(check_only: bool, requested: Option<String>) -> Result<(), String> {
    let current = release::parse_version(env!("CARGO_PKG_VERSION"))
        .ok_or("could not read the version of this phpz")?;
    let tag = match requested {
        Some(version) => format!("v{}", version.trim_start_matches('v')),
        None => release::latest_tag()?,
    };
    let target = release::parse_version(&tag)
        .ok_or_else(|| format!("'{}' is not a release version", tag))?;

    if let Ok(executable) = std::env::current_exe() {
        let _ = fs::remove_file(executable.with_extension("old"));
    }
    if target == current {
        println!("phpz {} is already installed.", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if check_only {
        println!(
            "phpz {} is available (you have {}). Run 'phpz self-update' to install it.",
            tag.trim_start_matches('v'),
            env!("CARGO_PKG_VERSION")
        );
        return Ok(());
    }

    let executable = std::env::current_exe()
        .map_err(|error| format!("could not find the phpz program: {}", error))?;
    let staged = executable.with_file_name(".phpz.update");
    let result =
        release::download_binary(&tag, &staged).and_then(|_| replace(&staged, &executable));
    let _ = fs::remove_file(&staged);
    result?;

    let _ = shim::ensure();
    println!(
        "Updated phpz from {} to {}.",
        env!("CARGO_PKG_VERSION"),
        tag.trim_start_matches('v')
    );
    Ok(())
}

#[cfg(unix)]
fn replace(staged: &Path, executable: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(staged, fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())?;
    fs::rename(staged, executable).map_err(|error| {
        format!(
            "could not replace {}: {}. Is it in a directory you can write to?",
            executable.display(),
            error
        )
    })
}

/// Windows cannot overwrite a running program, but it can rename it out of the way.
#[cfg(windows)]
fn replace(staged: &Path, executable: &Path) -> Result<(), String> {
    let old = executable.with_extension("old");
    let _ = fs::remove_file(&old);
    fs::rename(executable, &old)
        .map_err(|error| format!("could not move {} aside: {}", executable.display(), error))?;
    if let Err(error) = fs::copy(staged, executable) {
        let _ = fs::rename(&old, executable);
        return Err(format!(
            "could not write {}: {}",
            executable.display(),
            error
        ));
    }
    let _ = fs::remove_file(&old);
    Ok(())
}
