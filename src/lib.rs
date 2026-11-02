use clap::Parser;

pub mod commands;
pub mod composer;
pub mod download;
pub mod extensions;
pub mod php;
pub mod platform;
pub mod project;
pub mod release;

#[derive(Parser)]
#[command(
    name = "phpz",
    version,
    about = "Simple PHP version manager",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: commands::Commands,
}
