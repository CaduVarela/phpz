use std::path::PathBuf;

use clap::Subcommand;

use crate::extensions::{self, Outcome};
use crate::{extension_install, pecl};
use crate::ini::IniDocument;
use crate::project::{Project, PROJECT_FILE};
use crate::settings::Settings;
use crate::{installed, paths, selection};

#[derive(Subcommand)]
pub enum ExtCommands {
    /// List the extensions of the active PHP version
    List,

    /// Enable an extension in the active PHP version
    Enable {
        name: String,

        /// Also keep it enabled in every version, now and when installed later
        #[arg(long)]
        global: bool,
    },

    /// Disable an extension in the active PHP version
    Disable {
        name: String,

        /// Also keep it disabled in every version, now and when installed later
        #[arg(long)]
        global: bool,
    },

    /// Download and enable an extension that does not ship with PHP (Windows)
    Install {
        name: String,

        /// A specific release instead of the newest stable one
        #[arg(long)]
        version: Option<String>,

        /// Install even though the server publishes no checksum (needed in scripts)
        #[arg(long)]
        allow_unverified: bool,

        /// Also keep it enabled in every version, now and when installed later
        #[arg(long)]
        global: bool,
    },

    /// Apply the global extension settings to the active version (or to all with --all)
    Sync {
        /// Every installed version, not only the active one
        #[arg(long, conflicts_with = "project")]
        all: bool,

        /// Check (and with --install, download) the extensions the project's .phpz.toml enables
        #[arg(long)]
        project: bool,

        /// Also download the extensions a version is missing (Windows)
        #[arg(long)]
        install: bool,

        /// With --install, go ahead even though the server publishes no checksum
        #[arg(long, requires = "install")]
        allow_unverified: bool,
    },
}

struct ActiveVersion {
    name: String,
    directory: PathBuf,
    project: Option<Project>,
}

impl ActiveVersion {
    fn find() -> Result<ActiveVersion, String> {
        let selection = selection::require_here()?;
        let directory = selection.directory()?;
        Ok(ActiveVersion { name: selection.version, directory, project: selection.project })
    }

    fn ini_path(&self) -> PathBuf {
        self.directory.join("php.ini")
    }
}

