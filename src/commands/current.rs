use crate::installed;

pub fn run() {
    match installed::active_version() {
        Ok(Some(version)) => println!("{}", version),
        Ok(None) => println!("No PHP version is active. Run 'phpz use <version>'."),
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }
}
