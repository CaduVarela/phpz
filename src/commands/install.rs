use std::fs::{self, File};
use std::io;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;

use std::collections::BTreeSet;

use crate::settings::Settings;
use crate::variant::{self, Origin, Variant};
use crate::version::Build;
use crate::{consent, download, extensions, interaction, paths, shell, shim, source_build, version};

const DEFAULT_INI: &str = "\
; php.ini managed by phpz
[PHP]
memory_limit = 512M
error_reporting = E_ALL
display_errors = On
date.timezone = UTC
";

/// Install a specific PHP version
pub fn run(version: String, allow_unverified: bool, variant: Option<Variant>, replace: bool, build: bool, jobs: Option<usize>) {
    let result = if build {
        install_from_source(&version, replace, jobs)
    } else if replace && variant.is_none() {
        Err("--replace needs --variant (or --build) to say what to switch the version to".to_string())
    } else {
        install(&version, allow_unverified, variant, replace)
    };
    if let Err(error) = result {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

/// Move an installed version to another build (used by `phpz ext enable` when an extension needs it).
pub fn switch_build(version: &str, variant: Variant) -> Result<(), String> {
    install(version, false, Some(variant), true)
}

/// Which build to install: the one asked for, or the default, or a larger one the user agrees to
/// because their global extension settings need it. Windows has a single official build.
fn choose_variant(requested: Option<Variant>) -> Result<Variant, String> {
    if cfg!(windows) {
        return match requested {
            Some(_) => Err("--variant is for Linux and macOS. Windows has one official build per version.".to_string()),
            None => Ok(variant::DEFAULT),
        };
    }
    if let Some(variant) = requested {
        variant::runs_here(variant)?;
        return Ok(variant);
    }
    // Looking for a larger build must never get in the way of installing the usual one.
    Ok(offer_larger_build().ok().flatten().unwrap_or(variant::DEFAULT))
}

/// When the global extension settings enable something the default build lacks, offer the smallest
/// build that has all of it, and say what that build lacks. Only in a terminal, and only asking.
fn offer_larger_build() -> Result<Option<Variant>, String> {
    if !interaction::is_interactive() {
        return Ok(None);
    }
    let wanted: BTreeSet<String> = Settings::load()?.extensions.enable.into_iter().collect();
    if wanted.is_empty() {
        return Ok(None);
    }
    let default_extensions = version::variant_extensions(variant::DEFAULT)?;
    let missing: Vec<String> = wanted.difference(&default_extensions).cloned().collect();
    if missing.is_empty() {
        return Ok(None);
    }

    let mut candidates = Vec::new();
    for candidate in Variant::ALL {
        if candidate == variant::DEFAULT || variant::runs_here(candidate).is_err() {
            continue;
        }
        if let Ok(extensions) = version::variant_extensions(candidate) {
            candidates.push((candidate, extensions));
        }
    }
    let Some(target) = variant::smallest_covering(&candidates, &wanted) else {
        return Ok(None);
    };
    let target_extensions = candidates
        .iter()
        .find(|(candidate, _)| *candidate == target)
        .map(|(_, extensions)| extensions.clone())
        .unwrap_or_default();

    eprintln!("Your global extension settings enable {}, which the {} build lacks.", missing.join(", "), variant::DEFAULT.name());
    eprintln!("The {} build has them, and is a larger download.", target.name());
    let lost = variant::lost_by_switching(&default_extensions, &target_extensions);
    if !lost.is_empty() {
        eprintln!("It lacks {}, which the {} build has.", lost.join(", "), variant::DEFAULT.name());
    }
    let question = format!("Use the {} build for this version?", target.name());
    Ok(interaction::confirm(&question, false).filter(|answer| *answer).map(|_| target))
}

fn install(
    requested_version: &str,
    allow_unverified: bool,
    requested_variant: Option<Variant>,
    replace: bool,
) -> Result<(), String> {
    let variant = choose_variant(requested_variant)?;
    println!("Resolving version '{}'...", requested_version);
    let build = version::resolve_build(requested_version, variant)?;
    let resolved_version = build.version_name();

    let versions_directory = paths::versions_directory()?;
    let final_directory = versions_directory.join(&resolved_version);
    let temporary_directory = versions_directory.join(format!("{}.tmp", resolved_version));

    fs::create_dir_all(&versions_directory)
        .map_err(|error| format!("creating versions directory: {}", error))?;

    let replacing = if final_directory.exists() {
        let existing = variant::read_origin(&final_directory);
        if replace && !cfg!(windows) && existing != Origin::Prebuilt(variant) {
            true
        } else {
            let which = if cfg!(windows) { String::new() } else { format!(" (the {} build)", existing.name()) };
            println!("PHP {} is already installed{}.", resolved_version, which);
            if !cfg!(windows) && requested_variant.is_some() && existing != Origin::Prebuilt(variant) {
                println!("To switch it to the {} build, add --replace.", variant.name());
            }
            return Ok(());
        }
    } else {
        false
    };

    confirm_unverified(&build, allow_unverified)?;

    // Clean up leftovers from a previous failed attempt
    if temporary_directory.exists() {
        let _ = fs::remove_dir_all(&temporary_directory);
    }
    fs::create_dir_all(&temporary_directory)
        .map_err(|error| format!("creating temporary directory: {}", error))?;

    if replacing {
        println!(
            "Switching PHP {} from the {} build to the {} build",
            resolved_version,
            variant::read_origin(&final_directory).name(),
            variant.name()
        );
    } else if cfg!(windows) || variant == variant::DEFAULT {
        println!("Installing PHP {}", resolved_version);
    } else {
        println!("Installing PHP {} (the {} build)", resolved_version, variant.name());
    }
    let result = populate(&build, &temporary_directory).and_then(|_| {
        if replacing {
            carry_over_configuration(&final_directory, &temporary_directory)?;
        }
        apply_global_extension_settings(&temporary_directory);
        if replacing {
            let backup = versions_directory.join(format!("{}.old", resolved_version));
            swap_in(&temporary_directory, &final_directory, &backup)
        } else {
            fs::rename(&temporary_directory, &final_directory)
                .map_err(|error| format!("finalizing installation: {}", error))
        }
    });

    if let Err(error) = result {
        let _ = fs::remove_dir_all(&temporary_directory);
        return Err(error);
    }

    println!("Successfully installed PHP {}", resolved_version);
    let _ = shim::ensure();
    // The question belongs to a first install, not to moving a version to another build.
    if !replacing {
        shell::offer_setup();
    }
    Ok(())
}

/// Compile a version from the official source at php.net, on Linux. The archive is checked against
/// the SHA-256 and the PGP signature php.net publishes, and what the machine lacks is reported.
fn install_from_source(requested_version: &str, replace: bool, jobs: Option<usize>) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("--build is available on Linux only for now.".to_string());
    }
    println!("Looking for PHP '{}' at php.net...", requested_version);
    let release = source_build::resolve(requested_version)?;
    if release.series() < source_build::OLDEST_BUILDABLE {
        let (major, minor) = source_build::OLDEST_BUILDABLE;
        return Err(format!("the build recipe covers PHP {}.{} and newer, not {}.", major, minor, release.version));
    }

    let versions_directory = paths::versions_directory()?;
    let final_directory = versions_directory.join(&release.version);
    let temporary_directory = versions_directory.join(format!("{}.tmp", release.version));
    fs::create_dir_all(&versions_directory).map_err(|error| format!("creating versions directory: {}", error))?;

    let replacing = if final_directory.exists() {
        let existing = variant::read_origin(&final_directory);
        if replace && existing != Origin::Source {
            true
        } else {
            println!("PHP {} is already installed (built from {}).", release.version, if existing == Origin::Source { "source".to_string() } else { format!("the {} build", existing.name()) });
            if existing != Origin::Source {
                println!("To replace it with one compiled here, add --replace.");
            }
            return Ok(());
        }
    } else {
        false
    };

    let missing = source_build::missing_tools(source_build::tool_exists);
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|(name, _)| *name).collect();
        return Err(format!(
            "building PHP needs {}. On Debian or Ubuntu: sudo apt install build-essential",
            names.join(" and ")
        ));
    }
    let has_pkg_config = source_build::tool_exists("pkg-config");
    if !has_pkg_config {
        println!("Note: pkg-config was not found, so only the extensions that need no library will be built.");
    }
    let plan = source_build::plan(|package| has_pkg_config && source_build::pkg_config_has(package));
    if release.series() < source_build::FIRST_OPENSSL_3_SERIES {
        println!("Note: PHP before 8.1 may fail to compile with the OpenSSL 3 of current distributions.");
    }

    println!("Compiling PHP {} from the official source", release.version);
    println!("  compiled in: {}", plan.built_in.join(", "));
    println!("  as files   : {} (switch them with 'phpz ext enable|disable')", plan.shared.join(", "));
    for skipped in &plan.skipped {
        println!("  left out   : {} (on Debian or Ubuntu: sudo apt install {})", skipped.name, skipped.hint);
    }

    let work_directory = paths::phpz_home()?.join("build").join(&release.version);
    let _ = fs::remove_dir_all(&work_directory);
    fs::create_dir_all(&work_directory).map_err(|error| format!("creating {}: {}", work_directory.display(), error))?;
    let log_path = work_directory.join("build.log");
    let jobs = jobs
        .or_else(|| std::thread::available_parallelism().ok().map(|count| count.get()))
        .unwrap_or(1)
        .max(1);

    let result = compile_and_stage(&release, &final_directory, &work_directory, &plan, jobs, &log_path).and_then(|tree| {
        let _ = fs::remove_dir_all(&temporary_directory);
        fs::rename(&tree, &temporary_directory).map_err(|error| format!("moving the build into place: {}", error))?;
        if replacing {
            carry_over_configuration(&final_directory, &temporary_directory)?;
        }
        apply_global_extension_settings(&temporary_directory);
        if replacing {
            swap_in(&temporary_directory, &final_directory, &versions_directory.join(format!("{}.old", release.version)))
        } else {
            fs::rename(&temporary_directory, &final_directory).map_err(|error| format!("finalizing installation: {}", error))
        }
    });

    match result {
        Ok(()) => {
            let _ = fs::remove_dir_all(&work_directory);
        }
        Err(error) => {
            // The source tree is large; the log is what is worth keeping.
            for leftover in ["src", "staging", "gnupg", "php.tar.gz", "php-keyring.gpg", "signature.asc"] {
                let _ = fs::remove_dir_all(work_directory.join(leftover));
            }
            let _ = fs::remove_dir_all(&temporary_directory);
            return Err(error);
        }
    }

    println!("Successfully installed PHP {} (compiled from source)", release.version);
    let _ = shim::ensure();
    if !replacing {
        shell::offer_setup();
    }
    Ok(())
}

