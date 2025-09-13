mod commands;

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
    let cli = Cli::parse();
    cli.command.run();
}
