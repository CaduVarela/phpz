use clap::CommandFactory;
use clap_complete::Shell;

/// Print a shell completion script to standard output
pub fn run(shell: Shell) {
    let mut command = crate::Cli::command();
    clap_complete::generate(shell, &mut command, "phpz", &mut std::io::stdout());
}
