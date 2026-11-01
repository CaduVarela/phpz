use crate::php::selection;

/// Show the PHP version that applies in the current directory
pub fn run() -> Result<(), String> {
    match selection::select_here()? {
        Some(selection) => println!("{}", selection.version),
        None => println!("No PHP version is active. Run 'phpz use <version>'."),
    }
    Ok(())
}
