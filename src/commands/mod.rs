pub mod install;
pub mod uninstall;
pub mod use_cmd;
pub mod list;
pub mod list_remote;
pub mod reset;
pub mod current;
pub mod env;
pub mod completions;
pub mod setup;
pub mod config;
pub mod ext;

use clap::Subcommand;
use clap_complete::Shell;

#[derive(Subcommand)]
pub enum Commands {
    /// Install a PHP version ("8", "8.3" or "8.3.9")
    Install { version: String },
    /// Remove an installed version (exact version, as shown by 'phpz list')
    Uninstall { version: String },
    /// Switch the active PHP version
    Use { version: String },
    /// List installed versions, or the ones available for download with --remote
    List {
        /// List versions available for download instead of installed ones
        #[arg(long)]
        remote: bool,

        /// With --remote, show every patch version instead of the newest of each minor
        #[arg(long, requires = "remote")]
        all: bool,
    },
    /// Deactivate the active PHP version
    Reset,
    /// Show the active PHP version
    Current,
    /// Print shell code to put the active version on PATH (use: eval "$(phpz env)")
    Env,
    /// Add phpz to your PATH
    Setup {
        /// Remove phpz from your shell profile instead
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
            Commands::Install { version } => install::run(version),
            Commands::Uninstall { version } => uninstall::run(version),
            Commands::Use { version } => use_cmd::run(version),
            Commands::List { remote: true, all } => list_remote::run(all),
            Commands::List { remote: false, .. } => list::run(),
            Commands::Reset => reset::run(),
            Commands::Current => current::run(),
            Commands::Env => env::run(),
            Commands::Setup { undo } => setup::run(undo),
            Commands::Completions { shell } => completions::run(shell),
            Commands::Config { command } => command.run(),
            Commands::Ext { command } => command.run(),
        }
    }
}
