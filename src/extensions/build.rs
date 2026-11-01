use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::download;
use crate::download::consent;
use crate::download::lock::DownloadLock;
use crate::extensions::install::Installed;
use crate::extensions::pecl;
use crate::php::source_build;
use crate::platform::paths;

/// Compile `name` from its PECL source against a PHP that was compiled here, and install the `.so`.
pub fn build_package(
    version_directory: &Path,
    php_version: &str,
    name: &str,
    requested: Option<&str>,
    allow_unverified: bool,
) -> Result<Installed, String> {
    if !pecl::is_valid_name(name) {
        return Err(format!("'{}' is not a valid extension name", name));
    }
    let phpize = version_directory.join("bin").join("phpize");
    let php_config = version_directory.join("bin").join("php-config");
    if !phpize.exists() || !php_config.exists() {
        return Err(format!(
            "PHP {} cannot build extensions because it was not compiled here. \
             Run 'phpz install {} --build --replace' first.",
            php_version, php_version
        ));
    }
    let missing = missing_tools(source_build::tool_exists);
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|(what, _)| *what).collect();
        return Err(format!(
            "building an extension needs {}. On Debian or Ubuntu: sudo apt install build-essential autoconf",
            names.join(" and ")
        ));
    }

    let package = pecl::find_source(name, requested)?;
    let mut lock = DownloadLock::load()?;
    let remembered = lock.get(&package.url).map(str::to_string);
    if remembered.is_none() {
        let subject = format!("{} {}", name, package.version);
        consent::require(true, &subject, allow_unverified)?;
    }

    let work_directory = paths::phpz_home()?
        .join("build")
        .join(format!("{}-{}", name, package.version));
    let _ = fs::remove_dir_all(&work_directory);
    fs::create_dir_all(&work_directory)
        .map_err(|error| format!("creating {}: {}", work_directory.display(), error))?;
    let log_path = work_directory.join("build.log");

    let result = compile(
        &package,
        name,
        &phpize,
        &php_config,
        &work_directory,
        &log_path,
        remembered.as_deref(),
    );
    match result {
        Ok((actual, module)) => {
            let target = module_target(&php_config, name)?;
            install_module(&module, &target)?;
            if remembered.is_none() {
                lock.record(&package.url, &actual);
                if let Err(error) = lock.save() {
                    let _ = fs::remove_file(&target);
                    return Err(error);
                }
            } else {
                println!("Matches the checksum recorded when you first installed it.");
            }
            let _ = fs::remove_dir_all(&work_directory);
            Ok(Installed {
                version: package.version,
                first_use: remembered.is_none(),
            })
        }
        Err(error) => {
            for leftover in ["source", "package.tgz"] {
                let _ = fs::remove_dir_all(work_directory.join(leftover));
            }
            let _ = fs::remove_file(work_directory.join("package.tgz"));
            Err(error)
        }
    }
}

/// The tools an extension build needs that are missing, with the package that has them.
fn missing_tools(has_tool: impl Fn(&str) -> bool) -> Vec<(&'static str, &'static str)> {
    let mut missing = source_build::missing_tools(&has_tool);
    if !has_tool("autoconf") {
        missing.push(("autoconf", "autoconf"));
    }
    missing
}

/// Download, unpack and compile; returns the SHA-256 of the download and the built module.
fn compile(
    package: &pecl::Package,
    name: &str,
    phpize: &Path,
    php_config: &Path,
    work_directory: &Path,
    log_path: &Path,
    remembered: Option<&str>,
) -> Result<(String, PathBuf), String> {
    let archive_path = work_directory.join("package.tgz");
    let actual = download::fetch(&package.url, &archive_path, remembered).map_err(|error| {
        if remembered.is_some() {
            format!("{} The file at this address is not the one you installed before, so nothing was installed.", error)
        } else {
            error
        }
    })?;

    let source = unpack(&archive_path, &work_directory.join("source"))?;
    let jobs = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1);
    println!(
        "Compiling {} {} with {} jobs; the log is {}",
        name,
        package.version,
        jobs,
        log_path.display()
    );

    let mut prepare = Command::new(phpize);
    prepare.current_dir(&source);
    source_build::run_logged(prepare, "phpize", log_path)?;

    let mut configure = Command::new(source.join("configure"));
    configure
        .current_dir(&source)
        .arg(format!("--with-php-config={}", php_config.display()));
    source_build::run_logged(configure, "configure", log_path)?;

    let mut make = Command::new("make");
    make.current_dir(&source).arg(format!("-j{}", jobs));
    source_build::run_logged(make, "compiling", log_path)?;

    let module = source.join("modules").join(format!("{}.so", name));
    if !module.exists() {
        return Err(format!(
            "the build finished but did not produce {}.so; see {}",
            name,
            log_path.display()
        ));
    }
    Ok((actual, module))
}

/// Unpack a PECL package and return the directory that holds its `config.m4`.
fn unpack(archive_path: &Path, destination: &Path) -> Result<PathBuf, String> {
    let file = fs::File::open(archive_path).map_err(|error| error.to_string())?;
    tar::Archive::new(flate2::read::GzDecoder::new(file))
        .unpack(destination)
        .map_err(|error| format!("unpacking the package: {}", error))?;
    find_source_root(destination)
        .ok_or_else(|| "the package does not contain a config.m4".to_string())
}

fn find_source_root(directory: &Path) -> Option<PathBuf> {
    if directory.join("config.m4").exists() {
        return Some(directory.to_path_buf());
    }
    fs::read_dir(directory)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .find(|path| path.join("config.m4").exists())
}

/// Where PHP looks for `name.so`, asked of `php-config`.
fn module_target(php_config: &Path, name: &str) -> Result<PathBuf, String> {
    let output = Command::new(php_config)
        .arg("--extension-dir")
        .output()
        .map_err(|error| format!("could not run php-config: {}", error))?;
    let directory = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || directory.is_empty() {
        return Err("php-config did not say where extensions go".to_string());
    }
    Ok(PathBuf::from(directory).join(format!("{}.so", name)))
}

fn install_module(module: &Path, target: &Path) -> Result<(), String> {
    let directory = target.parent().ok_or("invalid extension directory")?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("creating {}: {}", directory.display(), error))?;
    fs::copy(module, target)
        .map(|_| ())
        .map_err(|error| format!("copying to {}: {}", target.display(), error))
}

#[cfg(test)]
mod tests;
