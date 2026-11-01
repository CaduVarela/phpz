use std::path::PathBuf;

use clap::Subcommand;

use crate::extensions::{self, Outcome};
use crate::ini::IniDocument;
use crate::settings::Settings;
use crate::{installed, paths};

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

    /// Apply the global extension settings to the active version (or to all with --all)
    Sync {
        /// Every installed version, not only the active one
        #[arg(long)]
        all: bool,
    },
}

struct ActiveVersion {
    name: String,
    directory: PathBuf,
}

impl ActiveVersion {
    fn find() -> Result<ActiveVersion, String> {
        let name = installed::active_version()?
            .ok_or("no PHP version is active. Run 'phpz use <version>'.")?;
        let directory = paths::versions_directory()?.join(&name);
        Ok(ActiveVersion { name, directory })
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
            ExtCommands::Sync { all } => sync(all),
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
                "This build has a fixed set of extensions."
            } else {
                "It is not bundled with this PHP, and installing extra extensions is not available yet."
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

fn sync(all: bool) -> Result<(), String> {
    if Settings::load()?.extensions.is_empty() {
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
        match extensions::apply_global_settings(&directory) {
            Ok(Some(report)) if report.describe().is_empty() => println!("PHP {}: already in sync", name),
            Ok(Some(report)) => {
                println!("PHP {}:", name);
                for line in report.describe() {
                    println!("  {}", line);
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
        return Err(format!("{} version(s) could not be synced", failures));
    }
    Ok(())
}
