use crate::extensions;
use crate::php::selection;

/// Print the path of the php that runs in the current directory
pub fn run() -> Result<(), String> {
    let directory = selection::require_here()?.directory()?;
    println!("{}", extensions::php_executable(&directory).display());
    Ok(())
}
