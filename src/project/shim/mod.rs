use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::php::selection;
use crate::platform::paths;

/// The name the phpz binary is given inside the shims directory.
const SHIM_NAME: &str = "php";

/// Whether this process was started under the name `php`.
pub fn invoked_as_shim() -> bool {
    std::env::args_os()
        .next()
        .map(|name| is_shim_name(&name))
        .unwrap_or(false)
}

fn is_shim_name(name: &OsStr) -> bool {
    Path::new(name)
        .file_stem()
        .map(|stem| stem.eq_ignore_ascii_case(SHIM_NAME))
        .unwrap_or(false)
}

/// How to start the php of one version.
#[derive(Debug, PartialEq)]
struct Launch {
    program: PathBuf,
    set: Vec<(&'static str, PathBuf)>,
    remove: Vec<&'static str>,
}

/// Run php with this version's own ini directory, ignoring variables another PHP left behind.
fn launch_plan(directory: &Path, ini_directory: &Path, windows: bool, has_libexec: bool) -> Launch {
    if windows {
        Launch {
            program: directory.join("php.exe"),
            set: vec![("PHPRC", ini_directory.to_path_buf())],
            remove: vec!["PHP_INI_SCAN_DIR"],
        }
    } else if has_libexec {
        Launch {
            program: directory.join("libexec").join("php"),
            set: vec![
                ("PHPRC", ini_directory.to_path_buf()),
                ("PHP_INI_SCAN_DIR", directory.join("conf.d")),
            ],
            remove: vec![],
        }
    } else {
        Launch {
            program: directory.join("bin").join("php"),
            set: vec![],
            remove: vec![],
        }
    }
}

/// Run the php that applies here; returns the exit code.
pub fn run() -> i32 {
    match launch() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("phpz: {}", error);
            1
        }
    }
}

fn launch() -> Result<i32, String> {
    let selection = selection::require_here()?;
    let directory = selection.directory()?;
    let has_libexec = directory.join("libexec").join("php").exists();

    // Fall back to the version's own settings.
    let ini_directory = match &selection.project {
        Some(project) if !project.extensions.is_empty() => super::ini::ini_directory(
            &directory,
            &selection.version,
            &project.directory,
            &project.extensions,
        )
        .unwrap_or_else(|_| directory.clone()),
        _ => directory.clone(),
    };
    let plan = launch_plan(&directory, &ini_directory, cfg!(windows), has_libexec);

    let mut command = Command::new(&plan.program);
    command.args(std::env::args_os().skip(1));
    for (name, value) in &plan.set {
        command.env(name, value);
    }
    for name in &plan.remove {
        command.env_remove(name);
    }
    start(command, &plan.program)
}

#[cfg(unix)]
fn start(mut command: Command, program: &Path) -> Result<i32, String> {
    use std::os::unix::process::CommandExt;
    let error = command.exec();
    Err(format!("could not run {}: {}", program.display(), error))
}

#[cfg(not(unix))]
fn start(mut command: Command, program: &Path) -> Result<i32, String> {
    let status = command
        .status()
        .map_err(|error| format!("could not run {}: {}", program.display(), error))?;
    Ok(status.code().unwrap_or(1))
}

/// Make sure the shims directory holds a `php` that is this binary.
pub fn ensure() -> Result<(), String> {
    let directory = paths::shims_directory()?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("creating {}: {}", directory.display(), error))?;
    let target = std::env::current_exe()
        .map_err(|error| format!("could not find the phpz binary: {}", error))?;
    install(&target, &directory)
}

#[cfg(unix)]
fn install(target: &Path, directory: &Path) -> Result<(), String> {
    let link = directory.join(SHIM_NAME);
    if fs::read_link(&link)
        .map(|existing| existing == target)
        .unwrap_or(false)
    {
        return Ok(());
    }
    match fs::remove_file(&link) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("replacing {}: {}", link.display(), error)),
    }
    std::os::unix::fs::symlink(target, &link)
        .map_err(|error| format!("creating {}: {}", link.display(), error))
}

#[cfg(windows)]
fn install(target: &Path, directory: &Path) -> Result<(), String> {
    let copy = directory.join(format!("{}.exe", SHIM_NAME));
    let up_to_date = match (fs::metadata(&copy), fs::metadata(target)) {
        (Ok(have), Ok(want)) => {
            have.len() == want.len()
                && matches!((have.modified(), want.modified()), (Ok(a), Ok(b)) if a >= b)
        }
        _ => false,
    };
    if up_to_date {
        return Ok(());
    }
    fs::copy(target, &copy)
        .map(|_| ())
        .map_err(|error| format!("copying phpz to {}: {}", copy.display(), error))
}

#[cfg(test)]
mod tests;
