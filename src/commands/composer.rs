use clap::Subcommand;

use crate::composer;

#[derive(Subcommand)]
pub enum ComposerCommands {
    /// Install Composer for the PHP versions you have, or update it (run again to get a newer one)
    Install,
}

impl ComposerCommands {
    pub fn run(self) -> Result<(), String> {
        match self {
            ComposerCommands::Install => {
                composer::install_for_installed_versions(true)?;
                crate::project::shim::ensure()
            }
        }
    }
}
