use clap::Subcommand;

use crate::ini::{IniDocument, PHP_SECTION};
use crate::selection;

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Set a php.ini key (the value is written exactly as given)
    Set { key: String, value: String },

    /// Get a php.ini key
    Get { key: String },

    /// Remove a php.ini key
    Remove { key: String },
}

impl ConfigCommands {
    pub fn run(self) {
        if let Err(error) = self.execute() {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    }

    fn execute(self) -> Result<(), String> {
        let ini_path = selection::require_here()?.directory()?.join("php.ini");
        let mut document = IniDocument::load(&ini_path)
            .map_err(|error| format!("reading {}: {}", ini_path.display(), error))?;

        match self {
            ConfigCommands::Set { key, value } => {
                document.set(PHP_SECTION, &key, &value);
                document
                    .save(&ini_path)
                    .map_err(|error| format!("writing {}: {}", ini_path.display(), error))?;
                println!("{} = {}", key, value);
            }
            ConfigCommands::Get { key } => match document.get(PHP_SECTION, &key) {
                Some(value) => println!("{}", value),
                None => return Err(format!("'{}' is not set", key)),
            },
            ConfigCommands::Remove { key } => {
                if document.remove(PHP_SECTION, &key) == 0 {
                    return Err(format!("'{}' is not set", key));
                }
                document
                    .save(&ini_path)
                    .map_err(|error| format!("writing {}: {}", ini_path.display(), error))?;
                println!("Removed {}", key);
            }
        }
        Ok(())
    }
}
