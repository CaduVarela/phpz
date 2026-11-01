use std::fs;
use std::path::Path;

use crate::lock::DownloadLock;
use crate::{consent, download, pecl};

pub struct Installed {
    /// The extension release that was installed.
    pub version: String,
    /// The file had no known checksum, so the one just computed was recorded.
    pub first_use: bool,
}

/// Download `name` from the PECL server into the `ext` directory of a PHP version.
///
/// The server publishes no checksums, so the first download of a file needs the user's consent
/// and its SHA-256 is remembered. Any later download of the same address must match it.
pub fn install_package(
    version_directory: &Path,
    php_version: &str,
    name: &str,
    requested: Option<&str>,
    allow_unverified: bool,
) -> Result<Installed, String> {
    if !pecl::is_valid_name(name) {
        return Err(format!("'{}' is not a valid extension name", name));
    }
    let series = pecl::series_of(php_version)
        .ok_or_else(|| format!("cannot tell the PHP series of '{}'", php_version))?;
    let package = pecl::find(name, series, requested)?;

    let mut lock = DownloadLock::load()?;
    let remembered = lock.get(&package.url).map(str::to_string);
    if remembered.is_none() {
        let subject = format!("{} {} for PHP {}.{}", name, package.version, series.0, series.1);
        consent::require(true, &subject, allow_unverified)?;
    }

    let ext_directory = version_directory.join("ext");
    fs::create_dir_all(&ext_directory).map_err(|error| format!("creating {}: {}", ext_directory.display(), error))?;
    let archive_path = ext_directory.join(format!(".{}.download", name));

    let downloaded = download::fetch(&package.url, &archive_path, remembered.as_deref()).map_err(|error| {
        if remembered.is_some() {
            format!("{} The file at this address is not the one you installed before, so nothing was installed.", error)
        } else {
            error
        }
    });
    let result = downloaded.and_then(|actual| {
        // Remember the checksum before installing, so a failure to save it stops the install.
        if remembered.is_none() {
            lock.record(&package.url, &actual);
            lock.save()?;
        }
        pecl::install_dll(&archive_path, name, &ext_directory)
    });
    let _ = fs::remove_file(&archive_path);
    result?;

    Ok(Installed {
        version: package.version,
        first_use: remembered.is_none(),
    })
}
