use clap::Subcommand;

#[derive(Subcommand)]
pub enum ExtCommands {
    /// Enable a PHP extension
    Enable { name: String },

    /// Disable a PHP extension
    Disable { name: String },
}

impl ExtCommands {
    pub fn run(self) {
        match self {
            ExtCommands::Enable { name } => {
                println!("(ext) Enabling extension {}", name);
                // TODO: parse php.ini and uncomment 'extension={name}'
                // TODO: if not found, append 'extension={name}' to the Extensions section
            }
            ExtCommands::Disable { name } => {
                println!("(ext) Disabling extension {}", name);
                // TODO: parse php.ini and comment 'extension={name}'
            }
        }
    }
}
