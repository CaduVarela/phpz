pub mod install;
pub mod use_cmd;
pub mod list;
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
    Use { version: String },
    List,
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
            Commands::Use { version } => use_cmd::run(version),
            Commands::List => list::run(),
            Commands::Reset => reset::run(),
            Commands::Current => current::run(),
            Commands::Env => env::run(),
            Commands::Completions { shell } => completions::run(shell),
            Commands::Config { command } => command.run(),
            Commands::Ext { command } => command.run(),
        }
    }
}
