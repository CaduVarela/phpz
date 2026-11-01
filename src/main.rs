use clap::Parser;
use phpz::Cli;

fn main() {
    phpz::platform::pipe::exit_quietly_on_closed_pipe();
    if phpz::project::shim::invoked_as_shim() {
        std::process::exit(phpz::project::shim::run());
    }
    Cli::parse().command.run();
}
