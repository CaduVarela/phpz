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
    List,
    /// List versions available for download (newest patch of each minor by default)
    #[command(name = "ls-remote")]
    ListRemote {
        /// Show every patch version
        #[arg(long)]
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
            Commands::List => list::run(),
            Commands::ListRemote { all } => list_remote::run(all),
            Commands::Reset => reset::run(),
            Commands::Current => current::run(),
            Commands::Env => env::run(),
            Commands::Completions { shell } => completions::run(shell),
            Commands::Config { command } => command.run(),
            Commands::Ext { command } => command.run(),
        }
    }
}
