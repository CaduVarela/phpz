use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::commands::ext;
use crate::download;
use crate::download::consent;
use crate::extensions;
use crate::extensions::settings::Settings;
use crate::php::installed;
use crate::php::source_build;
use crate::php::source_build::Signature;
use crate::php::variant::{self, Origin, Variant};
use crate::php::version;
use crate::php::version::Build;
use crate::platform::interaction;
use crate::platform::paths;
use crate::platform::shell;
use crate::project;
use crate::project::shim;

pub mod archive;
pub mod layout;

use layout::{lay_out_source_build, populate};

/// Install a PHP version, or the one this folder pins when none is given
pub fn run(
    version: Option<String>,
    allow_unverified: bool,
    variant: Option<Variant>,
    replace: bool,
    build: bool,
    jobs: Option<usize>,
) -> Result<(), String> {
    match version {
        Some(version) => {
            install_requested(&version, allow_unverified, variant, replace, build, jobs)
        }
        None => install_for_project(allow_unverified, variant, replace, build, jobs),
    }
}

fn install_requested(
    version: &str,
    allow_unverified: bool,
    variant: Option<Variant>,
    replace: bool,
    build: bool,
    jobs: Option<usize>,
) -> Result<(), String> {
    if build {
        install_from_source(version, replace, jobs, allow_unverified)
    } else if replace && variant.is_none() {
        Err(
            "--replace needs --variant (or --build) to say what to switch the version to"
                .to_string(),
        )
    } else {
        install(version, allow_unverified, variant, replace)
    }
}

/// Install the version this folder pins, then the extensions its `.phpz.toml` enables.
fn install_for_project(
    allow_unverified: bool,
    variant: Option<Variant>,
    replace: bool,
    build: bool,
    jobs: Option<usize>,
) -> Result<(), String> {
    let here = std::env::current_dir()
        .map_err(|error| format!("could not read the current directory: {}", error))?;
    let project = project::find(&here)?;
    let pinned = project
        .as_ref()
        .and_then(|project| project.php.as_ref())
        .ok_or("no version given and no .php-version or .phpz.toml with a version found here")?;

    if installed::find_installed(&pinned.request).is_ok() {
        println!(
            "PHP {} (pinned by {}) is already installed.",
            pinned.request,
            pinned.file.display()
        );
    } else {
        install_requested(
            &pinned.request,
            allow_unverified,
            variant,
            replace,
            build,
            jobs,
        )?;
    }

    let wants_extensions = project
        .as_ref()
        .is_some_and(|project| !project.extensions.enable.is_empty());
    if wants_extensions {
        ext::sync_project(true, allow_unverified)?;
    }
    Ok(())
}

/// Move an installed version to another build (used by `phpz ext enable` when an extension needs it).
pub fn switch_build(version: &str, variant: Variant) -> Result<(), String> {
    install(version, false, Some(variant), true)
}

/// Which build to install: the one asked for, the default, or a larger one the user accepts.
fn choose_variant(requested: Option<Variant>) -> Result<Variant, String> {
    if cfg!(windows) {
        return match requested {
            Some(_) => Err(
                "--variant is for Linux and macOS. Windows has one official build per version."
                    .to_string(),
            ),
            None => Ok(variant::DEFAULT),
        };
    }
    if let Some(variant) = requested {
        variant::runs_here(variant)?;
        return Ok(variant);
    }
    // Looking for a larger build must never get in the way of installing the usual one.
    Ok(offer_larger_build()
        .ok()
        .flatten()
        .unwrap_or(variant::DEFAULT))
}

