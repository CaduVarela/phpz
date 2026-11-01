use crate::shell::{self, Shell};

/// Add phpz to your PATH by editing your shell profile (or remove it with `undo`)
pub fn run(undo: bool) {
    if let Err(error) = setup(undo) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn setup(undo: bool) -> Result<(), String> {
    let shell = Shell::detect().ok_or(
        "unsupported shell (only bash and zsh are supported). \
         Add this line to your shell profile manually: eval \"$(phpz env)\"",
    )?;
    let profile = shell.profile_path()?;

    if undo {
        if shell::unconfigure(&shell)? {
            println!("Removed phpz from {}. Open a new terminal to apply it.", shell::tilde_path(&profile));
        } else {
            println!("phpz is not set up in {}.", shell::tilde_path(&profile));
        }
        return Ok(());
    }

    if shell::configure(&shell)? {
        println!("Updated {}. {}", shell::tilde_path(&profile), shell::apply_hint(&profile));
    } else {
        println!("phpz is already set up in {}.", shell::tilde_path(&profile));
    }
    Ok(())
}
