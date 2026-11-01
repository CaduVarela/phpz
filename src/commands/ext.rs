use clap::Subcommand;

#[derive(Subcommand)]
pub enum ExtCommands {
    /// Enable a PHP extension (not available yet)
    Enable { name: String },

    /// Disable a PHP extension (not available yet)
    Disable { name: String },
}

impl ExtCommands {
    pub fn run(self) {
        eprintln!(
            "Error: extension management is not available yet. \
             The static PHP builds used by phpz ship a fixed set of extensions \
             and cannot load external ones."
        );
        std::process::exit(1);
    }
}