/// Download, verify, unpack and compile. Returns the installed tree, laid out like any other version.
fn compile_and_stage(
    release: &source_build::SourceRelease,
    final_directory: &Path,
    work_directory: &Path,
    plan: &source_build::Plan,
    jobs: usize,
    log_path: &Path,
) -> Result<std::path::PathBuf, String> {
    let archive = work_directory.join("php.tar.gz");
    download::fetch(&release.url, &archive, Some(&release.sha256))?;
    println!("Verified SHA-256 checksum");
    match source_build::verify_signature(release, &archive, work_directory)? {
        true => println!("Verified the PGP signature with the keyring from php.net"),
        false => println!("Note: gpg was not found, so the PGP signature was not checked (the SHA-256 was)."),
    }

    let source = source_build::unpack(&archive, &work_directory.join("src"))?;
    println!("Compiling with {} jobs. This takes a few minutes; the log is {}", jobs, log_path.display());
    let staging = work_directory.join("staging");
    let tree = source_build::compile(&source, final_directory, &staging, plan, jobs, log_path)?;
    lay_out_source_build(&tree, plan)?;
    Ok(tree)
}

/// Give a freshly compiled tree the layout every version has: the real binary in `libexec`, the
/// wrapper in `bin`, a php.ini and a conf.d, and the shared extensions that were built switched on
/// (except opcache, which is of no use to a command line script).
fn lay_out_source_build(tree: &Path, plan: &source_build::Plan) -> Result<(), String> {
    let libexec = tree.join("libexec");
    fs::create_dir_all(&libexec).map_err(|error| error.to_string())?;
    fs::rename(tree.join("bin").join("php"), libexec.join("php"))
        .map_err(|error| format!("moving the php binary: {}", error))?;
    write_wrapper_script(&tree.join("bin").join("php"))?;
    fs::create_dir_all(tree.join("conf.d")).map_err(|error| error.to_string())?;
    variant::write_source(tree)?;

    let mut ini = crate::ini::IniDocument::parse(DEFAULT_INI);
    let available = extensions::available_shared(tree);
    for name in plan.shared.iter().filter(|name| **name != "opcache") {
        extensions::enable(&mut ini, name, &available, || Ok(BTreeSet::new()))?;
    }
    ini.save(&tree.join("php.ini")).map_err(|error| format!("writing php.ini: {}", error))
}

