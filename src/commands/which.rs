use crate::extensions;
use crate::php::selection;

/// Print the path of the php that runs in the current directory
pub fn run() {
    let result = selection::require_here()
        .and_then(|selection| Ok(extensions::php_executable(&selection.directory()?)));

    match result {
        Ok(path) => println!("{}", path.display()),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
