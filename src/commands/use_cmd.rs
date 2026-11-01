use std::fs;
use std::path::Path;

use crate::paths;

/// Switch the active PHP version
pub fn run(version: String) {
    if let Err(error) = activate(&version) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

/// Names of installed versions (finished installations only), sorted ascending.
pub fn installed_versions() -> Result<Vec<String>, String> {
    let versions_directory = paths::versions_directory()?;
    if !versions_directory.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&versions_directory)
        .map_err(|error| format!("reading versions directory: {}", error))?;

    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|file_type| file_type.is_dir()).unwrap_or(false))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !name.ends_with(".tmp"))
        .collect();
    names.sort_by_key(|name| version_sort_key(name));
    Ok(names)
}

fn version_sort_key(name: &str) -> Vec<u32> {
    name.split('.').filter_map(|part| part.parse().ok()).collect()
}

/// Find the newest installed version matching "8", "8.3" or "8.3.32".
fn find_installed(requested: &str) -> Result<String, String> {
    let requested_parts: Vec<u32> = requested
        .split('.')
        .map(|part| part.parse::<u32>())
        .collect::<Result<_, _>>()
        .map_err(|_| format!("invalid version '{}'", requested))?;

    installed_versions()?
        .into_iter()
        .filter(|name| version_sort_key(name).starts_with(&requested_parts))
        .last()
        .ok_or_else(|| {
            format!(
                "PHP {} is not installed. Run 'phpz install {}' first.",
                requested, requested
            )
        })
}

fn activate(requested: &str) -> Result<(), String> {
    let version = find_installed(requested)?;
    let target = paths::versions_directory()?.join(&version);
    let link = paths::current_link()?;

    replace_link(&target, &link)?;

    println!("Now using PHP {}", version);
    if !is_on_path(&link.join("bin")) {
        println!("Note: add phpz to your PATH with: eval \"$(phpz env)\"");
    }
    Ok(())
}

/// Atomically point `link` at `target` (create a temporary link, then rename over it).
#[cfg(unix)]
fn replace_link(target: &Path, link: &Path) -> Result<(), String> {
    let temporary_link = link.with_extension("tmp");
    let _ = fs::remove_file(&temporary_link);
    std::os::unix::fs::symlink(target, &temporary_link)
        .map_err(|error| format!("creating symlink: {}", error))?;
    fs::rename(&temporary_link, link).map_err(|error| {
        let _ = fs::remove_file(&temporary_link);
        format!("updating symlink: {}", error)
    })
}

#[cfg(not(unix))]
fn replace_link(_target: &Path, _link: &Path) -> Result<(), String> {
    Err("switching versions is not implemented on this platform yet".to_string())
}

fn is_on_path(directory: &Path) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|entry| entry == directory))
        .unwrap_or(false)
}