/// A new build of a version keeps the old one's php.ini and conf.d, so switching builds does not
/// lose the settings the user made.
fn carry_over_configuration(old: &Path, new: &Path) -> Result<(), String> {
    let old_ini = old.join("php.ini");
    if old_ini.exists() {
        fs::copy(&old_ini, new.join("php.ini")).map_err(|error| format!("keeping php.ini: {}", error))?;
    }
    if let Ok(entries) = fs::read_dir(old.join("conf.d")) {
        fs::create_dir_all(new.join("conf.d")).map_err(|error| error.to_string())?;
        for entry in entries.flatten().filter(|entry| entry.path().is_file()) {
            fs::copy(entry.path(), new.join("conf.d").join(entry.file_name()))
                .map_err(|error| format!("keeping conf.d: {}", error))?;
        }
    }
    Ok(())
}

/// Put `new` where `current` is, setting the old one aside first and putting it back on failure.
fn swap_in(new: &Path, current: &Path, backup: &Path) -> Result<(), String> {
    if backup.exists() {
        let _ = fs::remove_dir_all(backup);
    }
    fs::rename(current, backup).map_err(|error| format!("setting the old build aside: {}", error))?;
    if let Err(error) = fs::rename(new, current) {
        let _ = fs::rename(backup, current);
        return Err(format!("installing the new build: {}", error));
    }
    let _ = fs::remove_dir_all(backup);
    Ok(())
}

