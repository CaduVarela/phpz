pub mod install;
pub mod use_cmd;
pub mod list;
pub mod reset;
pub mod config;
pub mod ext;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    Install { version: String },
    Use { version: String },
    List,
    Reset,
    Current,
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
            Commands::Current => {
                println!("(current) Showing current PHP version...");
            }
            Commands::Config { command } => command.run(),
            Commands::Ext { command } => command.run(),
        }
    }
}
