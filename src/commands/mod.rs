pub mod install;
pub mod uninstall;
pub mod use_cmd;
pub mod list;
pub mod list_remote;
pub mod reset;
pub mod current;
pub mod env;
pub mod completions;
pub mod config;
pub mod ext;

use clap::Subcommand;
use clap_complete::Shell;

#[derive(Subcommand)]
pub enum Commands {
    Install { version: String },
    /// Remove an installed version (exact version, as shown by 'phpz list')
    Uninstall { version: String },
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
    Reset,
    Current,
    /// Print shell code to put the active version on PATH (use: eval "$(phpz env)")
    Env,
    /// Print a shell completion script (bash, zsh, fish, elvish, powershell)
    Completions { shell: Shell },
    Config {
        #[command(subcommand)]
        command: config::ConfigCommands,
    },
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
            Commands::Completions { shell } => completions::run(shell),
            Commands::Config { command } => command.run(),
            Commands::Ext { command } => command.run(),
        }
    }
}