/// Best effort: a problem with the saved settings must never fail an install.
fn apply_global_extension_settings(directory: &Path) {
    match extensions::apply_global_settings(directory) {
        Ok(Some(report)) if !report.describe().is_empty() => {
            println!("Applied your global extension settings:");
            for line in report.describe() {
                println!("  {}", line);
            }
            if report.has_unavailable() {
                if cfg!(windows) {
                    println!("  Run 'phpz ext sync --install' to download the missing ones.");
                } else {
                    println!("  A larger build may have them: see 'phpz variants'.");
                }
            }
        }
        Ok(_) => {}
        Err(error) => eprintln!("Note: could not apply your global extension settings: {}", error),
    }
}

/// Windows builds that should have a checksum but do not need the user's consent.
fn confirm_unverified(build: &Build, allow_unverified: bool) -> Result<(), String> {
    let unusual = build.expects_checksum && build.sha256.is_none();
    consent::require(unusual, &format!("PHP {}", build.version_name()), allow_unverified)
}

fn populate(build: &Build, directory: &Path) -> Result<(), String> {
    if cfg!(windows) {
        lay_out_windows_build(build, directory)
    } else {
        lay_out_static_build(build, directory)
    }
}

/// Extensions enabled by default in the Windows php.ini, when the build ships them.
/// "gd2" is how PHP 7.0 to 7.3 name GD; only the spelling the template has gets activated.
const WINDOWS_DEFAULT_EXTENSIONS: &[&str] = &[
    "bz2", "curl", "exif", "fileinfo", "gd", "gd2", "gmp", "intl", "mbstring", "mysqli", "openssl",
    "pdo_mysql", "pdo_pgsql", "pdo_sqlite", "pgsql", "sockets", "sodium", "sqlite3", "zip",
];

/// Official Windows zip, extracted as it comes:
///   php.exe, DLLs, ext\   from the zip
///   php.ini               php.ini-development with extension_dir and common extensions enabled
fn lay_out_windows_build(build: &Build, directory: &Path) -> Result<(), String> {
    let archive_path = directory.join("download.zip");
    download::fetch(&build.url, &archive_path, build.sha256.as_deref())?;
    download::announce_checksum(build.sha256.is_some());
    extract_zip(&archive_path, directory)?;
    let _ = fs::remove_file(&archive_path);

    let template_path = directory.join("php.ini-development");
    let template = fs::read_to_string(&template_path)
        .map_err(|error| format!("reading {}: {}", template_path.display(), error))?;

    // PHP 7.0 and 7.1 name extensions by DLL ("extension=php_curl.dll"); 7.2 and later use short names.
    let directives = windows_directives(!build.at_least(7, 2));
    fs::write(directory.join("php.ini"), uncomment_directives(&template, &directives))
        .map_err(|error| format!("writing php.ini: {}", error))
}

/// The php.ini lines to activate: extension_dir and the default extensions.
fn windows_directives(uses_dll_names: bool) -> Vec<String> {
    let mut directives = vec!["extension_dir = \"ext\"".to_string()];
    directives.extend(WINDOWS_DEFAULT_EXTENSIONS.iter().map(|name| {
        if uses_dll_names {
            format!("extension=php_{}.dll", name)
        } else {
            format!("extension={}", name)
        }
    }));
    directives
}