impl ExtCommands {
    pub fn run(self) {
        if let Err(error) = self.execute() {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }

    fn execute(self) -> Result<(), String> {
        match self {
            ExtCommands::List => list(),
            ExtCommands::Enable { name, global } => toggle(&name, true, global),
            ExtCommands::Disable { name, global } => toggle(&name, false, global),
            ExtCommands::Install { name, version, allow_unverified, global } => {
                install_extension(&name, version.as_deref(), allow_unverified, global)
            }
            ExtCommands::Sync { all, project, install, allow_unverified } => {
                if project {
                    sync_project(install, allow_unverified)
                } else {
                    sync(all, install, allow_unverified)
                }
            }
        }
    }
}

fn load_ini(version: &ActiveVersion) -> Result<IniDocument, String> {
    IniDocument::load(&version.ini_path())
        .map_err(|error| format!("reading {}: {}", version.ini_path().display(), error))
}

fn list() -> Result<(), String> {
    let version = ActiveVersion::find()?;
    let ini = load_ini(&version)?;
    let available = extensions::available_shared(&version.directory);
    let loaded = extensions::loaded_modules(&version.directory)?;
    let listing = extensions::classify(&available, &loaded, &extensions::enabled_in_ini(&ini));

    println!("PHP {}", version.name);
    if let Some(project) = version.project.as_ref().filter(|project| !project.extensions.is_empty()) {
        println!("project {}: {}", project.directory.display(), describe_choices(&project.extensions));
    }
    print_group("enabled", &listing.enabled);
    print_group("disabled (can be enabled)", &listing.disabled);
    print_group("enabled in php.ini but not loading", &listing.failing);
    print_group("built in (always on)", &listing.built_in);
    Ok(())
}

fn print_group(title: &str, names: &[String]) {
    if names.is_empty() {
        return;
    }
    println!("\n{} ({}):", title, names.len());
    for line in extensions::wrap_names(names, 76) {
        println!("  {}", line);
    }
}

fn toggle(name: &str, enable: bool, global: bool) -> Result<(), String> {
    let version = ActiveVersion::find()?;
    let mut ini = load_ini(&version)?;
    let available = extensions::available_shared(&version.directory);
    let loaded = || extensions::loaded_modules(&version.directory);

    let outcome = if enable {
        extensions::enable(&mut ini, name, &available, loaded)?
    } else {
        extensions::disable(&mut ini, name, &available, loaded)?
    };
    let name = extensions::extension_name(name);

    match outcome {
        Outcome::Changed => {
            ini.save(&version.ini_path())
                .map_err(|error| format!("writing {}: {}", version.ini_path().display(), error))?;
            let verb = if enable { "Enabled" } else { "Disabled" };
            println!("{} {} in PHP {}", verb, name, version.name);
        }
        Outcome::Unchanged => {
            let state = if enable { "enabled" } else { "disabled" };
            println!("{} is already {} in PHP {}.", name, state, version.name);
        }
        Outcome::BuiltIn if enable => {
            println!("{} is built into PHP {} and is always on.", name, version.name)
        }
        Outcome::BuiltIn => {
            return Err(format!("{} is built into PHP {} and cannot be disabled.", name, version.name))
        }
        Outcome::Unavailable => {
            let reason = if available.is_empty() {
                "This build has a fixed set of extensions.".to_string()
            } else {
                format!("It is not bundled with this PHP. Run 'phpz ext install {}' to download it.", name)
            };
            return Err(format!("{} is not available in PHP {}. {}", name, version.name, reason));
        }
    }

    // Only reached when the extension is known here, so a typo never ends up in the settings.
    if global {
        let mut settings = Settings::load()?;
        if enable {
            settings.want_enabled(&name);
        } else {
            settings.want_disabled(&name);
        }
        settings.save()?;
        let state = if enable { "enabled" } else { "disabled" };
        println!("Saved {} as {} in your global settings. Every version you install or sync gets it.", name, state);
    }
    Ok(())
}

const WINDOWS_ONLY: &str = "installing extensions is only available on Windows for now. \
Linux and macOS builds are static and cannot load extra extensions.";

fn install_extension(name: &str, requested: Option<&str>, allow_unverified: bool, global: bool) -> Result<(), String> {
    if !cfg!(windows) {
        return Err(WINDOWS_ONLY.to_string());
    }
    let version = ActiveVersion::find()?;
    let name = extensions::extension_name(name);
    if !pecl::is_valid_name(&name) {
        return Err(format!("'{}' is not a valid extension name", name));
    }

    // Already part of PHP (shared or built in): nothing to download, just switch it on.
    let available = extensions::available_shared(&version.directory);
    if available.contains(&name) || extensions::loaded_modules(&version.directory)?.contains(&name) {
        println!("{} already ships with PHP {}.", name, version.name);
        return toggle(&name, true, global);
    }

    let installed = extension_install::install_package(&version.directory, &version.name, &name, requested, allow_unverified)?;
    println!("Installed {} {} for PHP {}", name, installed.version, version.name);
    if installed.first_use {
        println!("Its checksum was recorded in downloads.lock; phpz will refuse a different file at the same address from now on.");
    }
    toggle(&name, true, global)
}

/// "enables intl, redis; disables xdebug"
fn describe_choices(choices: &crate::settings::ExtensionSettings) -> String {
    let mut parts = Vec::new();
    if !choices.enable.is_empty() {
        parts.push(format!("enables {}", choices.enable.join(", ")));
    }
    if !choices.disable.is_empty() {
        parts.push(format!("disables {}", choices.disable.join(", ")));
    }
    parts.join("; ")
}

/// Make sure the version that applies here has the extensions the project enables. The project's
/// choices themselves are applied at run time by the php shim, through a generated php.ini.
fn sync_project(install: bool, allow_unverified: bool) -> Result<(), String> {
    if install && !cfg!(windows) {
        return Err(WINDOWS_ONLY.to_string());
    }
    let version = ActiveVersion::find()?;
    let Some(project) = version.project.as_ref().filter(|project| !project.extensions.is_empty()) else {
        println!("No extension choices apply here. Add an [extensions] table to a {} file.", PROJECT_FILE);
        return Ok(());
    };

    let wanted = &project.extensions.enable;
    let available = extensions::available_shared(&version.directory);
    let missing = extensions::missing_extensions(wanted, &available, || extensions::loaded_modules(&version.directory))?;
    if missing.is_empty() {
        println!("PHP {} has everything this project enables.", version.name);
        return Ok(());
    }
    if !install {
        println!("PHP {} lacks: {}", version.name, missing.join(", "));
        if cfg!(windows) {
            println!("Run 'phpz ext sync --project --install' to download them.");
        }
        return Ok(());
    }

    let (downloaded, failed) = download_missing(&version.directory, &version.name, &missing, allow_unverified)?;
    if !downloaded.is_empty() {
        println!("downloaded: {}", downloaded.join(", "));
    }
    for line in &failed {
        eprintln!("{}", line);
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(format!("{} extension(s) could not be downloaded", failed.len()))
    }
}

fn sync(all: bool, install: bool, allow_unverified: bool) -> Result<(), String> {
    if install && !cfg!(windows) {
        return Err(WINDOWS_ONLY.to_string());
    }
    let settings = Settings::load()?;
    if settings.extensions.is_empty() {
        println!("There are no global extension settings yet. Use 'phpz ext enable <name> --global'.");
        return Ok(());
    }

    let versions: Vec<String> = if all {
        installed::installed_versions()?
    } else {
        vec![ActiveVersion::find()?.name]
    };
    if versions.is_empty() {
        return Err("no PHP versions are installed".to_string());
    }

    let mut failures = 0;
    for name in versions {
        let directory = paths::versions_directory()?.join(&name);
        let mut lines = Vec::new();

        if install {
            match download_missing(&directory, &name, &settings.extensions.enable, allow_unverified) {
                Ok((downloaded, failed)) => {
                    if !downloaded.is_empty() {
                        lines.push(format!("downloaded: {}", downloaded.join(", ")));
                    }
                    failures += failed.len();
                    lines.extend(failed);
                }
                Err(error) => {
                    eprintln!("PHP {}: {}", name, error);
                    failures += 1;
                    continue;
                }
            }
        }

        match extensions::apply_global_settings(&directory) {
            Ok(Some(report)) => {
                lines.extend(report.describe());
                if lines.is_empty() {
                    println!("PHP {}: already in sync", name);
                    continue;
                }
                println!("PHP {}:", name);
                for line in &lines {
                    println!("  {}", line);
                }
                if report.has_unavailable() && !install && cfg!(windows) {
                    println!("  Run 'phpz ext sync --install' to download the missing ones.");
                }
            }
            Ok(None) => {}
            Err(error) => {
                eprintln!("PHP {}: {}", name, error);
                failures += 1;
            }
        }
    }

    if failures > 0 {
        return Err(format!("{} problem(s) found while syncing", failures));
    }
    Ok(())
}

/// Download the wanted extensions this PHP lacks. Returns what was downloaded, and one line for
/// each extension that could not be, because one bad name must not stop the others.
fn download_missing(
    directory: &std::path::Path,
    php_version: &str,
    wanted: &[String],
    allow_unverified: bool,
) -> Result<(Vec<String>, Vec<String>), String> {
    let available = extensions::available_shared(directory);
    let missing = extensions::missing_extensions(wanted, &available, || extensions::loaded_modules(directory))?;

    let mut downloaded = Vec::new();
    let mut failed = Vec::new();
    for name in missing {
        match extension_install::install_package(directory, php_version, &name, None, allow_unverified) {
            Ok(installed) => downloaded.push(format!("{} {}", name, installed.version)),
            Err(error) => failed.push(format!("could not download {}: {}", name, error)),
        }
    }
    Ok((downloaded, failed))
}
