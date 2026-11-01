mod commands;
mod consent;
mod download;
mod environment;
mod extension_install;
mod extensions;
mod ini;
mod installed;
mod interaction;
mod link;
mod lock;
mod paths;
mod project;
mod project_ini;
mod pecl;
mod pipe;
mod selection;
mod shim;
mod settings;
mod shell;
mod user_path;
mod variant;
mod version;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "phpz",
    version,
    about = "Simple PHP version manager",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: commands::Commands,
}

fn main() {
    pipe::exit_quietly_on_closed_pipe();
    if shim::invoked_as_shim() {
        std::process::exit(shim::run());
    }
    let cli = Cli::parse();
    cli.command.run();
}