/// Offer the smallest build that has every extension the global settings enable.
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

    eprintln!(
        "Your global extension settings enable {}, which the {} build lacks.",
        missing.join(", "),
        variant::DEFAULT.name()
    );
    eprintln!(
        "The {} build has them, and is a larger download.",
        target.name()
    );
    let lost = variant::lost_by_switching(&default_extensions, &target_extensions);
    if !lost.is_empty() {
        eprintln!(
            "It lacks {}, which the {} build has.",
            lost.join(", "),
            variant::DEFAULT.name()
        );
    }
    let question = format!("Use the {} build for this version?", target.name());
    Ok(interaction::confirm(&question, false)
        .filter(|answer| *answer)
        .map(|_| target))
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
            let which = if cfg!(windows) {
                String::new()
            } else {
                format!(" (the {} build)", existing.name())
            };
            println!("PHP {} is already installed{}.", resolved_version, which);
            if !cfg!(windows)
                && requested_variant.is_some()
                && existing != Origin::Prebuilt(variant)
            {
                println!(
                    "To switch it to the {} build, add --replace.",
                    variant.name()
                );
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
        println!(
            "Installing PHP {} (the {} build)",
            resolved_version,
            variant.name()
        );
    }
    let backup = versions_directory.join(format!("{}.old", resolved_version));
    let result = populate(&build, &temporary_directory)
        .and_then(|_| put_in_place(&temporary_directory, &final_directory, &backup, replacing));

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

/// Check for the compiler tools; says whether pkg-config is there to find the libraries.
fn require_build_tools() -> Result<bool, String> {
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
    Ok(has_pkg_config)
}

/// Say what will be compiled in, what becomes a loadable file and what is left out.
fn announce_plan(version: &str, plan: &source_build::Plan) {
    println!("Compiling PHP {} from the official source", version);
    println!("  compiled in: {}", plan.built_in.join(", "));
    println!(
        "  as files   : {} (switch them with 'phpz ext enable|disable')",
        plan.shared.join(", ")
    );
    for skipped in &plan.skipped {
        println!(
            "  left out   : {} (on Debian or Ubuntu: sudo apt install {})",
            skipped.name, skipped.hint
        );
    }
}

/// Failed builds keep only the log: the source tree is large and the log is what is worth reading.
fn discard_build_leftovers(work_directory: &Path) {
    for leftover in [
        "src",
        "staging",
        "gnupg",
        "php.tar.gz",
        "php-keyring.gpg",
        "signature.asc",
    ] {
        let _ = fs::remove_dir_all(work_directory.join(leftover));
    }
}

/// Finish an install: keep the old settings when replacing, apply the global ones, move into place.
fn put_in_place(
    staged: &Path,
    final_directory: &Path,
    backup: &Path,
    replacing: bool,
) -> Result<(), String> {
    if replacing {
        carry_over_configuration(final_directory, staged)?;
    }
    apply_global_extension_settings(staged);
    if replacing {
        swap_in(staged, final_directory, backup)
    } else {
        fs::rename(staged, final_directory)
            .map_err(|error| format!("finalizing installation: {}", error))
    }
}

