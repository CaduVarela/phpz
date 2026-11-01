use crate::shell::SetupTarget;

/// Add phpz to your PATH (or remove it with `undo`)
pub fn run(undo: bool) {
    if let Err(error) = setup(undo) {
        eprintln!("Error: {}", error);
        std::process::exit(1);
    }
}

fn setup(undo: bool) -> Result<(), String> {
    let target = SetupTarget::detect().ok_or(
        "unsupported shell (only bash and zsh are supported). \
         Add this line to your shell profile manually: eval \"$(phpz env)\"",
    )?;
    let description = target.description();

    if undo {
        if target.unconfigure()? {
            println!("Removed phpz from {}. Open a new terminal to apply it.", description);
        } else {
            println!("phpz is not set up in {}.", description);
        }
        return Ok(());
    }

    if target.configure()? {
        println!("Updated {}. {}", description, target.apply_hint());
    } else {
        println!("phpz is already set up in {}.", description);
    }
    Ok(())
}
