use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::platform::paths;

const FILE_NAME: &str = "downloads.lock";

const HEADER: &str = "# SHA-256 of downloads that publish no checksum, recorded the first time.\n# phpz refuses a different file at the same address afterwards.\n";

/// Checksums remembered from the first download of files whose server publishes none.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DownloadLock {
    #[serde(default)]
    downloads: BTreeMap<String, String>,
}

impl DownloadLock {
    pub fn get(&self, url: &str) -> Option<&str> {
        self.downloads.get(url).map(String::as_str)
    }

    pub fn record(&mut self, url: &str, sha256: &str) {
        self.downloads
            .insert(url.to_string(), sha256.to_lowercase());
    }

    pub fn parse(text: &str) -> Result<DownloadLock, String> {
        toml::from_str(text).map_err(|error| error.to_string())
    }

    pub fn render(&self) -> String {
        format!("{}\n{}", HEADER, toml::to_string(self).unwrap_or_default())
    }

    fn path() -> Result<PathBuf, String> {
        Ok(paths::phpz_home()?.join(FILE_NAME))
    }

    pub fn load() -> Result<DownloadLock, String> {
        let path = Self::path()?;
        match fs::read_to_string(&path) {
            Ok(text) => {
                Self::parse(&text).map_err(|error| format!("invalid {}: {}", path.display(), error))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(DownloadLock::default()),
            Err(error) => Err(format!("reading {}: {}", path.display(), error)),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("creating {}: {}", parent.display(), error))?;
        }
        let temporary_path = path.with_extension("lock.tmp");
        fs::write(&temporary_path, self.render())
            .and_then(|_| fs::rename(&temporary_path, &path))
            .map_err(|error| format!("writing {}: {}", path.display(), error))
    }
}
