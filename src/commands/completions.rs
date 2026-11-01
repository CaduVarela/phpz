use clap::CommandFactory;
use clap_complete::Shell;

pub fn run(shell: Shell) {
    let mut command = crate::Cli::command();
    clap_complete::generate(shell, &mut command, "phpz", &mut std::io::stdout());
}
