pub mod completions;
pub mod config;
pub mod current;
pub mod env;
pub mod ext;
pub mod install;
pub mod list;
pub mod list_remote;
pub mod local;
pub mod prune;
pub mod reset;
pub mod setup;
pub mod uninstall;
pub mod use_cmd;
pub mod variants;
pub mod which;

use clap::Subcommand;
use clap_complete::Shell;

use crate::variant::Variant;

#[derive(Subcommand)]
pub enum Commands {
    /// Install a PHP version ("8", "8.3" or "8.3.9")
    Install {
        version: String,

        /// Install even when no checksum is published to verify the download (needed in scripts)
        #[arg(long)]
        allow_unverified: bool,

        /// Linux and macOS build: minimal, common (default), bulk or gnu-bulk. See 'phpz variants'
        #[arg(long, value_parser = Variant::parse_argument)]
        variant: Option<Variant>,

        /// Switch an installed version to the build given by --variant (or --build), keeping its php.ini
        #[arg(long)]
        replace: bool,

        /// Linux: compile the version from the official source at php.net instead of downloading a
        /// prebuilt one. Gives the newest patch releases and extensions that can be switched.
        #[arg(long, conflicts_with = "variant")]
        build: bool,

        /// With --build, how many compile jobs to run (default: one per core)
        #[arg(long, requires = "build")]
        jobs: Option<usize>,
    },
    /// Remove an installed version (exact version, as shown by 'phpz list')
    Uninstall { version: String },
    /// Switch the active PHP version, or the one this folder pins when none is given
    Use { version: Option<String> },
    /// List installed versions, or the ones available for download with --remote
    List {
        /// List versions available for download instead of installed ones
        #[arg(long)]
        remote: bool,

        /// With --remote, show every patch version instead of the newest of each minor
        #[arg(long, requires = "remote")]
        all: bool,

        /// With --remote, list the versions of this build (Linux and macOS)
        #[arg(long, requires = "remote", value_parser = Variant::parse_argument)]
        variant: Option<Variant>,
    },
    /// Show the builds of PHP you can install on Linux and macOS, and what each has
    Variants,
    /// Deactivate the active PHP version
    Reset,
    /// Remove generated files of projects that no longer exist
    Prune {
        /// Only list what would be removed
        #[arg(long)]
        dry_run: bool,
    },
    /// Show the PHP version that applies in the current directory
    Current,
    /// Pin the PHP version of the current directory (writes .php-version), or show it
    Local {
        /// "8", "8.3" or "8.3.9"
        version: Option<String>,

        /// Remove the .php-version of this directory
        #[arg(long)]
        unset: bool,
    },
    /// Print the path of the php that runs in the current directory
    Which,
    /// Print shell code to put the active version on PATH (eval "$(phpz env)", or in PowerShell: phpz env | Invoke-Expression)
    Env,
    /// Add phpz to your PATH
    Setup {
        /// Remove phpz from your PATH setup instead
        #[arg(long)]
        undo: bool,
    },
    /// Print a shell completion script (bash, zsh, fish, elvish, powershell)
    Completions { shell: Shell },
    /// Read and edit the active version's php.ini
    Config {
        #[command(subcommand)]
        command: config::ConfigCommands,
    },
    /// Manage PHP extensions (not available yet)
    Ext {
        #[command(subcommand)]
        command: ext::ExtCommands,
    },
}

impl Commands {
    pub fn run(self) {
        match self {
            Commands::Install {
                version,
                allow_unverified,
                variant,
                replace,
                build,
                jobs,
            } => install::run(version, allow_unverified, variant, replace, build, jobs),
            Commands::Uninstall { version } => uninstall::run(version),
            Commands::Use { version } => use_cmd::run(version),
            Commands::List {
                remote: true,
                all,
                variant,
            } => list_remote::run(all, variant),
            Commands::List { remote: false, .. } => list::run(),
            Commands::Variants => variants::run(),
            Commands::Reset => reset::run(),
            Commands::Prune { dry_run } => prune::run(dry_run),
            Commands::Current => current::run(),
            Commands::Local { version, unset } => local::run(version, unset),
            Commands::Which => which::run(),
            Commands::Env => env::run(),
            Commands::Setup { undo } => setup::run(undo),
            Commands::Completions { shell } => completions::run(shell),
            Commands::Config { command } => command.run(),
            Commands::Ext { command } => command.run(),
        }
    }
}