/// Compile a version from the official source at php.net, verifying SHA-256 and signature.
fn install_from_source(
    requested_version: &str,
    replace: bool,
    jobs: Option<usize>,
    allow_unverified: bool,
) -> Result<(), String> {
    if !cfg!(target_os = "linux") {
        return Err("--build is available on Linux only for now.".to_string());
    }
    println!("Looking for PHP '{}' at php.net...", requested_version);
    let release = source_build::resolve(requested_version)?;
    if release.series() < source_build::OLDEST_BUILDABLE {
        let (major, minor) = source_build::OLDEST_BUILDABLE;
        return Err(format!(
            "the build recipe covers PHP {}.{} and newer, not {}.",
            major, minor, release.version
        ));
    }

    let versions_directory = paths::versions_directory()?;
    let final_directory = versions_directory.join(&release.version);
    let temporary_directory = versions_directory.join(format!("{}.tmp", release.version));
    fs::create_dir_all(&versions_directory)
        .map_err(|error| format!("creating versions directory: {}", error))?;

    let replacing = if final_directory.exists() {
        let existing = variant::read_origin(&final_directory);
        if replace && existing != Origin::Source {
            true
        } else {
            println!(
                "PHP {} is already installed (built from {}).",
                release.version,
                if existing == Origin::Source {
                    "source".to_string()
                } else {
                    format!("the {} build", existing.name())
                }
            );
            if existing != Origin::Source {
                println!("To replace it with one compiled here, add --replace.");
            }
            return Ok(());
        }
    } else {
        false
    };

    let has_pkg_config = require_build_tools()?;
    let plan = source_build::plan(release.series(), |package| {
        has_pkg_config && source_build::pkg_config_has(package)
    });
    if release.series() < source_build::FIRST_OPENSSL_3_SERIES {
        println!(
            "Note: PHP before 8.1 may fail to compile with the OpenSSL 3 of current distributions."
        );
    }

    announce_plan(&release.version, &plan);

    let work_directory = paths::phpz_home()?.join("build").join(&release.version);
    let _ = fs::remove_dir_all(&work_directory);
    fs::create_dir_all(&work_directory)
        .map_err(|error| format!("creating {}: {}", work_directory.display(), error))?;
    let log_path = work_directory.join("build.log");
    let jobs = jobs
        .or_else(|| {
            std::thread::available_parallelism()
                .ok()
                .map(|count| count.get())
        })
        .unwrap_or(1)
        .max(1);

    let result = compile_and_stage(
        &release,
        &final_directory,
        &work_directory,
        &plan,
        jobs,
        &log_path,
        allow_unverified,
    )
    .and_then(|tree| {
        let _ = fs::remove_dir_all(&temporary_directory);
        fs::rename(&tree, &temporary_directory)
            .map_err(|error| format!("moving the build into place: {}", error))?;
        let backup = versions_directory.join(format!("{}.old", release.version));
        put_in_place(&temporary_directory, &final_directory, &backup, replacing)
    });

    match result {
        Ok(()) => {
            let _ = fs::remove_dir_all(&work_directory);
        }
        Err(error) => {
            discard_build_leftovers(&work_directory);
            let _ = fs::remove_dir_all(&temporary_directory);
            return Err(error);
        }
    }

    println!(
        "Successfully installed PHP {} (compiled from source)",
        release.version
    );
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
    allow_unverified: bool,
) -> Result<std::path::PathBuf, String> {
    let archive = work_directory.join("php.tar.gz");
    download::fetch(&release.url, &archive, Some(&release.sha256))?;
    println!("Verified SHA-256 checksum");
    match source_build::verify_signature(release, &archive, work_directory)? {
        Signature::Pinned => {
            println!("Verified the PGP signature against the pinned release manager keys")
        }
        Signature::NotChecked => println!(
            "Note: gpg was not found, so the PGP signature was not checked (the SHA-256 was)."
        ),
        Signature::Unrecognized(fingerprint) => {
            consent::require_known_signer(&fingerprint, allow_unverified)?
        }
    }

    let source = source_build::unpack(&archive, &work_directory.join("src"))?;
    println!(
        "Compiling with {} jobs. This takes a few minutes; the log is {}",
        jobs,
        log_path.display()
    );
    let staging = work_directory.join("staging");
    let tree = source_build::compile(&source, final_directory, &staging, plan, jobs, log_path)?;
    lay_out_source_build(&tree, plan)?;
    Ok(tree)
}

/// Keep the old build's php.ini and conf.d so switching builds keeps settings.
pub fn carry_over_configuration(old: &Path, new: &Path) -> Result<(), String> {
    let old_ini = old.join("php.ini");
    if old_ini.exists() {
        fs::copy(&old_ini, new.join("php.ini"))
            .map_err(|error| format!("keeping php.ini: {}", error))?;
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
pub fn swap_in(new: &Path, current: &Path, backup: &Path) -> Result<(), String> {
    if backup.exists() {
        let _ = fs::remove_dir_all(backup);
    }
    fs::rename(current, backup)
        .map_err(|error| format!("setting the old build aside: {}", error))?;
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
        Err(error) => eprintln!(
            "Note: could not apply your global extension settings: {}",
            error
        ),
    }
}

/// Windows builds that should have a checksum but do not need the user's consent.
fn confirm_unverified(build: &Build, allow_unverified: bool) -> Result<(), String> {
    let unusual = build.expects_checksum && build.sha256.is_none();
    consent::require(
        unusual,
        &format!("PHP {}", build.version_name()),
        allow_unverified,
    )
}
