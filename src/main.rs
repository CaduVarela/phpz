use clap::Parser;
use phpz::Cli;

fn main() {
    phpz::platform::pipe::exit_quietly_on_closed_pipe();
    if phpz::project::shim::invoked_as_shim() {
        std::process::exit(phpz::project::shim::run());
    }
    if let Err(error) = Cli::parse().command.run() {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}
