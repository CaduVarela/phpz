use crate::selection;

/// Show the PHP version that applies in the current directory
pub fn run() {
    match selection::select_here() {
        Ok(Some(selection)) => println!("{}", selection.version),
        Ok(None) => println!("No PHP version is active. Run 'phpz use <version>'."),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
