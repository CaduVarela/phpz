use clap::Subcommand;

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Set a php.ini key
    Set { key: String, value: String },

    /// Get a php.ini key
    Get { key: String },

    /// Remove a php.ini key
    Remove { key: String },
}

impl ConfigCommands {
    pub fn run(self) {
        match self {
            ConfigCommands::Set { key, value } => {
                println!("(config) Setting {} = {}", key, value);
                // TODO: call ini.rs logic to update php.ini
            }
            ConfigCommands::Get { key } => {
                println!("(config) Getting value of {}", key);
                // TODO: call ini.rs logic to read php.ini
            }
            ConfigCommands::Remove { key } => {
                println!("(config) Removing {}", key);
                // TODO: call ini.rs logic to remove entry from php.ini
            }
        }
    }
}
