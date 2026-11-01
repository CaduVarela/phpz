use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::extensions::extension_name;
use crate::paths;

const FILE_NAME: &str = "config.toml";

const HEADER: &str = "# phpz settings. 'phpz ext enable <name> --global' and 'phpz ext disable <name> --global' edit this file.\n";

/// phpz's own settings, kept in `config.toml` inside the phpz home.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub extensions: ExtensionSettings,
}

/// Extensions every PHP version should have on or off, now and when installed later.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExtensionSettings {
    #[serde(default)]
    pub enable: Vec<String>,
    #[serde(default)]
    pub disable: Vec<String>,
}

impl ExtensionSettings {
    pub fn is_empty(&self) -> bool {
        self.enable.is_empty() && self.disable.is_empty()
    }
}

fn insert_sorted(list: &mut Vec<String>, name: String) {
    if let Err(position) = list.binary_search(&name) {
        list.insert(position, name);
    }
}

impl Settings {
    /// Always have `name` on. A name lives in one list only, so this drops it from `disable`.
    pub fn want_enabled(&mut self, name: &str) {
        let name = extension_name(name);
        self.extensions.disable.retain(|existing| existing != &name);
        insert_sorted(&mut self.extensions.enable, name);
    }

    /// Always have `name` off. A name lives in one list only, so this drops it from `enable`.
    pub fn want_disabled(&mut self, name: &str) {
        let name = extension_name(name);
        self.extensions.enable.retain(|existing| existing != &name);
        insert_sorted(&mut self.extensions.disable, name);
    }

    pub fn parse(text: &str) -> Result<Settings, String> {
        toml::from_str(text).map_err(|error| error.to_string())
    }

    pub fn render(&self) -> String {
        let body = toml::to_string(self).unwrap_or_default();
        format!("{}\n{}", HEADER, body)
    }

    fn path() -> Result<PathBuf, String> {
        Ok(paths::phpz_home()?.join(FILE_NAME))
    }

    /// The saved settings, or empty ones when nothing was saved yet.
    pub fn load() -> Result<Settings, String> {
        let path = Self::path()?;
        match fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text).map_err(|error| format!("invalid {}: {}", path.display(), error)),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(format!("reading {}: {}", path.display(), error)),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("creating {}: {}", parent.display(), error))?;
        }
        let temporary_path = path.with_extension("toml.tmp");
        fs::write(&temporary_path, self.render())
            .and_then(|_| fs::rename(&temporary_path, &path))
            .map_err(|error| format!("writing {}: {}", path.display(), error))
    }
}

#[cfg(test)]
mod tests;