/// Turn `;directive` lines into active `directive` lines, ignoring a trailing `; comment`
/// when matching. PHP 7.0 to 7.2 write `; extension_dir` with a space, so that one directive
/// also matches that way; other lines must match exactly so prose comments stay comments.
/// Anything else is left alone.
fn uncomment_directives(content: &str, directives: &[String]) -> String {
    content
        .split_inclusive('\n')
        .map(|line| {
            let text = line.trim_end_matches(['\r', '\n']);
            let is_wanted = text
                .strip_prefix(';')
                .map(|rest| rest.split(';').next().unwrap_or("").trim_end())
                .map(|rest| {
                    directives.iter().any(|wanted| {
                        wanted == rest
                            || (wanted.starts_with("extension_dir") && wanted == rest.trim_start())
                    })
                })
                .unwrap_or(false);
            if is_wanted {
                line[1..].trim_start().to_string()
            } else {
                line.to_string()
            }
        })
        .collect()
}

/// Extract every entry of the zip below `destination`, refusing paths that escape it.
fn extract_zip(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| format!("reading archive: {}", error))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("reading archive: {}", error))?;
        let relative_path = entry
            .enclosed_name()
            .map(Path::to_path_buf)
            .ok_or_else(|| format!("unsafe path in archive: {}", entry.name()))?;
        let target = destination.join(relative_path);

        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = File::create(&target)
            .map_err(|error| format!("creating {}: {}", target.display(), error))?;
        io::copy(&mut entry, &mut output)
            .map_err(|error| format!("extracting {}: {}", target.display(), error))?;
    }
    Ok(())
}

/// Static build from static-php-cli (Linux and macOS):
///   libexec/php   the real static binary
///   bin/php       wrapper script that points PHP at this version's php.ini
///   php.ini       configuration
///   conf.d/       additional ini files
fn lay_out_static_build(build: &Build, directory: &Path) -> Result<(), String> {
    let archive_path = directory.join("download.tar.gz");
    download::fetch(&build.url, &archive_path, build.sha256.as_deref())?;
    download::announce_checksum(build.sha256.is_some());

    let libexec_directory = directory.join("libexec");
    fs::create_dir_all(&libexec_directory).map_err(|error| error.to_string())?;
    extract_php_binary(&archive_path, &libexec_directory.join("php"))?;
    let _ = fs::remove_file(&archive_path);

    let bin_directory = directory.join("bin");
    fs::create_dir_all(&bin_directory).map_err(|error| error.to_string())?;
    write_wrapper_script(&bin_directory.join("php"))?;

    if let Some(variant) = build.variant {
        variant::write(directory, variant)?;
    }
    fs::write(directory.join("php.ini"), DEFAULT_INI)
        .map_err(|error| format!("writing php.ini: {}", error))?;
    fs::create_dir_all(directory.join("conf.d")).map_err(|error| error.to_string())?;
    Ok(())
}

/// Extract the `php` executable from the archive to `destination`.
fn extract_php_binary(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|error| error.to_string())?;
    let mut archive = Archive::new(GzDecoder::new(file));

    let entries = archive
        .entries()
        .map_err(|error| format!("reading archive: {}", error))?;

    for entry in entries {
        let mut entry = entry.map_err(|error| format!("reading archive: {}", error))?;
        let is_php_binary = entry
            .path()
            .map(|path| path.file_name().map(|name| name == "php").unwrap_or(false))
            .unwrap_or(false);
        if is_php_binary {
            entry
                .unpack(destination)
                .map_err(|error| format!("extracting php binary: {}", error))?;
            set_executable(destination)?;
            return Ok(());
        }
    }
    Err("archive does not contain a 'php' binary".to_string())
}

#[cfg(unix)]
fn set_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

/// The static binary has its php.ini path compiled in (/usr/local/etc/php),
/// so a small wrapper script points PHP at this version's own configuration.
fn write_wrapper_script(path: &Path) -> Result<(), String> {
    let script = "\
#!/bin/sh
DIR=\"$(cd \"$(dirname \"$0\")/..\" && pwd -P)\"
export PHPRC=\"$DIR\"
export PHP_INI_SCAN_DIR=\"$DIR/conf.d\"
exec \"$DIR/libexec/php\" \"$@\"
";
    fs::write(path, script).map_err(|error| format!("writing wrapper: {}", error))?;
    set_executable(path)
}

#[cfg(test)]
mod tests;
