mod commands;
mod environment;
mod ini;
mod installed;
mod link;
mod paths;
mod pipe;
mod shell;
mod user_path;
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
    let cli = Cli::parse();
    cli.command.run();
}
