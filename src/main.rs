mod commands;
mod download;
mod extensions;
mod php;
mod platform;
mod project;

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
    platform::pipe::exit_quietly_on_closed_pipe();
    if project::shim::invoked_as_shim() {
        std::process::exit(project::shim::run());
    }
    let cli = Cli::parse();
    cli.command.run();
}
