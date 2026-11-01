use crate::paths;

/// Print shell code that puts the active version first on PATH.
/// Usage: eval "$(phpz env)"
pub fn run() {
    match paths::current_link() {
        Ok(link) => println!("export PATH=\"{}/bin:$PATH\"", link.display()),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
