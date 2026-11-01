use crate::shell::{self, Shell};

/// Add phpz to your PATH by editing your shell profile
pub fn run() {
    if let Err(error) = setup() {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn setup() -> Result<(), String> {
    let shell = Shell::detect().ok_or(
        "unsupported shell (only bash and zsh are supported). \
         Add this line to your shell profile manually: eval \"$(phpz env)\"",
    )?;
    let profile = shell.profile_path()?;

    if shell::configure(&shell)? {
        println!("Updated {}. Open a new terminal to apply it.", profile.display());
    } else {
        println!("phpz is already set up in {}.", profile.display());
    }
    Ok(())
}
